use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc;
mod game_detection;
mod ipc;
mod settings;
mod game_integration;
mod recorder;
mod logger;

use game_integration::manager::GameIntegrationManager;
use crate::game_detection::{GameDetector, GameEvent};
use crate::recorder::win_recorder::WindowsCaptureRecorder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log!("🎬 Kaptik Recorder started");

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<recorder::RecorderEvent>();

    let integration_manager = Arc::new(GameIntegrationManager::new());
    integration_manager.start_monitoring().await;

    let recorder = Arc::new(WindowsCaptureRecorder::new());

    let game_event_tx = event_tx.clone();
    let integration_mgr = integration_manager.clone();

    tokio::spawn(async move {
        log!("🎮 Game Detection Thread started");

        let mut detector = GameDetector::new();

        detector.set_callback(move |event| match &event {
            GameEvent::GameStarted(game) => {
                log!("🎮 Game detected: {} (PID: {}) - {}", game.name, game.pid, game.window_title);

                let mgr = integration_mgr.clone();
                let game_name = game.name.clone();
                tokio::spawn(async move {
                    let _ = mgr.activate_for_game(&game_name).await;
                });

                let _ = game_event_tx.send(recorder::RecorderEvent::GameDetected(game.clone()));
            }
            GameEvent::GameStopped(name) => {
                log!("🛑 Game closed: {}", name);
                let mgr = integration_mgr.clone();
                tokio::spawn(async move {
                    mgr.stop_active_integration().await;
                });
                let _ = game_event_tx.send(recorder::RecorderEvent::GameStopped(name.clone()));
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
    });

    let recording_state = Arc::new(tokio::sync::RwLock::new(recorder::RecordingState::default()));
    let recording_state_clone = recording_state.clone();
    let recorder_clone = recorder.clone();
    let integration_mgr_handler = integration_manager.clone();

    tokio::spawn(async move {
        log!("📋 Event Handler Thread started");

        while let Some(event) = event_rx.recv().await {
            let mut state = recording_state_clone.write().await;

            match event {
                recorder::RecorderEvent::GameDetected(game) => {
                    state.active_games.push(game.clone());

                    let auto_record = settings::get_setting(|s| s.auto_record).await;

                    if auto_record && !state.is_recording {
                        log!("🎯 Auto-Record aktiviert, warte auf Runde...");

                        let rec = recorder_clone.clone();
                        let mgr = integration_mgr_handler.clone();
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
                                    log!("▶️  Runde erkannt! Starte Recording...");

                                    // Isolate the !Send operation
                                    let window_title_clone = window_title.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        tokio::runtime::Handle::current().block_on(async {
                                            rec.start_recording(&window_title_clone, game_state).await
                                        })
                                    }).await;

                                    match result {
                                        Ok(Ok(())) => log!("✅ Recording gestartet"),
                                        Ok(Err(e)) => log!("❌ Recording Start Fehler: {}", e),
                                        Err(e) => log!("❌ Task Fehler: {}", e),
                                    }
                                    break;
                                }

                                if attempts >= max_attempts {
                                    log!("⚠️ Keine Runde erkannt nach {}s, starte Recording trotzdem", max_attempts);

                                    let window_title_clone = window_title.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        tokio::runtime::Handle::current().block_on(async {
                                            rec.start_recording(&window_title_clone, None).await
                                        })
                                    }).await;

                                    match result {
                                        Ok(Ok(())) => log!("✅ Recording gestartet"),
                                        Ok(Err(e)) => log!("❌ Recording Start Fehler: {}", e),
                                        Err(e) => log!("❌ Task Fehler: {}", e),
                                    }
                                    break;
                                }
                            }
                        });

                        // Re-acquire lock to update state
                        let mut state = recording_state_clone.write().await;
                        state.is_recording = true;
                        state.current_game = Some(game.name.clone());
                    } else {
                        // Normal path - keep lock
                    }
                }

                recorder::RecorderEvent::GameStopped(name) => {
                    state.active_games.retain(|g| g.name != name);

                    if state.is_recording && state.current_game.as_ref() == Some(&name) {
                        log!("⏹️  Game beendet, stoppe Recording");

                        let rec = recorder_clone.clone();
                        drop(state); // Release lock before blocking operation

                        // Isolate !Send operation
                        let result = tokio::task::spawn_blocking(move || {
                            tokio::runtime::Handle::current().block_on(async {
                                rec.stop_recording().await
                            })
                        }).await;

                        match result {
                            Ok(Ok(())) => log!("✅ Recording gestoppt"),
                            Ok(Err(e)) => log!("❌ Recording Stop Fehler: {}", e),
                            Err(e) => log!("❌ Task Fehler: {}", e),
                        }

                        // Re-acquire lock
                        let mut state = recording_state_clone.write().await;
                        state.is_recording = false;
                        state.current_game = None;
                    }
                }

                recorder::RecorderEvent::StartRecording(game_name) => {
                    log!("▶️  Manueller Start Recording");

                    if state.is_recording {
                        log!("⚠️ Bereits am Aufnehmen");
                        continue;
                    }

                    if let Some(game) = state.active_games.first() {
                        let window_title = game.window_title.clone();
                        let game_name = game.name.clone();
                        let rec = recorder_clone.clone();
                        let mgr = integration_mgr_handler.clone();

                        drop(state); // Release lock

                        // Get game state
                        let game_state = mgr.get_current_state().await;

                        // Isolate !Send operation
                        let result = tokio::task::spawn_blocking(move || {
                            tokio::runtime::Handle::current().block_on(async {
                                rec.start_recording(&window_title, game_state).await
                            })
                        }).await;

                        match result {
                            Ok(Ok(())) => {
                                log!("✅ Recording gestartet");
                                let mut state = recording_state_clone.write().await;
                                state.is_recording = true;
                                state.current_game = Some(game_name);
                            }
                            Ok(Err(e)) => log!("❌ Recording Start Fehler: {}", e),
                            Err(e) => log!("❌ Task Fehler: {}", e),
                        }
                    } else {
                        log!("⚠️  Kein aktives Game gefunden");
                    }
                }

                recorder::RecorderEvent::StopRecording => {
                    log!("⏹️  Manueller Stop Recording");

                    if !state.is_recording {
                        log!("⚠️  Kein aktives Recording");
                        continue;
                    }

                    let rec = recorder_clone.clone();
                    drop(state); // Release lock

                    // Isolate !Send operation
                    let result = tokio::task::spawn_blocking(move || {
                        tokio::runtime::Handle::current().block_on(async {
                            rec.stop_recording().await
                        })
                    }).await;

                    match result {
                        Ok(Ok(())) => log!("✅ Recording gestoppt"),
                        Ok(Err(e)) => log!("❌ Recording Stop Fehler: {}", e),
                        Err(e) => log!("❌ Task Fehler: {}", e),
                    }

                    let mut state = recording_state_clone.write().await;
                    state.is_recording = false;
                    state.current_game = None;
                }
            }
        }
    });

    let ipc_event_tx = event_tx.clone();
    let ipc_state = recording_state.clone();

    tokio::spawn(async move {
        log!("🔌 IPC server thread started");

        if let Err(e) = run_ipc_server(ipc_event_tx, ipc_state).await {
            log!("❌ IPC Server Fehler: {}", e);
        }
    });

    log!("✅ Recorder running");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

