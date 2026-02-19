// game_integration/games/league_of_legends/integration.rs
use crate::domain::game_stats::{GameOutcome, KDA};
use crate::game_integration::{GameIntegrationTrait};
use crate::log;

use crate::game_integration::games::league_of_legends::game_mode::game_mode_to_string;
use anyhow::Result;
use shaco::ingame::IngameClient;
use shaco::model::ingame::GameResult;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::domain::events::{EventType, RecordingEvent};
use crate::domain::game::GameState;

#[derive(Debug, Clone, Default)]
struct PlayerState {
    /// Full Riot ID (e.g. `"PlayerName#EUW"`).
    full_name: Option<String>,
    /// The part before `#`, used for event matching.
    short_name: Option<String>,
}

impl PlayerState {
    fn is_known(&self) -> bool {
        self.full_name.is_some()
    }

    /// Returns the short name (before `#`) used for event matching, falling
    /// back to the full name if no `#` separator is present.
    fn short(&self) -> Option<&str> {
        self.short_name.as_deref().or(self.full_name.as_deref())
    }
}

pub struct LeagueOfLegendsIntegration {
    client: IngameClient,
    /// All player identity lives here – one lock instead of three.
    player: Arc<RwLock<PlayerState>>,
    /// Highest event-id we've already processed (prevents replaying events).
    last_event_id: Arc<RwLock<u32>>,
    /// Snapshot captured at the `GameEnd` event so that the recorder can
    /// attach final KDA even after the in-game API shuts down.
    final_state: Arc<RwLock<Option<GameState>>>,
}

impl LeagueOfLegendsIntegration {
    pub fn new() -> Self {
        Self {
            client: IngameClient::new(),
            player: Arc::new(RwLock::new(PlayerState::default())),
            last_event_id: Arc::new(RwLock::new(0)),
            final_state: Arc::new(RwLock::new(None)),
        }
    }

    async fn fetch_champion_name(&self) -> Result<Option<String>> {
        let player = self.player.read().await;
        let full_name = match player.full_name.as_deref() {
            Some(n) => n.to_owned(),
            None => return Ok(None),
        };
        drop(player);

        let players = self.client.player_list(None).await?;
        Ok(players
            .into_iter()
            .find(|p| p.summoner_name == full_name)
            .map(|p| p.champion_name))
    }

    /// Returns KDA for the active player.
    /// - `Some(KDA)` → successfully fetched (even if 0/0/0)
    /// - `None` → could not fetch (API error / player unknown)
    async fn fetch_kda(&self) -> Option<KDA> {
        let player = self.player.read().await;
        let Some(name) = player.full_name.clone() else {
            return None;
        };
        drop(player);

        match self.client.player_scores(&name).await {
            Ok(s) => Some(KDA {
                kills: s.kills as u32,
                deaths: s.deaths as u32,
                assists: s.assists as u32,
            }),
            Err(_) => None,
        }
    }

    /// Refreshes the player identity from the in-game API. Called inside
    /// `get_game_state` so the names are always up-to-date.
    async fn refresh_player_identity(&self) -> Result<()> {
        let full_name = self.client.active_player_name().await?;
        let short_name = full_name.split('#').next().map(str::to_string);

        let mut player = self.player.write().await;
        player.full_name = Some(full_name);
        player.short_name = short_name;
        Ok(())
    }

    /// Converts LoL-specific events into generic [`RecordingEvent`]s, filtering to
    /// only those that involve the active player.
    pub async fn fetch_new_events(&self) -> Result<Vec<RecordingEvent>> {
        let shaco_events = self.client.event_data(None).await?;

        let last_id = *self.last_event_id.read().await;
        let current_kda = self.fetch_kda().await;
        let player_short = self
            .player
            .read()
            .await
            .short()
            .map(str::to_string)
            .unwrap_or_default();

        let mut new_events: Vec<RecordingEvent> = Vec::new();

        for event in shaco_events {
            let event_id = event.get_event_id();
            if event_id <= last_id {
                continue;
            }

            let maybe_event = self.convert_event(event, &player_short, current_kda).await;
            if let Some(game_event) = maybe_event {
                new_events.push(game_event);
            }
        }

        if let Some(last) = new_events.last() {
            *self.last_event_id.write().await = last.event_id;
        }

        Ok(new_events)
    }

