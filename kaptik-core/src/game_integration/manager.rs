// game_integration/manager.rs

use crate::game_integration::games::league_of_legends::integration::LeagueOfLegendsIntegration;
use crate::log;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use crate::domain::events::RecordingEvent;
use crate::domain::game::{GameIdentifier, GameState};
use crate::game_integration::GameIntegrationTrait;
use crate::game_integration::games::valorant::ValorantIntegration;

type IntegrationMap = HashMap<String, Arc<RwLock<Box<dyn GameIntegrationTrait>>>>;
type EventCallback = Arc<dyn Fn(RecordingEvent) + Send + Sync>;
type StopCallback  = Arc<dyn Fn() + Send + Sync>;

pub struct GameIntegrationManager {
    /// Integrations keyed by lower-cased executable name (the `exe_key` of
    /// [`GameIdentifier`]).  Using `String` as the key keeps the HashMap simple
    /// while [`GameIdentifier`] carries the display name alongside.
    integrations: IntegrationMap,

    ///  `None` when no game is running.
    active_id: Arc<RwLock<Option<GameIdentifier>>>,

    /// Handle for the background monitoring task.
    monitor_task: Arc<Mutex<Option<JoinHandle<()>>>>,

    /// Callback invoked for every new [`RecordingEvent`] – wired to the recorder.
    event_callback: Arc<RwLock<Option<EventCallback>>>,

    stop_recording_callback: Arc<RwLock<Option<StopCallback>>>,

    /// Most-recently seen [`GameState`] that passes [`GameState::is_meaningful`].
    /// Survives the in-game API shutting down so the recorder can attach metadata
    /// like KDA which is required at the finalization of a recording.
    last_known_state: Arc<RwLock<Option<GameState>>>,
}

impl GameIntegrationManager {
    /// Creates the manager and registers built-in integrations.
    pub fn new() -> Self {
        let mut manager = Self {
            integrations: HashMap::new(),
            active_id: Arc::new(RwLock::new(None)),
            monitor_task: Arc::new(Mutex::new(None)),
            event_callback: Arc::new(RwLock::new(None)),
            stop_recording_callback: Arc::new(RwLock::new(None)),
            last_known_state: Arc::new(RwLock::new(None)),
        };

        // Built-in integrations registered here.
        manager.register("League of Legends.exe", LeagueOfLegendsIntegration::new());

        match ValorantIntegration::new() {
            Ok(integration) => {
                manager.register("VALORANT-Win64-Shipping.exe", integration);
            }
            Err(e) => {
                log!("⚠️  ValorantIntegration init failed, skipping: {}", e);
            }
        }

        manager
    }

    /// Register a game integration. The exe name is normalised to lowercase
    /// internally; the display name is taken from [`GameIntegrationTrait::get_game_name`].
    pub fn register(
        &mut self,
        exe_name: &str,
        integration: impl GameIntegrationTrait + 'static,
    ) {
        let key = exe_name.to_lowercase();
        log!("📦 Registered integration for '{}'", exe_name);
        self.integrations
            .insert(key, Arc::new(RwLock::new(Box::new(integration))));
    }

    /// Activate the integration that corresponds to `exe_name`.
    ///
    /// - If no integration is registered for this exe, we still set
    ///   `active_id` to `None` so that state queries return defaults instead
    ///   of stale data from a previous game.
    /// - Returns `Ok(())` in all non-I/O-error cases (no integration ≠ error).
    pub async fn activate_for_game(&self, exe_name: &str) -> anyhow::Result<()> {
        let key = exe_name.to_lowercase();

        if let Some(integration) = self.integrations.get(&key) {
            // Build the identifier using the integration's canonical display name.
            let display = integration.read().await.get_game_name().to_owned();
            let id = GameIdentifier::new(&key, display);

            integration.write().await.initialize().await?;
            *self.active_id.write().await = Some(id.clone());

            if let Some(cb) = self.stop_recording_callback.read().await.clone() {
                integration
                    .read()
                    .await
                    .set_stop_recording_callback(cb)
                    .await;
                log!("🔗 Stop-recording callback wired into integration: {}", id.display_name);
            }

            log!("🎮 Integration activated: {} ({})", id.display_name, exe_name);
        } else {
            *self.active_id.write().await = None;
            log!("ℹ️  No integration available for '{}'", exe_name);
        }

        Ok(())
    }

