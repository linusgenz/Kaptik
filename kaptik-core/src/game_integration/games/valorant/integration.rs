// game_integration/games/valorant/integration.rs
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::domain::events::{EventData, EventMetadata, EventType, RecordingEvent};
use crate::domain::game::GameState;
use crate::domain::game_stats::{GameOutcome, KDA};
use crate::game_integration::GameIntegrationTrait;
use crate::log;

use super::client::{ClientSession, ValorantClient};
use super::maps::{agent_id_to_name, map_id_to_name, queue_id_to_mode};
use super::models::MatchDetailsResponse;

#[derive(Default)]
struct InnerState {
    /// Cached session; cleared when the lockfile disappears.
    session: Option<ClientSession>,

    // Presence-derived game state
    session_loop_state: String,
    last_round_total: u32,
    last_ally_score: u32,
    last_enemy_score: u32,

    // Contextual data enriched by CoreGame
    current_match_id: Option<String>,
    current_agent: Option<String>,
    current_map: Option<String>,
    current_team: Option<String>,
    current_queue: Option<String>,

    // Event bookkeeping
    next_event_id: u32,
    pending_events: Vec<RecordingEvent>,

    // Post-game bookkeeping
    /// Match IDs we have already processed after game end (avoid double-emit).
    processed_match_ids: HashSet<String>,
    /// Best known post-game state (survives API shutdown).
    final_state: Option<GameState>,
}

impl InnerState {
    fn next_id(&mut self) -> u32 {
        let id = self.next_event_id;
        self.next_event_id += 1;
        id
    }
}

pub struct ValorantIntegration {
    client: ValorantClient,
    state: Arc<RwLock<InnerState>>,
}

