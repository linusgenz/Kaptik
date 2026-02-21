// game_integration/games/valorant/integration.rs
//
// Detection strategy:
//   Poll core-game/v1/players/{puuid} every second.
//   200 → INGAME  (is_in_round = true, fetch match context via core-game/v1/matches/{id})
//   non-200 → MENUS  (is_in_round = false)

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
use super::maps::{queue_id_to_mode};
use super::models::MatchDetailsResponse;
use crate::game_integration::games::valorant::agent_lookup::agent_id_to_name;

// ─────────────────────────────────────────────────────────────────────────────
// File Logger
// ─────────────────────────────────────────────────────────────────────────────

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use crate::game_integration::games::valorant::map_lookup::map_id_to_name;

fn log_file_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("valorant_debug.log")
}

fn flog(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, msg);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file_path()) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

macro_rules! flog {
    ($($arg:tt)*) => { flog(&format!($($arg)*)); };
}

#[derive(Default)]
struct InnerState {
    session: Option<ClientSession>,

    is_ingame: bool,

    current_match_id: Option<String>,
    current_agent: Option<String>,
    current_map: Option<String>,
    current_team: Option<String>,
    current_queue: Option<String>,

    // Event bookkeeping
    next_event_id: u32,
    pending_events: Vec<RecordingEvent>,

    processed_match_ids: HashSet<String>,
    final_state: Option<GameState>,

    poll_count: u64,
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
                flog!("[SESSION] ✅ Created – PUUID={} region={} shard={}", session.puuid, session.region, session.shard);
                log!("✅ Valorant session created – region={} shard={}", session.region, session.shard);
                self.state.write().await.session = Some(session.clone());
                Some(session)
            }
            Err(e) => {
                flog!("[SESSION] ❌ Failed: {}", e);
                None
            }
        }
    }

    // ─── Core poll ────────────────────────────────────────────────────────────

    async fn poll(&self) {
        let poll_count = {
            let mut s = self.state.write().await;
            s.poll_count += 1;
            s.poll_count
        };

        let session = match self.get_or_create_session().await {
            Some(s) => s,
            None => return,
        };

        let was_ingame = self.state.read().await.is_ingame;

        match self.client.get_current_game_player(&session).await {
            Ok(player) => {
                if !was_ingame {
                    flog!("[POLL #{}] ✅ core-game → 200 (match={}) – entering INGAME", poll_count, player.match_id);
                    log!("🎮 Valorant: entered INGAME (match {})", player.match_id);
                    self.on_entered_ingame(&session, &player.match_id).await;
                    self.state.write().await.is_ingame = true;
                }
            }
            Err(_) => {
                if was_ingame {
                    flog!("[POLL #{}] core-game → non-200 – leaving INGAME", poll_count);
                    log!("🏁 Valorant: game ended");
                    self.on_game_ended(&session).await;
                    self.state.write().await.is_ingame = false;
                }
            }
        }
    }

    // ─── Transition handlers ──────────────────────────────────────────────────

    async fn on_entered_ingame(&self, session: &ClientSession, match_id: &str) {
        match self.client.get_current_game_match(session, match_id).await {
            Err(e) => {
                flog!("[INGAME] ❌ get_current_game_match failed: {}", e);
            }
            Ok(match_data) => {
                let map_name = map_id_to_name(&match_data.map_id).await;
                let own_player = match_data.players.iter().find(|p| p.subject == session.puuid);
                let agent = if let Some(p) = own_player {
                    agent_id_to_name(&p.character_id).await
                } else {
                    None
                };
                let team = own_player.map(|p| p.team_id.clone());
                let queue = match_data.mode_id
                    .split('/')
                    .last()
                    .map(|m| queue_id_to_mode(m).to_string());

                flog!("[INGAME] ✅ map={} agent={:?} team={:?}", map_name, agent, team);
                log!("✅ Valorant INGAME: map={} agent={:?}", map_name, agent);

                let mut s = self.state.write().await;
                s.current_match_id = Some(match_data.match_id.clone());
                s.current_map = Some(map_name);
                s.current_agent = agent;
                s.current_team = team;
                s.current_queue = queue;
            }
        }

        self.on_round_started(1, 0, 0).await;
    }

    async fn on_round_started(&self, round_number: u32, ally: u32, enemy: u32) {
        let map = self.state.read().await.current_map.clone();
        let mut s = self.state.write().await;
        let id = s.next_id();
        let mut extra = std::collections::HashMap::new();
        extra.insert("round".to_string(), round_number.to_string());
        extra.insert("ally_score".to_string(), ally.to_string());
        extra.insert("enemy_score".to_string(), enemy.to_string());
        s.pending_events.push(RecordingEvent {
            event_id: id,
            event_type: EventType::RoundStart,
            timestamp: 0.0,
            data: EventData {
                name: format!("Round {}", round_number),
                actor: None,
                target: None,
                participants: vec![],
                metadata: EventMetadata { map, extra, ..Default::default() },
            },
        });
    }

    async fn on_game_ended(&self, session: &ClientSession) {
        {
            let mut s = self.state.write().await;
            s.current_match_id = None;
            s.current_agent = None;
            s.current_map = None;
            s.current_team = None;
            s.current_queue = None;
        }

        let match_id = match self.client.get_latest_match_id(session).await {
            Ok(Some(id)) => id,
            Ok(None) => { flog!("[GAME_END] ❌ Match history empty"); return; }
            Err(e) => { flog!("[GAME_END] ❌ History fetch failed: {}", e); return; }
        };

        {
            let s = self.state.read().await;
            if s.processed_match_ids.contains(&match_id) {
                flog!("[GAME_END] Match {} already processed", match_id);
                return;
            }
        }

        flog!("[GAME_END] Waiting 5s for PD to finalize match {}...", match_id);
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        match self.client.get_match_details(session, &match_id).await {
            Ok(details) => {
                flog!("[GAME_END] ✅ Match details received – processing");
                self.process_match_details(&match_id, &details, &session.puuid).await;
            }
            Err(e) => { flog!("[GAME_END] ❌ Match details fetch failed: {}", e); }
        }
    }

    async fn process_match_details(&self, match_id: &str, details: &MatchDetailsResponse, puuid: &str) {
        let player = details.players.iter().find(|p| p.subject == puuid);
        let stats = player.and_then(|p| p.stats.as_ref());
        let player_team_id = player.map(|p| p.team_id.as_str()).unwrap_or("");

        let kda = stats.map(|s| KDA { kills: s.kills, deaths: s.deaths, assists: s.assists });

        let outcome = details.teams.iter()
            .find(|t| t.team_id == player_team_id)
            .map(|t| if t.won { GameOutcome::Victory } else { GameOutcome::Defeat });

        let agent_name: Option<String> = if let Some(p) = player {
            agent_id_to_name(&p.character_id).await
        } else {
            None
        };

        let map_name = map_id_to_name(&details.match_info.map_id).await;

        flog!("[POST_GAME] map={} kda={:?} outcome={:?}", map_name, kda, outcome);
        log!("📊 Valorant post-game: map={} kda={:?} outcome={:?}", map_name, kda, outcome);

        let mut s = self.state.write().await;
        s.processed_match_ids.insert(match_id.to_string());

        let mk = |state: &mut InnerState| -> u32 {
            let id = state.next_event_id;
            state.next_event_id += 1;
            id
        };

        if let Some(st) = stats {
            for _ in 0..st.kills {
                let id = mk(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id, event_type: EventType::Kill, timestamp: 0.0,
                    data: EventData {
                        name: "Kill".to_string(), actor: agent_name.clone(), target: None,
                        participants: vec![],
                        metadata: EventMetadata { kda, map: Some(map_name.clone()), ..Default::default() },
                    },
                });
            }
            for _ in 0..st.deaths {
                let id = mk(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id, event_type: EventType::Death, timestamp: 0.0,
                    data: EventData {
                        name: "Death".to_string(), actor: None, target: agent_name.clone(),
                        participants: vec![],
                        metadata: EventMetadata { kda, map: Some(map_name.clone()), ..Default::default() },
                    },
                });
            }
            for _ in 0..st.assists {
                let id = mk(&mut s);
                s.pending_events.push(RecordingEvent {
                    event_id: id, event_type: EventType::Assist, timestamp: 0.0,
                    data: EventData {
                        name: "Assist".to_string(), actor: agent_name.clone(), target: None,
                        participants: vec![],
                        metadata: EventMetadata { kda, map: Some(map_name.clone()), ..Default::default() },
                    },
                });
            }
        }

        {
            let id = mk(&mut s);
            s.pending_events.push(RecordingEvent {
                event_id: id, event_type: EventType::GameEnd, timestamp: 0.0,
                data: EventData {
                    name: "GameEnd".to_string(), actor: None, target: None, participants: vec![],
                    metadata: EventMetadata {
                        kda, map: Some(map_name.clone()),
                        team: player.map(|p| p.team_id.clone()),
                        ..Default::default()
                    },
                },
            });
        }

        s.final_state = Some(GameState {
            is_in_round: false,
            character_name: agent_name,
            kda,
            map_name: Some(map_name),
            game_mode: None,
            game_outcome: outcome,
            round_number: None,
            team: player.map(|p| p.team_id.clone()),
            score: None,
        });
    }
}

