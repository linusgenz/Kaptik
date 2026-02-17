use std::sync::Arc;

use tokio::sync::mpsc;

use crate::game_detection::{GameDetector, GameEvent};
use crate::game_integration::manager::GameIntegrationManager;
use crate::log;
use crate::recorder::RecorderEvent;

pub fn spawn(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    integration_manager: Arc<GameIntegrationManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log!("🎮 Game Detection Thread started");

        let mut detector = GameDetector::new();

        let game_event_tx = event_tx.clone();
        let integration_mgr = integration_manager.clone();

        detector.set_callback(move |event| match &event {
            GameEvent::GameStarted(game) => {
                log!(
                    "🎮 Game detected: {} (PID: {}) - {}",
                    game.name,
                    game.pid,
                    game.window_title
                );

                let mgr = integration_mgr.clone();
                let game_name = game.name.clone();
                tokio::spawn(async move {
                    let _ = mgr.activate_for_game(&game_name).await;
                });

                let _ = game_event_tx.send(RecorderEvent::GameDetected(game.clone()));
            }
            GameEvent::GameStopped(name) => {
                log!("🛑 Game closed: {}", name);
                let mgr = integration_mgr.clone();
                tokio::spawn(async move {
                    mgr.deactivate().await;
                });
                let _ = game_event_tx.send(RecorderEvent::GameStopped(name.clone()));
            }
            GameEvent::GameFocused(name) => {
                log!("👁️ Game focused: {}", name);
            }
            GameEvent::GameUnfocused(name) => {
                log!("💤 Game in the background: {}", name);
            }
        });

        detector.start_monitoring().await;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    })
}