impl ValorantIntegration {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: ValorantClient::new()?,
            state: Arc::new(RwLock::new(InnerState::default())),
        })
    }

    /// Returns a cloned session if one is cached, otherwise tries to create
    /// one from the lockfile. Returns `None` if Valorant is not running.
    async fn get_or_create_session(&self) -> Option<ClientSession> {
        {
            let s = self.state.read().await;
            if let Some(session) = &s.session {
                return Some(session.clone());
            }
        }

        match self.client.create_session().await {
            Ok(session) => {
                self.state.write().await.session = Some(session.clone());
                Some(session)
            }
            Err(e) => {
                log!("⚠️ Valorant: cannot create session ({})", e);
                None
            }
        }
    }

    /// Invalidates the cached session so the next poll re-authenticates.
    async fn invalidate_session(&self) {
        self.state.write().await.session = None;
    }

    // ─── Core poll loop ───

    /// Called every second.  Reads presence, detects transitions, accumulates
    /// events into `state.pending_events`.
    async fn poll(&self) {
        let session = match self.get_or_create_session().await {
            Some(s) => s,
            None => return,
        };

        // Fetch own presence from local API.
        let presence = match self.client.get_own_presence(&session).await {
            Ok(p) => p,
            Err(e) => {
                log!("⚠️ Valorant presence fetch failed: {}", e);

                //  presence is unavailable during loading screens and game startup.
                let prev_state = self.state.read().await.session_loop_state.clone();
                if prev_state != "INGAME" {
                    // Try GLZ to detect if we just entered a game
                    if let Ok(player) = self.client.get_current_game_player(&session).await {
                        log!("✅ Valorant: GLZ fallback confirmed INGAME (match {})", player.match_id);
                        // Synthesise an INGAME transition without presence data
                        self.on_entered_game_by_glz(&session, &player.match_id).await;
                        let mut s = self.state.write().await;
                        s.session_loop_state = "INGAME".to_string();
                    }
                }
                // Either way, nothing more to do this tick
                return;
            }
        };

        let new_loop_state = presence
            .session_loop_state
            .as_deref()
            .unwrap_or("MENUS")
            .to_string();
        let new_round_total = presence.total_rounds();
        let ally_score = presence.ally_score();
        let enemy_score = presence.enemy_score();

        // Snapshot previous state for transition detection.
        let (prev_loop_state, prev_round_total) = {
            let s = self.state.read().await;
            (s.session_loop_state.clone(), s.last_round_total)
        };

        // ── Transition: entered game ───
        if new_loop_state == "INGAME" && prev_loop_state != "INGAME" {
            self.on_entered_game(&session, &presence).await;
        }

        // ── Transition: new round started ───
        if new_loop_state == "INGAME" && new_round_total > prev_round_total {
            let current_round = new_round_total + 1; // next round about to play
            self.on_round_started(current_round, ally_score, enemy_score).await;
        }

        // ── Transition: game ended ───
        let was_in_game =
            prev_loop_state == "INGAME" || prev_loop_state == "PREGAME";
        if was_in_game && new_loop_state == "MENUS" {
            self.on_game_ended(&session).await;
        }

        // Persist updated presence data.
        {
            let mut s = self.state.write().await;
            s.session_loop_state = new_loop_state;
            s.last_round_total = new_round_total;
            s.last_ally_score = ally_score;
            s.last_enemy_score = enemy_score;

            if let Some(map) = &presence.match_map {
                s.current_map = Some(map_id_to_name(map).to_string());
            }
            if let Some(q) = &presence.queue_id {
                s.current_queue = Some(queue_id_to_mode(q).to_string());
            }
            if let Some(team) = &presence.party_owner_match_current_team {
                s.current_team = Some(team.clone());
            }
        }
    }

    // ─── Transition handlers ───

    async fn on_entered_game(&self, session: &ClientSession, presence: &super::models::PresencePrivate) {
        log!("🎮 Valorant: entered game – fetching match context");

        // Enrich with CoreGame data (agent, map, match ID).
        if let Ok(player) = self.client.get_current_game_player(session).await {
            if let Ok(match_data) = self
                .client
                .get_current_game_match(session, &player.match_id)
                .await
            {
                let mut s = self.state.write().await;
                s.current_match_id = Some(match_data.match_id.clone());
                s.current_team = Some(player.team_id.clone());

                if let Some(map) = presence.match_map.as_deref() {
                    s.current_map = Some(map_id_to_name(map).to_string());
                } else {
                    s.current_map = Some(map_id_to_name(&match_data.map_id).to_string());
                }

                // Find the local player in the player list to get their agent.
                let agent = match_data
                    .players
                    .iter()
                    .find(|p| p.subject == session.puuid)
                    .and_then(|p| p.character_id.as_deref())
                    .and_then(agent_id_to_name)
                    .map(str::to_string);

                s.current_agent = agent.clone();

                log!(
                    "✅ Valorant context: map={:?} agent={:?} team={}",
                    s.current_map,
                    agent,
                    player.team_id
                );
            }
        }

        // Emit round 1 start immediately (round 0 completed = 0+0 = 0, which
        // is where we are at game start before any round finishes).
        self.on_round_started(1, 0, 0).await;
    }

    /// Fallback variant of [`Self::on_entered_game`] used when presence is
    /// unavailable (e.g. during loading screens).  Uses only the GLZ match ID
    /// we already have; skips the presence-dependent map field.
    async fn on_entered_game_by_glz(&self, session: &ClientSession, match_id: &str) {
        log!("🎮 Valorant: entered game via GLZ fallback – fetching match context");

        if let Ok(match_data) = self.client.get_current_game_match(session, match_id).await {
            let mut s = self.state.write().await;
            s.current_match_id = Some(match_data.match_id.clone());

            let map = map_id_to_name(&match_data.map_id).to_string();
            s.current_map = Some(map.clone());

            let agent = match_data
                .players
                .iter()
                .find(|p| p.subject == session.puuid)
                .and_then(|p| p.character_id.as_deref())
                .and_then(agent_id_to_name)
                .map(str::to_string);

            s.current_agent = agent.clone();

            if let Some(player) = match_data.players.iter().find(|p| p.subject == session.puuid) {
                s.current_team = Some(player.team_id.clone());
            }

            log!(
                "✅ Valorant GLZ context: map={} agent={:?}",
                map, agent
            );
        }

        self.on_round_started(1, 0, 0).await;
    }

    async fn on_round_started(&self, round_number: u32, ally: u32, enemy: u32) {
        log!("📍 Valorant: Round {} started ({}–{})", round_number, ally, enemy);

        let map = self.state.read().await.current_map.clone();

        let mut s = self.state.write().await;
        let id = s.next_id();

        let event = RecordingEvent {
            event_id: id,
            event_type: EventType::RoundStart,
            timestamp: 0.0, // Will be overwritten by capture module
            data: EventData {
                name: format!("Round {}", round_number),
                actor: None,
                target: None,
                participants: vec![],
                metadata: EventMetadata {
                    map,
                    extra: {
                        let mut m = std::collections::HashMap::new();
                        m.insert("round".to_string(), round_number.to_string());
                        m.insert("ally_score".to_string(), ally.to_string());
                        m.insert("enemy_score".to_string(), enemy.to_string());
                        m
                    },
                    ..Default::default()
                },
            },
        };
        s.pending_events.push(event);
    }

    /// Called when `sessionLoopState` transitions from INGAME → MENUS.
    ///
    /// Fetches the most recent match history entry and, if it's a new match,
    /// retrieves full details to emit KDA events and `GameEnd`.
    async fn on_game_ended(&self, session: &ClientSession) {
        log!("🏁 Valorant: game ended – fetching post-game data");

        let match_id = match self.client.get_latest_match_id(session).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                log!("⚠️ Valorant: match history is empty after game end");
                return;
            }
            Err(e) => {
                log!("⚠️ Valorant: failed to fetch match history: {}", e);
                return;
            }
        };

        // Avoid processing the same match twice (e.g. disconnect/reconnect).
        {
            let s = self.state.read().await;
            if s.processed_match_ids.contains(&match_id) {
                log!("⚠️ Valorant: match {} already processed, skipping", match_id);
                return;
            }
        }

        // Give the PD server a moment to finalise the match record.
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        match self.client.get_match_details(session, &match_id).await {
            Ok(details) => {
                self.process_match_details(&match_id, &details, &session.puuid).await;
            }
            Err(e) => {
                log!("⚠️ Valorant: failed to fetch match details for {}: {}", match_id, e);
            }
        }
    }

    /// Turns a completed [`MatchDetailsResponse`] into domain events and
    /// persists a final `GameState`.
    async fn process_match_details(
        &self,
        match_id: &str,
        details: &MatchDetailsResponse,
        puuid: &str,
    ) {
        let player = details.players.iter().find(|p| p.subject == puuid);
        let stats = player.and_then(|p| p.stats.as_ref());
        let player_team_id = player.map(|p| p.team_id.as_str()).unwrap_or("");

        // Determine KDA.
        let kda = stats.map(|s| KDA {
            kills: s.kills,
            deaths: s.deaths,
            assists: s.assists,
        });

        // Determine outcome.
        let outcome = details
            .teams
            .iter()
            .find(|t| t.team_id == player_team_id)
            .map(|t| if t.won { GameOutcome::Victory } else { GameOutcome::Defeat });

        // Agent name.
        let agent_name = player
            .map(|p| p.character_id.as_str())
            .and_then(agent_id_to_name)
            .map(str::to_string);

        // Map & mode names.
        let map_name = map_id_to_name(&details.match_info.map_id).to_string();

        log!(
            "📊 Valorant post-game: map={} kda={:?} outcome={:?}",
            map_name, kda, outcome
        );

        let mut s = self.state.write().await;

        // Mark as processed.
        s.processed_match_ids.insert(match_id.to_string());

        let kda_clone = kda;
        let next_id = |state: &mut InnerState| -> u32 {
            let id = state.next_event_id;
            state.next_event_id += 1;
            id
        };

        // Emit per-type KDA events so the recorder captures them in the event
        // list.  We emit them at timestamp 0 (the capture module will set the
        // correct relative timestamp).
        if let Some(s_ref) = stats {
            for _ in 0..s_ref.kills {
                let id = next_id(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id,
                    event_type: EventType::Kill,
                    timestamp: 0.0,
                    data: EventData {
                        name: "Kill".to_string(),
                        actor: agent_name.clone(),
                        target: None,
                        participants: vec![],
                        metadata: EventMetadata {
                            kda: kda_clone,
                            map: Some(map_name.clone()),
                            ..Default::default()
                        },
                    },
                });
            }
            for _ in 0..s_ref.deaths {
                let id = next_id(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id,
                    event_type: EventType::Death,
                    timestamp: 0.0,
                    data: EventData {
                        name: "Death".to_string(),
                        actor: None,
                        target: agent_name.clone(),
                        participants: vec![],
                        metadata: EventMetadata {
                            kda: kda_clone,
                            map: Some(map_name.clone()),
                            ..Default::default()
                        },
                    },
                });
            }
            for _ in 0..s_ref.assists {
                let id = next_id(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id,
                    event_type: EventType::Assist,
                    timestamp: 0.0,
                    data: EventData {
                        name: "Assist".to_string(),
                        actor: agent_name.clone(),
                        target: None,
                        participants: vec![],
                        metadata: EventMetadata {
                            kda: kda_clone,
                            map: Some(map_name.clone()),
                            ..Default::default()
                        },
                    },
                });
            }
        }

        // Emit GameEnd event.
        {
            let id = next_id(&mut s);
            s.pending_events.push(RecordingEvent {
                event_id: id,
                event_type: EventType::GameEnd,
                timestamp: 0.0,
                data: EventData {
                    name: "GameEnd".to_string(),
                    actor: None,
                    target: None,
                    participants: vec![],
                    metadata: EventMetadata {
                        kda: kda_clone,
                        map: Some(map_name.clone()),
                        team: player.map(|p| p.team_id.clone()),
                        ..Default::default()
                    },
                },
            });
        }

        // Persist final state so `get_game_state()` returns real data even
        // after the in-game API has shut down.
        s.final_state = Some(GameState {
            is_in_round: false,
            character_name: agent_name,
            kda: kda_clone,
            map_name: Some(map_name),
            game_mode: None,
            game_outcome: outcome,
            round_number: None,
            team: player.map(|p| p.team_id.clone()),
            score: None,
        });
    }
}

