use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::game_integration::manager::GameIntegrationManager;
use crate::log;
use crate::recorder::capture::WindowsCaptureRecorder;
use crate::recorder::{RecorderEvent, RecordingState};
use crate::settings;

pub fn spawn(
    mut event_rx: mpsc::UnboundedReceiver<RecorderEvent>,
    recording_state: Arc<RwLock<RecordingState>>,
    recorder: Arc<WindowsCaptureRecorder>,
    integration_manager: Arc<GameIntegrationManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log!("📋 Event Handler Thread started");

        while let Some(event) = event_rx.recv().await {
            let mut state = recording_state.write().await;

            match event {
                RecorderEvent::GameDetected(game) => {
                    state.active_games.push(game.clone());

                    let auto_record = settings::get_setting(|s| s.auto_record).await;

                    if auto_record && !state.is_recording {
                        log!("Auto-Record activated, waiting for round...");

                        let rec = recorder.clone();
                        let mgr = integration_manager.clone();
                        let window_title = game.window_title.clone();

                        drop(state);

                        tokio::spawn(async move {
                            let mut attempts = 0;
                            let max_attempts = 30;

                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                attempts += 1;

                                if mgr.is_in_round().await {
                                    let game_state = mgr.get_current_state().await;
                                    log!("Round detected! Start recording...");

                                    // Isolate the !Send operation
                                    let window_title_clone = window_title.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        tokio::runtime::Handle::current().block_on(async {
                                            rec.start_recording(&window_title_clone, game_state)
                                                .await
                                        })
                                    })
                                    .await;

                                    match result {
                                        Ok(Ok(())) => {
                                            log!("✅ Recording started");
                                        }
                                        Ok(Err(e)) => {
                                            log!("❌ Recording start error: {}", e);
                                        }
                                        Err(e) => {
                                            log!("❌ Task error: {}", e);
                                        }
                                    }
                                    break;
                                }

                                if attempts >= max_attempts {
                                    log!(
                                        "⚠️ No round detected after {} seconds, start recording anyway",
                                        max_attempts
                                    );

                                    let window_title_clone = window_title.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        tokio::runtime::Handle::current().block_on(async {
                                            rec.start_recording(&window_title_clone, None).await
                                        })
                                    })
                                    .await;

                                    match result {
                                        Ok(Ok(())) => {
                                            log!("✅ Recording started");
                                        }
                                        Ok(Err(e)) => {
                                            log!("❌ Recording start error: {}", e);
                                        }
                                        Err(e) => {
                                            log!("❌ Task error: {}", e);
                                        }
                                    }
                                    break;
                                }
                            }
                        });

                        // Re-acquire lock to update state
                        let mut state = recording_state.write().await;
                        state.is_recording = true;
                        state.current_game = Some(game.name.clone());
                    }
                }

                RecorderEvent::GameStopped(name) => {
                    state.active_games.retain(|g| g.name != name);

                    if state.is_recording && state.current_game.as_ref() == Some(&name) {
                        log!("⏹️ Game over, stop recording");

                        let rec = recorder.clone();
                        drop(state); // Release lock before blocking operation

                        // Isolate !Send operation
                        let result = tokio::task::spawn_blocking(move || {
                            tokio::runtime::Handle::current()
                                .block_on(async { rec.stop_recording().await })
                        })
                        .await;

                        match result {
                            Ok(Ok(())) => {
                                log!("✅ Recording stopped");
                            }
                            Ok(Err(e)) => {
                                log!("❌ Recording stop error: {}", e);
                            }
                            Err(e) => {
                                log!("❌ Task error: {}", e);
                            }
                        }

                        // Re-acquire lock
                        let mut state = recording_state.write().await;
                        state.is_recording = false;
                        state.current_game = None;
                    }
                }

                RecorderEvent::StartRecording(_requested_game_name) => {
                    log!("▶️ Manual start recording");

                    if state.is_recording {
                        log!("⚠️ Already recording");
                        continue;
                    }

                    if let Some(game) = state.active_games.first() {
                        let window_title = game.window_title.clone();
                        let game_name = game.name.clone();
                        let rec = recorder.clone();
                        let mgr = integration_manager.clone();

                        drop(state); // Release lock

                        // Get game state
                        let game_state = mgr.get_current_state().await;

                        // Isolate !Send operation
                        let result = tokio::task::spawn_blocking(move || {
                            tokio::runtime::Handle::current().block_on(async {
                                rec.start_recording(&window_title, game_state).await
                            })
                        })
                        .await;

                        match result {
                            Ok(Ok(())) => {
                                log!("✅ Recording started");
                                let mut state = recording_state.write().await;
                                state.is_recording = true;
                                state.current_game = Some(game_name);
                            }
                            Ok(Err(e)) => {
                                log!("❌ Recording start error: {}", e);
                            }
                            Err(e) => {
                                log!("❌ Task error: {}", e);
                            }
                        }
                    } else {
                        log!("⚠️ No active game found");
                    }
                }

                RecorderEvent::StopRecording => {
                    log!("⏹️ Manual Stop Recording");

                    if !state.is_recording {
                        log!("⚠️ No active recording");
                        continue;
                    }

                    let rec = recorder.clone();
                    drop(state); // Release lock

                    // Isolate !Send operation
                    let result = tokio::task::spawn_blocking(move || {
                        tokio::runtime::Handle::current()
                            .block_on(async { rec.stop_recording().await })
                    })
                    .await;

                    match result {
                        Ok(Ok(())) => {
                            log!("✅ Recording stopped");
                        }
                        Ok(Err(e)) => {
                            log!("❌ Recording stop error: {}", e);
                        }
                        Err(e) => {
                            log!("❌ Task error: {}", e);
                        }
                    }

                    let mut state = recording_state.write().await;
                    state.is_recording = false;
                    state.current_game = None;
                }
            }
        }
    })
}
