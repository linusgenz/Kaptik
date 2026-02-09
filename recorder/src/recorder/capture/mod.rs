use super::RecordingMetadata;
use crate::game_integration::GameState;
use crate::log;
use crate::recorder::apm::{input_hook::InputHook, save_apm_msgpack, APMData, APMTracker};
use anyhow::Result;
use core::utils;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use strategy::{create_strategy, CaptureMethod, CaptureStrategy};
use tokio::fs;
use tokio::sync::RwLock;

pub(crate) mod core;
pub mod strategy;
mod windows_graphics;

/// Windows Capture Recorder mit APM Tracking
pub struct WindowsCaptureRecorder {
    is_recording: Arc<RwLock<bool>>,
    strategy: Arc<RwLock<Box<dyn CaptureStrategy>>>,
    input_hook: InputHook,
    apm_tracker: Arc<Mutex<APMTracker>>,
}

impl WindowsCaptureRecorder {
    pub fn new(method: CaptureMethod) -> Self {
        let apm_tracker = Arc::new(Mutex::new(APMTracker::new()));

        Self {
            is_recording: Arc::new(RwLock::new(false)),
            strategy: Arc::new(RwLock::new(create_strategy(method))),
            input_hook: InputHook::new(apm_tracker.clone()),
            apm_tracker,
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

    pub async fn start_recording(
        &self,
        window_title: &str,
        game_state: Option<GameState>,
    ) -> Result<()> {
        if *self.is_recording.read().await {
            return Err(anyhow::anyhow!("Bereits am Aufnehmen"));
        }

        let metadata = RecordingMetadata::with_game_state(
            utils::extract_game_name(window_title),
            game_state.as_ref().and_then(|s| s.character_name.clone()),
            game_state.as_ref().and_then(|s| s.map_name.clone()),
            game_state.as_ref().and_then(|s| s.round_number),
        );

        let filename = metadata.generate_filename();
        let output_path = self.get_output_path(&filename).await?;

        log!("🎬 Starting recording: {}", filename);

        let mut strategy = self.strategy.write().await;
        strategy
            .start_capture(window_title, metadata, output_path)
            .await?;

        *self.is_recording.write().await = true;

        self.apm_tracker.lock().start_recording();
        self.input_hook.start();

        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<()> {
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

        log!("✅ Recording saved to: {}", output_path.display());

        let mut dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("No local data dir found"))?;

        dir.push("Kaptik");
        dir.push("recordings");

        fs::create_dir_all(&dir).await?;

        let apm_file_path: PathBuf = dir.join(format!("{}.apm", metadata.recording_id));

        self.save_apm_data(&apm_file_path).await?;

        *self.is_recording.write().await = false;

        Ok(())
    }

    async fn save_apm_data(&self, apm_path: &PathBuf) -> Result<()> {
        let series = self
            .apm_tracker
            .lock()
            .compute_apm_series(20.0, 1.0, true);

        if series.is_empty() {
            log!("No APM data recorded");
            return Ok(());
        }

        let apm_data = APMData { series };
        let apm_path = apm_path.clone();

        tokio::task::spawn_blocking(move || {
            if let Err(e) = save_apm_msgpack(&apm_data, &apm_path) {
                log!("Failed to save APM data: {}", e);
            } else {
                log!("APM data saved to: {}", apm_path.display());
            }
        });

        Ok(())
    }

    pub async fn get_current_metadata(&self) -> Option<RecordingMetadata> {
        let strategy = self.strategy.read().await;
        strategy.get_metadata().cloned()
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