    /// Deactivate the current integration and abort the monitoring task.
    pub async fn deactivate(&self) {
        self.stop_monitor().await;
        *self.active_id.write().await = None;
        log!("🛑 Active integration deactivated");
    }

    /// Wire a callback that will be invoked for every new [`RecordingEvent`].
    /// Overwrites any previously registered callback.
    pub async fn set_event_callback<F>(&self, callback: F)
    where
        F: Fn(RecordingEvent) + Send + Sync + 'static,
    {
        *self.event_callback.write().await = Some(Arc::new(callback));
    }

    pub async fn set_stop_recording_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.stop_recording_callback.write().await = Some(Arc::new(callback));
    }

    /// Returns the current [`GameState`] from the active integration, or
    /// `None` when no integration is active.
    ///
    /// As a side-effect, updates [`Self::last_known_state`] whenever the
    /// returned state is meaningful.
    pub async fn get_current_state(&self) -> Option<GameState> {
        let integration = self.active_integration().await?;
        match integration.read().await.get_game_state().await {
            Ok(state) => {
                if state.is_meaningful() {
                    *self.last_known_state.write().await = Some(state.clone());
                }
                Some(state)
            }
            Err(e) => {
                log!("⚠️  get_game_state error: {}", e);
                None
            }
        }
    }

    /// Returns the last state that passed [`GameState::is_meaningful`], or
    /// `None` if no meaningful state has been seen yet.
    pub async fn get_last_known_state(&self) -> Option<GameState> {
        self.last_known_state.read().await.clone()
    }

    pub async fn is_in_round(&self) -> bool {
        match self.active_integration().await {
            Some(i) => i.read().await.is_in_round().await,
            None => false,
        }
    }

    /// Convenience accessor for the character name.
    pub async fn get_character_name(&self) -> Option<String> {
        self.get_current_state().await?.character_name
    }

    /// Returns the [`GameIdentifier`] for the currently active integration.
    pub async fn active_game_identifier(&self) -> Option<GameIdentifier> {
        self.active_id.read().await.clone()
    }

    /// loop runs every second and:
    /// 1. Polls game state (updates `last_known_state`).
    /// 2. Drains new events and forwards them through `event_callback`.
    pub async fn start_monitoring(&self) {
        self.stop_monitor().await;

        let active_id    = self.active_id.clone();
        let integrations = self.integrations.clone();
        let callback     = self.event_callback.clone();
        let last_state   = self.last_known_state.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                interval.tick().await;

                let key = {
                    let guard = active_id.read().await;
                    guard.as_ref().map(|id| id.exe_key.clone())
                };
                let Some(key) = key else { continue };
                let Some(integration) = integrations.get(&key) else { continue };

                match integration.read().await.get_game_state().await {
                    Ok(state) => {
                        if state.is_meaningful() {
                            *last_state.write().await = Some(state.clone());
                        }
                        log!(
                            "[State] in_round={} | champion={:?} | map={:?} | kda={:?}",
                            state.is_in_round,
                            state.character_name,
                            state.map_name,
                            state.kda,
                        );
                    }
                    Err(e) => log!("⚠️  State poll error: {}", e),
                }

                match integration.read().await.get_new_events().await {
                    Ok(Some(events)) if !events.is_empty() => {
                        let cb_guard = callback.read().await;
                        for event in events {
                            log!(
                                "[Event] {} | actor={:?} | target={:?}",
                                event.data.name,
                                event.data.actor,
                                event.data.target,
                            );
                            if let Some(cb) = cb_guard.as_ref() {
                                cb(event);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => log!("⚠️  Event drain error: {}", e),
                }
            }
        });

        *self.monitor_task.lock().await = Some(handle);
    }

    /// Returns a clone of the `Arc` for the active integration, or `None`.
    async fn active_integration(
        &self,
    ) -> Option<Arc<RwLock<Box<dyn GameIntegrationTrait>>>> {
        let key = self.active_id.read().await.as_ref().map(|id| id.exe_key.clone())?;
        self.integrations.get(&key).cloned()
    }

    /// Abort and discard the current monitoring task.
    async fn stop_monitor(&self) {
        if let Some(handle) = self.monitor_task.lock().await.take() {
            handle.abort();
        }
    }
}

impl Default for GameIntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}