    async fn convert_event(
        &self,
        event: shaco::model::ingame::GameEvent,
        player_short: &str,
        kda: Option<KDA>,
    ) -> Option<RecordingEvent> {
        use shaco::model::ingame::Killer;

        match event {
            shaco::model::ingame::GameEvent::ChampionKill(e) => {
                let killer_name = match &e.killer_name {
                    Killer::Summoner(name) => Some(name.as_str()),
                    _ => None,
                };

                let is_killer = killer_name == Some(player_short);
                let is_victim = e.victim_name == player_short;
                let is_assist = e.assisters.contains(&player_short.to_string());

                if !(is_killer || is_victim || is_assist) {
                    return None;
                }

                let event_type = if is_killer && is_victim {
                    // Practice Tool "kill champion button" player killed themselves → count as death
                    EventType::Death
                } else if is_killer {
                    EventType::Kill
                } else if is_victim {
                    EventType::Death
                } else {
                    EventType::Assist
                };

                Some(
                    RecordingEvent::new(
                        e.event_id,
                        event_type.clone(),
                        e.event_time as f64,
                        event_type.to_string(),
                    )
                    .with_actor(killer_name.unwrap_or("Unknown").to_owned())
                    .with_target(e.victim_name)
                    .with_participants(e.assisters)
                    .with_kda(kda),
                )
            }

            shaco::model::ingame::GameEvent::DragonKill(e) => {
                let killer_name = match &e.killer_name {
                    Killer::Summoner(name) => Some(name.as_str()),
                    _ => None,
                };
                let is_killer = killer_name == Some(player_short);
                let is_assist = e.assisters.contains(&player_short.to_string());

                if !(is_killer || is_assist) {
                    return None;
                }

                Some(
                    RecordingEvent::new(
                        e.event_id,
                        EventType::Objective,
                        e.event_time as f64,
                        "Dragon".to_string(),
                    )
                    .with_actor(killer_name.unwrap_or("Unknown").to_owned())
                    .with_kda(kda),
                )
            }

            shaco::model::ingame::GameEvent::BaronKill(e) => Some(
                RecordingEvent::new(
                    e.event_id,
                    EventType::Objective,
                    e.event_time as f64,
                    "BaronKill".to_string(),
                )
                .with_actor(e.killer_name.to_string())
                .with_kda(kda),
            ),

            shaco::model::ingame::GameEvent::Ace(e) => Some(
                RecordingEvent::new(
                    e.event_id,
                    EventType::Special,
                    e.event_time as f64,
                    "Ace".to_string(),
                )
                .with_kda(kda),
            ),

            shaco::model::ingame::GameEvent::TurretKilled(e) => Some(
                RecordingEvent::new(
                    e.event_id,
                    EventType::Objective,
                    e.event_time as f64,
                    "Turret".to_string(),
                )
                .with_actor(e.killer_name.to_string())
                .with_kda(kda),
            ),

            shaco::model::ingame::GameEvent::InhibKilled(e) => Some(
                RecordingEvent::new(
                    e.event_id,
                    EventType::Objective,
                    e.event_time as f64,
                    "Inhibitor".to_string(),
                )
                .with_actor(e.killer_name.to_string())
                .with_kda(kda),
            ),

            shaco::model::ingame::GameEvent::GameEnd(_e) => {
                let outcome = match _e.result {
                    GameResult::Win => GameOutcome::Victory,
                    GameResult::Lose => GameOutcome::Defeat,
                };
                self.capture_final_state(kda, outcome).await;
                None
            }

            _ => None,
        }
    }

    /// Persists a final [`GameState`] snapshot so the recorder can attach KDA and other metadata
    /// after the in-game API has gone away.
    async fn capture_final_state(&self, kda: Option<KDA>, game_outcome: GameOutcome) {
        let champion = self.fetch_champion_name().await.ok().flatten();

        let snapshot = GameState {
            character_name: champion,
            kda: kda.clone(),
            game_outcome: Some(game_outcome),
            ..Default::default()
        };

        log!(
            "📸 Final state captured at GameEnd: {}",
            kda.map_or("None".to_string(), |k| k.to_string())
        );

        *self.final_state.write().await = Some(snapshot);
    }
}

#[async_trait::async_trait]
impl GameIntegrationTrait for LeagueOfLegendsIntegration {
    async fn initialize(&mut self) -> Result<()> {
        match self.client.active_player().await {
            Ok(p) => {
                let full_name = p.summoner_name.clone();
                let short_name = full_name.split('#').next().map(str::to_string);
                let mut player = self.player.write().await;
                player.full_name = Some(full_name.clone());
                player.short_name = short_name;
                log!("✅ Connected to LoL In-Game API – player: {}", full_name);
            }
            Err(e) => {
                // Not fatal: the game may not have started a match yet.
                log!("⚠️  LoL In-Game API not available yet: {}", e);
            }
        }
        Ok(())
    }

    async fn get_game_state(&self) -> Result<GameState> {
        // If no active game, return the cached final state (or default).
        if !self.client.active_game().await {
            return Ok(self.final_state.read().await.clone().unwrap_or_default());
        }

        // Refresh identity eagerly so downstream callers always see current names.
        if let Err(e) = self.refresh_player_identity().await {
            log!("⚠️  Could not refresh player identity: {}", e);
        }

        let mut state = GameState {
            is_in_round: true,
            ..Default::default()
        };

        state.character_name = self.fetch_champion_name().await?;

        if let Ok(stats) = self.client.game_stats().await {
            state.map_name = Some(stats.map_name.to_string());
            state.game_mode = Some(game_mode_to_string(&stats.game_mode));
        }

        state.kda = self.fetch_kda().await;

        Ok(state)
    }

    async fn is_in_round(&self) -> bool {
        self.client.active_game().await
    }

    async fn get_character_name(&self) -> Result<Option<String>> {
        self.fetch_champion_name().await
    }

    fn get_game_name(&self) -> &str {
        "League of Legends"
    }

    async fn get_new_events(&self) -> Result<Option<Vec<RecordingEvent>>> {
        let events = self.fetch_new_events().await?;
        Ok(Some(events))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Default for LeagueOfLegendsIntegration {
    fn default() -> Self {
        Self::new()
    }
}
