use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use super::{CaptureStrategy, RecordingMetadata};
use crate::recorder::capture::windows_graphics::CaptureSession;

pub struct WindowsGraphicsCaptureStrategy {
    session: Option<CaptureSession>,
}

impl WindowsGraphicsCaptureStrategy {
    pub fn new() -> Self {
        Self { session: None }
    }
}

#[async_trait(?Send)]
impl CaptureStrategy for WindowsGraphicsCaptureStrategy {
    async fn start_capture(
        &mut self,
        window_title: &str,
        metadata: RecordingMetadata,
        output_path: PathBuf,
    ) -> Result<()> {
        let session = CaptureSession::create(window_title, metadata, output_path).await?;
        self.session = Some(session);

        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<PathBuf> {
        if let Some(session) = self.session.take() {
            let output_path = session.output_path().clone();
            session.stop().await?;

            Ok(output_path)
        } else {
            Err(anyhow::anyhow!("No active session"))
        }
    }

    fn is_active(&self) -> bool {
        self.session.is_some()
    }

    fn get_metadata(&self) -> Option<&RecordingMetadata> {
        self.session.as_ref().map(|s| s.metadata())
    }
}