// GameIntegrationTrait

#[async_trait::async_trait]
impl GameIntegrationTrait for ValorantIntegration {
    async fn initialize(&mut self) -> Result<()> {
        match self.client.create_session().await {
            Ok(session) => {
                log!("✅ Valorant integration initialised – PUUID: {}", session.puuid);
                self.state.write().await.session = Some(session);
            }
            Err(e) => {
                // Non-fatal: game may still be launching.
                log!("⚠️ Valorant: session not yet available during init: {}", e);
            }
        }
        Ok(())
    }

    async fn get_game_state(&self) -> Result<GameState> {
        // If we have a cached post-game final state, use it so the recorder
        // can attach KDA even after the in-game API has gone away.
        {
            let s = self.state.read().await;
            if !s.session_loop_state.is_empty() && s.session_loop_state != "INGAME" {
                if let Some(final_state) = &s.final_state {
                    return Ok(final_state.clone());
                }
            }
        }

        let s = self.state.read().await;
        Ok(GameState {
            is_in_round: s.session_loop_state == "INGAME",
            character_name: s.current_agent.clone(),
            kda: None, // KDA only available post-game
            map_name: s.current_map.clone(),
            game_mode: s.current_queue.clone(),
            game_outcome: None,
            round_number: Some(s.last_round_total + 1),
            team: s.current_team.clone(),
            score: None,
        })
    }

    async fn is_in_round(&self) -> bool {
        let s = self.state.read().await;
        s.session_loop_state == "INGAME"
    }

    async fn get_character_name(&self) -> Result<Option<String>> {
        Ok(self.state.read().await.current_agent.clone())
    }

    fn get_game_name(&self) -> &str {
        "VALORANT"
    }

    async fn get_new_events(&self) -> Result<Option<Vec<RecordingEvent>>> {
        // Run the poll logic first.
        self.poll().await;

        // Drain accumulated events.
        let mut s = self.state.write().await;
        if s.pending_events.is_empty() {
            return Ok(Some(vec![]));
        }
        Ok(Some(std::mem::take(&mut s.pending_events)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Default for ValorantIntegration {
    fn default() -> Self {
        Self::new().expect("Failed to create ValorantIntegration")
    }
}