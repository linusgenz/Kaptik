use super::*;
use crate::log;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct GameIntegrationManager {
    integrations: HashMap<String, Arc<RwLock<Box<dyn GameIntegrationTrait>>>>,
    active_integration: Arc<RwLock<Option<String>>>,
    active_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl GameIntegrationManager {
    pub fn new() -> Self {
        let mut manager = Self {
            integrations: HashMap::new(),
            active_integration: Arc::new(RwLock::new(None)),
            active_task: Arc::new(Mutex::new(None)),
        };

        /* manager.register_integration(
            "VALORANT-Win64-Shipping.exe",
            Box::new(valorant::ValorantIntegration::new()),
        );*/

        manager.register_integration(
            "League of Legends.exe",
            Box::new(league_of_legends::LeagueOfLegendsIntegration::new()),
        );

        manager
    }

    pub fn register_integration(
        &mut self,
        exe_name: &str,
        integration: Box<dyn GameIntegrationTrait>,
    ) {
        self.integrations
            .insert(exe_name.to_lowercase(), Arc::new(RwLock::new(integration)));
    }

    pub async fn activate_for_game(&self, exe_name: &str) -> anyhow::Result<()> {
        let exe_lower = exe_name.to_lowercase();

        if let Some(integration) = self.integrations.get(&exe_lower) {
            integration.write().await.initialize().await?;
            *self.active_integration.write().await = Some(exe_lower);
            log!("✅ Integration aktiviert für: {}", exe_name);
            Ok(())
        } else {
            log!("⚠️  Keine Integration verfügbar für: {}", exe_name);
            *self.active_integration.write().await = None;
            Ok(())
        }
    }

    pub async fn get_current_state(&self) -> Option<GameState> {
        let active = self.active_integration.read().await;

        if let Some(ref game_name) = *active {
            if let Some(integration) = self.integrations.get(game_name) {
                return integration.read().await.get_game_state().await.ok();
            }
        }

        None
    }

    pub async fn is_in_round(&self) -> bool {
        let active = self.active_integration.read().await;

        if let Some(ref game_name) = *active {
            if let Some(integration) = self.integrations.get(game_name) {
                return integration.read().await.is_in_round().await;
            }
        }

        false
    }

    pub async fn get_character_name(&self) -> Option<String> {
        if let Some(state) = self.get_current_state().await {
            state.character_name
        } else {
            None
        }
    }

    pub async fn start_monitoring(&self) {
        if let Some(handle) = self.active_task.lock().await.take() {
            handle.abort();
        }

        let active = self.active_integration.clone();
        let integrations = self.integrations.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                interval.tick().await;

                let active_game = active.read().await;
                if active_game.is_none() {
                    continue;
                }

                let game_name = active_game.as_ref().unwrap();

                if let Some(_integration) = integrations.get(game_name) {
                    let integration = {
                        let guard = integrations.get(game_name).unwrap();
                        guard.clone()
                    };

                    if let Ok(state) = integration.read().await.get_game_state().await {
                        log!(
                            "[State] In-Round: {} | Champion: {:?} | Map: {:?}",
                            state.is_in_round,
                            state.character_name,
                            state.map_name
                        );
                    }

                    if let Some(lol_integration) = integration
                        .read()
                        .await
                        .as_any()
                        .downcast_ref::<league_of_legends::LeagueOfLegendsIntegration>(
                    ) {
                        match lol_integration.get_new_events().await {
                            Ok(events) => {
                                let player_name = {
                                    let guard = lol_integration.player_name.read().await;
                                    guard.clone().unwrap_or_else(|| "".to_string())
                                };

                                for event in events {
                                    if league_of_legends::LeagueOfLegendsIntegration::is_highlight_event(&event, &player_name) {
                                        log!(
                                            "✨ Highlight Event: {} | Killer: {:?} | Victim: {:?} | Assisters: {:?}",
                                            event.event_name, event.killer_name, event.victim_name, event.assisters
                                        );
                                    } else {
                                        log!(
                                            "[Event] {} | Killer: {:?} | Victim: {:?}",
                                            event.event_name, event.killer_name, event.victim_name
                                        );
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        });

        *self.active_task.lock().await = Some(handle);
    }

    pub async fn stop_active_integration(&self) {
        if let Some(handle) = self.active_task.lock().await.take() {
            handle.abort();
        }

        *self.active_integration.write().await = None;
        log!("🛑 Active integration stopped");
    }
}