async fn run_ipc_server(
    event_tx: mpsc::UnboundedSender<recorder::RecorderEvent>,
    state: Arc<tokio::sync::RwLock<recorder::RecordingState>>,
) -> anyhow::Result<()> {
    let mut first_instance = true;

    loop {
        let mut server = if first_instance {
            ServerOptions::new().create(r"\\.\pipe\kaptik_pipe")?
        } else {
            ServerOptions::new()
                .first_pipe_instance(false)
                .create(r"\\.\pipe\kaptik_pipe")?
        };

        first_instance = false;

        log!("⏳ Waiting for app connection...");
        server.connect().await?;
        log!("✅ App connected!");

        if let Err(e) = handle_client(server, event_tx.clone(), state.clone()).await {
            log!("Client Handler error: {}", e);
        }

        log!("❌ App disconnected, waiting for new connection...");
    }
}

async fn handle_client(
    mut server: NamedPipeServer,
    event_tx: mpsc::UnboundedSender<recorder::RecorderEvent>,
    state: Arc<tokio::sync::RwLock<recorder::RecordingState>>,
) -> anyhow::Result<()> {
    let mut len_buf = [0u8; 4];

    loop {
        match server.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(_) => {
                log!("Client disconnected");
                break;
            }
        }

        let len = u32::from_le_bytes(len_buf) as usize;
        let mut data = vec![0u8; len];

        if let Err(_) = server.read_exact(&mut data).await {
            log!("Read error, client disconnected");
            break;
        }

        let cmd: ipc::Command = match rmp_serde::from_slice(&data) {
            Ok(c) => c,
            Err(e) => {
                log!("Deserialization error: {}", e);
                continue;
            }
        };

        match cmd.type_ {
            ipc::CommandType::StartRecording => {
                let _ = event_tx.send(recorder::RecorderEvent::StartRecording(None));
            }
            ipc::CommandType::StopRecording => {
                let _ = event_tx.send(recorder::RecorderEvent::StopRecording);
            }
            ipc::CommandType::UpdateSetting => {
                if let Some(u) = cmd.update {
                    log!("⚙️  Update setting: {}={}", u.key, u.value);

                    settings::update_setting(|s| {
                        if let Err(e) = s.update(&u.key, &u.value) {
                            log!("Failed to update setting: {}", e);
                        }
                    })
                    .await;
                }
            }
        }
    }

    Ok(())
}
