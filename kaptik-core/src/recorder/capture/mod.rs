// capture/mod.rs
use crate::apm::{APMTracker, input_hook::InputHook};
use crate::log;
use crate::recording_storage::{
    get_recording_path, save_recording_data,
};
use anyhow::Result;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use strategy::{CaptureMethod, CaptureStrategy, create_strategy};
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::domain::events::RecordingEvent;
use crate::domain::game::{GameName, GameState};
use crate::domain::recording::{RecordingData, RecordingMetadata};

pub(crate) mod core;
pub mod strategy;
mod windows_graphics;

pub struct WindowsCaptureRecorder {
    is_recording: Arc<RwLock<bool>>,
    strategy: Arc<RwLock<Box<dyn CaptureStrategy>>>,
    input_hook: InputHook,
    apm_tracker: Arc<Mutex<APMTracker>>,
    current_recording_data: Arc<RwLock<Option<RecordingData>>>,
    recording_start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl WindowsCaptureRecorder {
    pub fn new(method: CaptureMethod) -> Self {
        let apm_tracker = Arc::new(Mutex::new(APMTracker::new()));

        Self {
            is_recording: Arc::new(RwLock::new(false)),
            strategy: Arc::new(RwLock::new(create_strategy(method))),
            input_hook: InputHook::new(apm_tracker.clone()),
            apm_tracker,
            current_recording_data: Arc::new(RwLock::new(None)),
            recording_start_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_strategy(&self, method: CaptureMethod) -> Result<()> {
        if *self.is_recording.read().await {
            return Err(anyhow::anyhow!("Cannot change strategy while recording"));
        }

        *self.strategy.write().await = create_strategy(method);
        log!("📝 Capture method changed to: {:?}", method);
        Ok(())
    }

    pub async fn is_recording(&self) -> bool {
        *self.is_recording.read().await
    }

    /// Start recording a window.
    ///
    /// `game_name` is the canonical [`GameName`] for this session.  Pass
    /// [`GameName::from_display`] when an integration is active (so the name
    /// comes from the integration's hardcoded string), or
    /// [`GameName::from_window_title`] as a fallback when no integration is
    /// registered for the running game.
    pub async fn start_recording(
        &self,
        window_title: &str,
        game_name: GameName,
        game_state: Option<GameState>,
    ) -> Result<Uuid> {
        if *self.is_recording.read().await {
            return Err(anyhow::anyhow!("Already capturing"));
        }

        let metadata = if let Some(gs) = game_state {
            RecordingMetadata::with_game_state(game_name, gs.character_name, gs.map_name, gs.game_mode, gs.round_number)
        } else {
            RecordingMetadata::new(game_name)
        };

        let rid = metadata.recording_id;

        // Initialize RecordingData
        *self.current_recording_data.write().await = Some(RecordingData::new(metadata.clone()));
        *self.recording_start_time.write().await = Some(std::time::Instant::now());

        let filename = metadata.generate_filename();
        let output_path = self.get_output_path(&filename).await?;

        log!(
            "🎬 Starting recording: {} (game: {})",
            filename,
            metadata.game_name.display,
        );

        let mut strategy = self.strategy.write().await;
        strategy
            .start_capture(window_title, metadata, output_path)
            .await?;

        *self.is_recording.write().await = true;

        self.apm_tracker.lock().start_recording();
        self.input_hook.start();

        Ok(rid)
    }

    pub async fn stop_recording(&self, final_state: Option<GameState>) -> Result<()> {
        if !*self.is_recording.read().await {
            return Ok(());
        }

        let mut strategy = self.strategy.write().await;
        let metadata = strategy
            .get_metadata()
            .ok_or_else(|| anyhow::anyhow!("No metadata available for recording"))?
            .clone();

        self.input_hook.stop();
        self.apm_tracker.lock().stop_recording();

        let output_path = strategy.stop_capture().await?;

        log!("✅ Video recording saved to: {}", output_path.display());

        // Calculate duration
        let duration = if let Some(start_time) = *self.recording_start_time.read().await {
            start_time.elapsed().as_secs_f64()
        } else {
            0.0
        };

        // Get APM data
        let series = self.apm_tracker.lock().compute_apm_series(20.0, 1.0, true);

        if let Some(mut recording_data) = self.current_recording_data.write().await.take() {
            if let Some(state) = final_state {
                recording_data.metadata.set_kda(state.kda);
                recording_data.metadata.set_game_outcome(state.game_outcome);
            }

            recording_data.set_apm_data(series);
            recording_data.finalize(duration);

            let path = get_recording_path(&metadata.recording_id)?;

            let recording_data_clone = recording_data.clone();
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || {
                if let Err(e) = save_recording_data(&recording_data_clone, &path_clone) {
                    log!("❌ Failed to save recording data: {}", e);
                } else {
                    log!("💾 Recording data saved to: {}", path_clone.display());
                    log!(
                        "   📊 {} events, {} APM samples",
                        recording_data_clone.events.len(),
                        recording_data_clone.apm.series.len()
                    );
                    if let Some(avg) = recording_data_clone.apm.average_apm {
                        log!("   📈 Average APM: {:.1}", avg);
                    }
                }
            });
        }

        *self.recording_start_time.write().await = None;
        *self.is_recording.write().await = false;

        Ok(())
    }

    /// Add a game event to the current recording
    pub async fn add_event(&self, mut event: RecordingEvent) {
        if let Some(ref mut data) = *self.current_recording_data.write().await {
            if event.timestamp == 0.0 {
                if let Some(start_time) = *self.recording_start_time.read().await {
                    event.timestamp = start_time.elapsed().as_secs_f64();
                }
            }

            data.add_event(event);
        }
    }

    pub async fn get_current_metadata(&self) -> Option<RecordingMetadata> {
        let strategy = self.strategy.read().await;
        strategy.get_metadata().cloned()
    }

    pub async fn get_current_recording_id(&self) -> Option<Uuid> {
        self.current_recording_data
            .read()
            .await
            .as_ref()
            .map(|data| data.metadata.recording_id)
    }

    async fn get_output_path(&self, filename: &str) -> Result<PathBuf> {
        use crate::settings::SETTINGS;

        let settings = SETTINGS.read().await;
        let mut output_path = PathBuf::from(&settings.video_path);

        if output_path.as_os_str().is_empty() {
            output_path = dirs::video_dir().unwrap_or_else(|| PathBuf::from("../../.."));
            output_path.push("Kaptik");
        }

        std::fs::create_dir_all(&output_path)?;
        output_path.push(filename);

        Ok(output_path)
    }
}
