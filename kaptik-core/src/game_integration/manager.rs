use super::*;
use crate::game_integration::event_storage::{get_events_path, save_events_msgpack, RecordingEvents};
use crate::log;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct GameIntegrationManager {
    integrations: HashMap<String, Arc<RwLock<Box<dyn GameIntegrationTrait>>>>,
    active_integration: Arc<RwLock<Option<String>>>,
    active_task: Arc<Mutex<Option<JoinHandle<()>>>>,

    /// Event-Sammlung während der aktuellen Aufnahme
    current_recording: Arc<RwLock<Option<RecordingEvents>>>,

    /// Start-Zeitpunkt der Aufnahme für korrekte Event-Timestamps
    recording_start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl GameIntegrationManager {
    pub fn new() -> Self {
        let mut manager = Self {
            integrations: HashMap::new(),
            active_integration: Arc::new(RwLock::new(None)),
            active_task: Arc::new(Mutex::new(None)),
            current_recording: Arc::new(RwLock::new(None)),
            recording_start_time: Arc::new(RwLock::new(None)),
        };

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

    pub async fn start_event_recording(&self, recording_id: Uuid) {
        let active = self.active_integration.read().await;

        if let Some(ref game_name) = *active {
            if let Some(integration) = self.integrations.get(game_name) {
                let game_display_name = integration.read().await.get_game_name().to_string();

                let recording = RecordingEvents::new(game_display_name, recording_id);
                *self.current_recording.write().await = Some(recording);
                *self.recording_start_time.write().await = Some(std::time::Instant::now());

                log!("🎬 Event-Recording gestartet für Session: {}", recording_id);
            }
        }
    }

    pub async fn stop_event_recording(&self) -> anyhow::Result<()> {
        if let Some(recording) = self.current_recording.write().await.take() {
            let recording_id = recording.recording_id;

            // Events speichern
            let path = get_events_path(&recording_id)?;
            save_events_msgpack(&recording, &path)?;

            log!(
                "💾 Events gespeichert: {} Events für Recording {}",
                recording.events.len(),
                recording_id
            );
            log!("   Davon {} Highlights", recording.get_highlights().len());
            log!("   Pfad: {:?}", path);
        }

        *self.recording_start_time.write().await = None;
        Ok(())
    }

    pub async fn get_current_session_events(&self) -> Vec<events::GameEvent> {
        if let Some(ref recording) = *self.current_recording.read().await {
            recording.events.clone()
        } else {
            Vec::new()
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
        let current_recording = self.current_recording.clone();
        let recording_start_time = self.recording_start_time.clone();

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
                            "[State] In-Round: {} | Character: {:?} | Map: {:?}",
                            state.is_in_round,
                            state.character_name,
                            state.map_name
                        );
                    }

                    if let Ok(Some(events)) = integration.read().await.get_new_events().await {
                        let is_recording = current_recording.read().await.is_some();

                        for mut event in events {
                            if let Some(start_time) = *recording_start_time.read().await {
                                let elapsed = start_time.elapsed().as_secs_f64();
                                event.timestamp = elapsed;
                            }

                            if event.data.metadata.is_highlight {
                                log!(
                                    "✨ Highlight: {} | Actor: {:?} | Target: {:?}",
                                    event.data.name,
                                    event.data.actor,
                                    event.data.target
                                );
                            } else {
                                log!(
                                    "[Event] {} | Actor: {:?} | Target: {:?}",
                                    event.data.name,
                                    event.data.actor,
                                    event.data.target
                                );
                            }

                            if is_recording {
                                if let Some(ref mut recording) = *current_recording.write().await {
                                    recording.add_event(event);
                                }
                            }
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