// ─── GameIntegrationTrait ─────────────────────────────────────────────────────

#[async_trait::async_trait]
impl GameIntegrationTrait for ValorantIntegration {
    async fn initialize(&mut self) -> Result<()> {
        match self.client.create_session().await {
            Ok(session) => {
                flog!("[INIT] ✅ Session ready – PUUID={}", session.puuid);
                log!("✅ Valorant integration initialised – PUUID: {}", session.puuid);
                self.state.write().await.session = Some(session);
            }
            Err(e) => {
                flog!("[INIT] ⚠️ Session not yet available: {} – will retry on first poll", e);
            }
        }
        Ok(())
    }

    async fn get_game_state(&self) -> Result<GameState> {
        {
            let s = self.state.read().await;
            if !s.is_ingame {
                if let Some(final_state) = &s.final_state {
                    return Ok(final_state.clone());
                }
            }
        }

        let s = self.state.read().await;
        Ok(GameState {
            is_in_round: s.is_ingame,
            character_name: s.current_agent.clone(),
            kda: None,
            map_name: s.current_map.clone(),
            game_mode: s.current_queue.clone(),
            game_outcome: None,
            round_number: Some(1),
            team: s.current_team.clone(),
            score: None,
        })
    }

    async fn is_in_round(&self) -> bool {
        self.state.read().await.is_ingame
    }

    async fn get_character_name(&self) -> Result<Option<String>> {
        Ok(self.state.read().await.current_agent.clone())
    }

    fn get_game_name(&self) -> &str {
        "VALORANT"
    }

    async fn get_new_events(&self) -> Result<Option<Vec<RecordingEvent>>> {
        self.poll().await;
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