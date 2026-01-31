use std::any::Any;
use super::{GameIntegrationTrait, GameState, Score};
use anyhow::Result;
use shaco::ingame::IngameClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::log;

#[derive(Debug, Clone)]
pub struct GameEvent {
    pub event_id: u32,
    pub event_name: String,
    pub event_time: f64,
    pub killer_name: Option<String>,
    pub victim_name: Option<String>,
    pub assisters: Vec<String>,
}

pub struct LeagueOfLegendsIntegration {
    client: Arc<RwLock<IngameClient>>,
    last_event_id: Arc<RwLock<u32>>,
    pub(crate) player_name: Arc<RwLock<Option<String>>>,
}

impl LeagueOfLegendsIntegration {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(IngameClient::new().unwrap())),
            last_event_id: Arc::new(RwLock::new(0)),
            player_name: Arc::new(RwLock::new(None)),
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

    pub async fn get_new_events(&self) -> Result<Vec<GameEvent>> {
        let client = self.client.read().await;
        let last_id = *self.last_event_id.read().await;

        let shaco_events = client.event_data(None).await?;
        let mut new_events = Vec::new();

        for event in shaco_events {
            let event_id = event.get_event_id();
            if event_id <= last_id {
                continue;
            }

            let game_event = match event {
                shaco::model::ingame::GameEvent::ChampionKill(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "ChampionKill".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: Some(e.killer_name.to_string()),
                    victim_name: Some(e.victim_name.to_string()),
                    assisters: e.assisters.to_vec(),
                },
                shaco::model::ingame::GameEvent::DragonKill(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "DragonKill".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: Some(e.killer_name.to_string()),
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::BaronKill(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "BaronKill".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: Some(e.killer_name.to_string()),
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::Ace(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "Ace".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: None,
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::FirstBlood(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "FirstBlood".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: None,
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::TurretKilled(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "TurretKilled".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: Some(e.killer_name.to_string()),
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::InhibKilled(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "InhibKilled".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: Some(e.killer_name.to_string()),
                    victim_name: None,
                    assisters: vec![],
                },
                shaco::model::ingame::GameEvent::GameEnd(e) => GameEvent {
                    event_id: e.event_id,
                    event_name: "GameEnd".to_string(),
                    event_time: e.event_time as f64,
                    killer_name: None,
                    victim_name: None,
                    assisters: vec![],
                },
                _ => continue,
            };

            new_events.push(game_event);
        }

        if let Some(last) = new_events.last() {
            *self.last_event_id.write().await = last.event_id;
        }

        Ok(new_events)
    }

    pub fn is_highlight_event(event: &GameEvent, player_name: &str) -> bool {
        match event.event_name.as_str() {
            "ChampionKill" => {
                event.killer_name.as_deref() == Some(player_name)
                    || event.victim_name.as_deref() == Some(player_name)
            }
            "Multikill" | "Ace" | "FirstBlood" | "TurretKilled" | "InhibKilled" | "DragonKill"
            | "BaronKill" => true,
            _ => false,
        }
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

            if let Ok(name) = client.active_player_name().await {
                *self.player_name.write().await = Some(name);
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
        let events = LeagueOfLegendsIntegration::get_new_events(self).await?;
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
