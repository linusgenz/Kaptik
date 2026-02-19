// event_handler.rs
//! Bridges the recorder event bus with the game integration manager.
//!
//! All mutations to [`GameTracker`] happen here; both the recorder and the
//! integration manager are treated as pure-async services.

use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use crate::domain::game::{GameName, GameState};
use crate::game_detection::GameProcess;
use crate::game_integration::manager::GameIntegrationManager;
use crate::log;
use crate::recorder::capture::WindowsCaptureRecorder;
use crate::recorder::{GameTracker, RecorderEvent};
use crate::settings;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

pub fn spawn(
    mut event_rx: mpsc::UnboundedReceiver<RecorderEvent>,
    game_tracker: Arc<RwLock<GameTracker>>,
    recorder: Arc<WindowsCaptureRecorder>,
    integration_manager: Arc<GameIntegrationManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log!("📋 Event handler started");

        wire_event_forwarding(&recorder, &integration_manager).await;

        while let Some(event) = event_rx.recv().await {
            handle_event(
                event,
                &game_tracker,
                &recorder,
                &integration_manager,
            )
                .await;
        }

        log!("📋 Event handler stopped (channel closed)");
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Setup
// ─────────────────────────────────────────────────────────────────────────────

async fn wire_event_forwarding(
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    let rec = recorder.clone();
    manager
        .set_event_callback(move |event| {
            let rec = rec.clone();
            tokio::spawn(async move {
                rec.add_event(event).await;
            });
        })
        .await;
    log!("✅ Game-event → recorder forwarding active");
}

async fn handle_event(
    event: RecorderEvent,
    tracker: &Arc<RwLock<GameTracker>>,
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    match event {
        RecorderEvent::GameDetected(game) => {
            on_game_detected(game, tracker, recorder, manager).await;
        }
        RecorderEvent::GameStopped(name) => {
            on_game_stopped(name, tracker, recorder, manager).await;
        }
        RecorderEvent::StartRecording => {
            on_start_recording_manual(tracker, recorder, manager).await;
        }
        RecorderEvent::StopRecording => {
            on_stop_recording_manual(recorder, manager).await;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn on_game_detected(
    game: GameProcess,
    tracker: &Arc<RwLock<GameTracker>>,
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    tracker.write().await.active_games.push(game.clone());

    let auto_record = settings::get_setting(|s| s.auto_record).await;

    // Recorder is the single source of truth for recording state.
    if !auto_record || recorder.is_recording().await {
        return;
    }

    log!("🔍 Auto-record: waiting for round to start…");

    // Optimistically claim the game name now so the spawned task is 'static.
    let game_name = match manager.active_game_identifier().await {
        Some(id) => id.to_game_name(),
        None => GameName::from_window_title(&game.window_title),
    };
    let game_name_fallback = GameName::from_window_title(&game.window_title);
    let game_name_str = game.name.clone();
    let window = game.window_title.clone();

    let rec     = recorder.clone();
    let mgr     = manager.clone();
    let tracker_c = tracker.clone();

    tokio::spawn(async move {
        const MAX_WAIT_SECS: u32 = 30;
        loop {
       // for attempt in 1..=MAX_WAIT_SECS {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Another recording may have started in the meantime (e.g. manual).
            if rec.is_recording().await {
                log!("↩️  Auto-record cancelled – recording already active");
                return;
            }

            if mgr.is_in_round().await {
                log!("🟢 Round detected – starting recording");//, attempt);
                let game_state = mgr.get_current_state().await;
                let started = start_recording_blocking(&rec, &window, game_name, game_state).await;
                if started {
                    tracker_c.write().await.current_game = Some(game_name_str);
                }
                return;
            }
        }

     /*   log!(
            "⚠️  No round detected after {}s – starting recording anyway",
            MAX_WAIT_SECS
        );
        let started = start_recording_blocking(&rec, &window, game_name_fallback, None).await;
        if started {
            tracker_c.write().await.current_game = Some(game_name_str);
        }*/
    });
}

async fn on_game_stopped(
    name: String,
    tracker: &Arc<RwLock<GameTracker>>,
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    let was_recording_this_game = {
        let mut t = tracker.write().await;
        t.active_games.retain(|g| g.name != name);
        // Only stop if we were recording exactly this game.
        recorder.is_recording().await && t.current_game.as_deref() == Some(&name)
    };

    if !was_recording_this_game {
        return;
    }

    log!("🛑 Game '{}' stopped – stopping recording", name);

    let final_state = resolve_final_state(manager).await;
    stop_recording_blocking(recorder, final_state).await;

    tracker.write().await.current_game = None;
}

async fn on_start_recording_manual(
    tracker: &Arc<RwLock<GameTracker>>,
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    log!("▶️  Manual start recording");

    if recorder.is_recording().await {
        log!("⚠️  Already recording – ignoring manual start");
        return;
    }

    let maybe_game = tracker.read().await.first_active_game().cloned();

    let Some(game) = maybe_game else {
        log!("⚠️  No active game detected – cannot start recording");
        return;
    };

    let game_name = match manager.active_game_identifier().await {
        Some(id) => id.to_game_name(),
        None => GameName::from_window_title(&game.window_title),
    };

    let game_state = manager.get_current_state().await;
    let started = start_recording_blocking(recorder, &game.window_title, game_name, game_state).await;

    if started {
        tracker.write().await.current_game = Some(game.name);
    }
}

async fn on_stop_recording_manual(
    recorder: &Arc<WindowsCaptureRecorder>,
    manager: &Arc<GameIntegrationManager>,
) {
    log!("⏹️  Manual stop recording");

    if !recorder.is_recording().await {
        log!("⚠️  No active recording – ignoring manual stop");
        return;
    }

    let final_state = resolve_final_state(manager).await;
    stop_recording_blocking(recorder, final_state).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Determine the best available [`GameState`] to attach to a finished
/// recording – prefer a live state with KDA, fall back to the cached one.
async fn resolve_final_state(
    manager: &Arc<GameIntegrationManager>,
) -> Option<GameState> {
    match manager.get_current_state().await {
        Some(state) if state.kda.is_some() => Some(state),
        _ => manager.get_last_known_state().await,
    }
}

/// Run `recorder.start_recording` on a blocking thread (the underlying
/// Windows capture API is not `Send`-safe on the async executor).
///
/// Returns `true` on success.
async fn start_recording_blocking(
    recorder: &Arc<WindowsCaptureRecorder>,
    window_title: &str,
    game_name: GameName,
    game_state: Option<GameState>,
) -> bool {
    let rec   = recorder.clone();
    let title = window_title.to_owned();

    let result = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current()
            .block_on(async move { rec.start_recording(&title, game_name, game_state).await })
    })
        .await;

    match result {
        Ok(Ok(id)) => {
            log!("✅ Recording started – ID: {}", id);
            true
        }
        Ok(Err(e)) => {
            log!("❌ Recording start failed: {}", e);
            false
        }
        Err(e) => {
            log!("❌ spawn_blocking error: {}", e);
            false
        }
    }
}

/// Run `recorder.stop_recording` on a blocking thread.
async fn stop_recording_blocking(
    recorder: &Arc<WindowsCaptureRecorder>,
    final_state: Option<GameState>,
) {
    let rec = recorder.clone();

    let result = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current()
            .block_on(async move { rec.stop_recording(final_state).await })
    })
        .await;

    match result {
        Ok(Ok(())) => log!("✅ Recording stopped"),
        Ok(Err(e)) => log!("❌ Recording stop failed: {}", e),
        Err(e)     => log!("❌ spawn_blocking error: {}", e),
    }
}