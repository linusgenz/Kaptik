use super::RecordingMetadata;
use crate::game_integration::GameState;
use crate::log;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

mod capture;
mod d3d;
mod encoder;
pub(crate) mod tonemap;
mod utils;

use capture::CaptureSession;

pub struct WindowsCaptureRecorder {
    is_recording: Arc<RwLock<bool>>,
    current_session: Arc<RwLock<Option<CaptureSession>>>,
}

impl WindowsCaptureRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(RwLock::new(false)),
            current_session: Arc::new(RwLock::new(None)),
        }
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
        log!("🎬 Starte Windows.Graphics.Capture Recording: {}", filename);

        // Determine output path
        let output_path = self.get_output_path(&filename).await?;

        // Create capture session
        let session = CaptureSession::create(window_title, metadata, output_path).await?;

        // Save session and update state
        *self.is_recording.write().await = true;
        *self.current_session.write().await = Some(session);

        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<()> {
        if !*self.is_recording.read().await {
            return Ok(());
        }

        let session = {
            let mut current = self.current_session.write().await;
            current.take()
        };

        if let Some(session) = session {
            let output_path = session.output_path().clone();
            session.stop().await?;
            log!("✅ Capture saved under: {}", output_path.display());
        }

        *self.is_recording.write().await = false;
        Ok(())
    }

    pub async fn get_current_metadata(&self) -> Option<RecordingMetadata> {
        let session = self.current_session.read().await;
        session.as_ref().map(|s| s.metadata().clone())
    }

    async fn get_output_path(&self, filename: &str) -> Result<PathBuf> {
        use crate::settings::SETTINGS;

        let settings = SETTINGS.read().await;
        let mut output_path = PathBuf::from(&settings.video_path);

        if output_path.as_os_str().is_empty() {
            output_path = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));
            output_path.push("Kaptik");
        }

        std::fs::create_dir_all(&output_path)?;
        output_path.push(filename);

        Ok(output_path)
    }
}