use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::RecordingMetadata;

mod windows_graphics;

pub use windows_graphics::WindowsGraphicsCaptureStrategy;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CaptureMethod {
    #[serde(rename = "windows_graphics")]
    WindowsGraphicsCapture,
}

impl Default for CaptureMethod {
    fn default() -> Self {
        Self::WindowsGraphicsCapture
    }
}

#[async_trait(?Send)]
pub trait CaptureStrategy: Send + Sync {
    /// Startet die Aufnahme
    async fn start_capture(
        &mut self,
        window_title: &str,
        metadata: RecordingMetadata,
        output_path: PathBuf,
    ) -> Result<()>;

    async fn stop_capture(&mut self) -> Result<PathBuf>;

    fn is_active(&self) -> bool;

    fn get_metadata(&self) -> Option<&RecordingMetadata>;
}

pub fn create_strategy(method: CaptureMethod) -> Box<dyn CaptureStrategy> {
    match method {
        CaptureMethod::WindowsGraphicsCapture => {
            Box::new(WindowsGraphicsCaptureStrategy::new())
        }
    }
}