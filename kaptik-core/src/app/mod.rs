mod event_handler;
mod game_detection_task;
mod ipc_server;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::game_integration::manager::GameIntegrationManager;
use crate::log;
use crate::recorder::capture::strategy::CaptureMethod;
use crate::recorder::capture::WindowsCaptureRecorder;
use crate::recorder::{RecorderEvent, RecordingState};
use crate::tray::run_tray;

pub async fn run() -> anyhow::Result<()> {
    log!("🎬 Kaptik Recorder started");

    let (event_tx, event_rx) = mpsc::unbounded_channel::<RecorderEvent>();

    let event_tx_clone = event_tx.clone();

    let integration_manager = Arc::new(GameIntegrationManager::new());
    integration_manager.start_monitoring().await;

    let recorder = Arc::new(WindowsCaptureRecorder::new(
        CaptureMethod::WindowsGraphicsCapture,
    ));

    let recording_state = Arc::new(tokio::sync::RwLock::new(RecordingState::default()));

    game_detection_task::spawn(event_tx.clone(), integration_manager.clone());
    event_handler::spawn(
        event_rx,
        recording_state.clone(),
        recorder.clone(),
        integration_manager.clone(),
    );
    ipc_server::spawn(event_tx, recording_state);

    log!("✅ Recorder running");

    run_tray(event_tx_clone).ok();

    log!("👋 Recorder shutdown complete");
    Ok(())
}

