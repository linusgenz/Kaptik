use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use core::utils;
use crate::game_integration::GameState;
use crate::log;

use strategy::{create_strategy, CaptureMethod, CaptureStrategy};
use super::RecordingMetadata;

pub(crate) mod core;
pub mod strategy;
mod win_recorder;
mod audio_mixer;

pub struct WindowsCaptureRecorder {
    is_recording: Arc<RwLock<bool>>,
    strategy: Arc<RwLock<Box<dyn CaptureStrategy>>>,
}

impl WindowsCaptureRecorder {
    pub fn new(method: CaptureMethod) -> Self {
        Self {
            is_recording: Arc::new(RwLock::new(false)),
            strategy: Arc::new(RwLock::new(create_strategy(method))),
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

        let metadata = RecordingMetadata {
            game_name: utils::extract_game_name(window_title),
            character_name: game_state.as_ref().and_then(|s| s.character_name.clone()),
            map_name: game_state.as_ref().and_then(|s| s.map_name.clone()),
            round_number: game_state.as_ref().and_then(|s| s.round_number),
            timestamp: chrono::Local::now(),
        };

        let filename = metadata.generate_filename();
        let output_path = self.get_output_path(&filename).await?;

        log!("🎬 Starting recording: {}", filename);

        // Use selected strategy
        let mut strategy = self.strategy.write().await;
        strategy.start_capture(window_title, metadata, output_path).await?;

        *self.is_recording.write().await = true;
        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<()> {
        if !*self.is_recording.read().await {
            return Ok(());
        }

        let mut strategy = self.strategy.write().await;
        let output_path = strategy.stop_capture().await?;

        log!("✅ Recording saved to: {}", output_path.display());
        *self.is_recording.write().await = false;

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