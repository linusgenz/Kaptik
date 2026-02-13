use super::{GameIntegrationTrait, GameState, Score};
use crate::game_integration::events::{EventData, EventMetadata, EventType, GameEvent};
use crate::log;
use anyhow::Result;
use shaco::ingame::IngameClient;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LeagueOfLegendsIntegration {
    client: Arc<RwLock<IngameClient>>,
    last_event_id: Arc<RwLock<u32>>,
    pub(crate) player_name: Arc<RwLock<Option<String>>>,
    pub(crate) player_name_short: Arc<RwLock<Option<String>>>,
}

impl LeagueOfLegendsIntegration {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(IngameClient::new())),
            last_event_id: Arc::new(RwLock::new(0)),
            player_name: Arc::new(RwLock::new(None)),
            player_name_short: Arc::new(RwLock::new(None)),
        }
    }

    async fn fetch_champion_name(&self) -> Result<Option<String>> {
        let client = self.client.read().await;
        let player_name = self.player_name.read().await.clone();

        let players = client.player_list(None).await?;

        if let Some(name) = player_name {
            for p in players {
                if p.summoner_name == name {
                    return Ok(Some(p.champion_name.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Konvertiert LoL-spezifische Events in generische GameEvents
    pub async fn fetch_new_events(&self) -> Result<Vec<GameEvent>> {
        let client = self.client.read().await;
        let last_id = *self.last_event_id.read().await;

        let shaco_events = client.event_data(None).await?;
        let mut new_events = Vec::new();

        // KDA für Metadata abrufen
        let current_kda = self.get_player_scores().await.unwrap_or((0, 0, 0));
        let player_name_short = self.player_name_short.read().await.clone().unwrap_or_default();

        for event in shaco_events {
            let event_id = event.get_event_id();
            if event_id <= last_id {
                continue;
            }

            let game_event = match event {
                shaco::model::ingame::GameEvent::ChampionKill(e) => {
                    let killer_player = match &e.killer_name {
                        shaco::model::ingame::Killer::Summoner(name) => Some(name),
                        _ => None,
                    };

                    let is_player_killer = killer_player == Some(&player_name_short);
                    let is_player_victim = e.victim_name == player_name_short;
                    let is_player_assist = e.assisters.contains(&player_name_short);
                    
                    if !(is_player_killer || is_player_victim || is_player_assist) {
                        continue;
                    }

                    let event_type = if is_player_killer {
                        EventType::Kill
                    } else if is_player_victim {
                        EventType::Death
                    } else {
                        EventType::Assist
                    };

                    GameEvent::new(
                        e.event_id,
                        event_type.clone(),
                        e.event_time as f64,
                        event_type.to_string(),
                    )
                        .with_actor(killer_player.cloned().unwrap_or("Unknown".into()))
                        .with_target(e.victim_name.clone())
                        .with_participants(e.assisters.clone())
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)
                }
                shaco::model::ingame::GameEvent::DragonKill(e) => {
                    let killer_player = match &e.killer_name {
                        shaco::model::ingame::Killer::Summoner(name) => Some(name),
                        _ => None,
                    };

                    let is_player_killer = killer_player == Some(&player_name_short);
                    let is_player_assist = e.assisters.contains(&player_name_short);

                    if !(is_player_killer || is_player_assist) {
                        continue;
                    }

                    GameEvent::new(
                        e.event_id,
                        EventType::Objective,
                        e.event_time as f64,
                        "Dragon".to_string(),
                    )
                        .with_actor(killer_player.cloned().unwrap_or("Unknown".into()))
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)
                }
                shaco::model::ingame::GameEvent::BaronKill(e) => {
                    GameEvent::new(
                        e.event_id,
                        EventType::Objective,
                        e.event_time as f64,
                        "BaronKill".to_string(),
                    )
                        .with_actor(e.killer_name.clone().to_string())
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)
                }
                shaco::model::ingame::GameEvent::Ace(e) => {
                    GameEvent::new(
                        e.event_id,
                        EventType::Special,
                        e.event_time as f64,
                        "Ace".to_string(),
                    )
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)
                }
                shaco::model::ingame::GameEvent::TurretKilled(e) => {
                    GameEvent::new(
                        e.event_id,
                        EventType::Objective,
                        e.event_time as f64,
                        "Turret".to_string(),
                    )
                        .with_actor(e.killer_name.clone().to_string())
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)

                }
                shaco::model::ingame::GameEvent::InhibKilled(e) => {
                    GameEvent::new(
                        e.event_id,
                        EventType::Objective,
                        e.event_time as f64,
                        "Inhibitor".to_string(),
                    )
                        .with_actor(e.killer_name.clone().to_string())
                        .with_kda(current_kda.0, current_kda.1, current_kda.2)
                }
                _ => continue,
            };

            new_events.push(game_event);
        }

        if let Some(last) = new_events.last() {
            *self.last_event_id.write().await = last.event_id;
        }

        Ok(new_events)
    }

    pub async fn get_player_scores(&self) -> Result<(u32, u32, u32)> {
        let client = self.client.read().await;

        if let Some(player_name) = self.player_name.read().await.as_ref() {
            match client.player_scores(player_name).await {
                Ok(scores) => Ok((
                    scores.kills as u32,
                    scores.deaths as u32,
                    scores.assists as u32,
                )),
                Err(_) => Ok((0, 0, 0)),
            }
        } else {
            Ok((0, 0, 0))
        }
    }
}

#[async_trait::async_trait]
impl GameIntegrationTrait for LeagueOfLegendsIntegration {
    async fn initialize(&mut self) -> Result<()> {
        let client = self.client.read().await;

        match client.active_player().await {
            Ok(active_player) => {
                let name = active_player.summoner_name.clone();

                log!(
                    "Connected to League of Legends In-Game API - Player: {}",
                    name
                );
                *self.player_name.write().await = Some(name);
                Ok(())
            }
            Err(e) => {
                log!("League of Legends In-Game API not available yet: {}", e);
                Ok(())
            }
        }
    }

    async fn get_game_state(&self) -> Result<GameState> {
        let client = self.client.read().await;
        let mut state = GameState::default();

        if client.active_game().await {
            state.is_in_round = true;

            if let Ok(full_name) = client.active_player_name().await {
                let short_name = full_name.split('#').next().map(|s| s.to_string());

                *self.player_name_short.write().await = short_name;

                *self.player_name.write().await = Some(full_name);
            }

            state.character_name = self.fetch_champion_name().await?;

            if let Ok(stats) = client.game_stats().await {
                state.map_name = Some(stats.map_name.to_string());
            }

            if let Some(name) = self.player_name.read().await.as_ref() {
                if let Ok(scores) = client.player_scores(name).await {
                    state.score = Some(Score {
                        team1: scores.kills as u32,
                        team2: scores.deaths as u32,
                    });
                }
            }
        }

        Ok(state)
    }

    async fn is_in_round(&self) -> bool {
        let client = self.client.read().await;
        client.active_game().await
    }

    async fn get_character_name(&self) -> Result<Option<String>> {
        self.fetch_champion_name().await
    }

    fn get_game_name(&self) -> &str {
        "League of Legends"
    }

    async fn get_new_events(&self) -> Result<Option<Vec<GameEvent>>> {
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