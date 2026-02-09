use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{RwLock, mpsc, watch, Mutex};

use crate::ipc;
use crate::ipc::CommandType;
use crate::log;
use crate::recorder::{RecorderEvent, RecordingState};
use crate::settings;

pub fn spawn(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    state: Arc<RwLock<RecordingState>>,
    shutdown_tx: watch::Sender<bool>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    let (shutdown_notification_tx, shutdown_notification_rx) = mpsc::unbounded_channel::<()>();
    let shutdown_notification_rx = Arc::new(Mutex::new(shutdown_notification_rx));

    {
        let event_tx = event_tx.clone();
        let state = state.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let shutdown_tx = shutdown_tx.clone();
        let shutdown_notification_rx = shutdown_notification_rx.clone();

        handles.push(tokio::spawn(async move {
            log!("🔌 IPC server thread started");

            if let Err(e) = run_main_ipc_server(
                event_tx,
                state,
                shutdown_tx,
                shutdown_rx,
                shutdown_notification_rx,
                r"\\.\pipe\kaptik_pipe",
            )
                .await
            {
                log!("❌ IPC server error: {}", e);
            }
        }));
    }

    {
        let event_tx = event_tx.clone();
        let state = Arc::new(RwLock::new(RecordingState::default()));
        let shutdown_rx = shutdown_tx.subscribe();

        handles.push(tokio::spawn(async move {
            log!("🔌 IPC control server thread started");

            if let Err(e) = run_control_ipc_server(
                event_tx,
                state,
                shutdown_rx,
                shutdown_notification_tx,
                r"\\.\pipe\kaptik_control_pipe",
            )
                .await
            {
                log!("❌ IPC control server error: {}", e);
            }
        }));
    }

    handles
}

async fn run_main_ipc_server(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    _state: Arc<RwLock<RecordingState>>,
    shutdown_tx: watch::Sender<bool>,
    mut shutdown_rx: watch::Receiver<bool>,
    shutdown_notification_rx: Arc<Mutex<mpsc::UnboundedReceiver<()>>>,
    pipe_name: &str,
) -> anyhow::Result<()> {
    let mut first_instance = true;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                log!("🛑 IPC server shutting down ({})", pipe_name);
                break;
            }

            result = async {
                let server = if first_instance {
                    ServerOptions::new().create(pipe_name)?
                } else {
                    ServerOptions::new()
                        .first_pipe_instance(false)
                        .create(pipe_name)?
                };

                first_instance = false;

                log!("⏳ Waiting for connection on {}...", pipe_name);
                server.connect().await?;
                log!("✅ Client connected to {}!", pipe_name);

                handle_main_client(
                    server,
                    event_tx.clone(),
                    shutdown_tx.clone(),
                    shutdown_notification_rx.clone(),
                ).await
            } => {
                match result {
                    Err(e) if e.to_string().contains("Shutdown") => {
                        log!("🛑 Main pipe shutting down");
                        break;
                    }
                    Err(e) => {
                        log!("Client handler error: {}", e);
                    }
                    Ok(_) => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_control_ipc_server(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    _state: Arc<RwLock<RecordingState>>,
    mut shutdown_rx: watch::Receiver<bool>,
    shutdown_notification_tx: mpsc::UnboundedSender<()>,
    pipe_name: &str,
) -> anyhow::Result<()> {
    let mut first_instance = true;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                log!("🛑 IPC control server shutting down ({})", pipe_name);
                break;
            }

            result = async {
                let server = if first_instance {
                    ServerOptions::new().create(pipe_name)?
                } else {
                    ServerOptions::new()
                        .first_pipe_instance(false)
                        .create(pipe_name)?
                };

                first_instance = false;

                log!("⏳ Waiting for connection on {}...", pipe_name);
                server.connect().await?;
                log!("✅ Client connected to {}!", pipe_name);

                handle_control_client(
                    server,
                    event_tx.clone(),
                    shutdown_notification_tx.clone(),
                ).await
            } => {
                match result {
                    Err(e) if e.to_string().contains("Shutdown requested") => {
                        log!("🛑 Shutdown via client command ({})", pipe_name);
                        break;
                    }
                    Err(e) => {
                        log!("Control client handler error: {}", e);
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    Ok(())
}

async fn handle_main_client(
    mut server: NamedPipeServer,
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_notification_rx: Arc<Mutex<mpsc::UnboundedReceiver<()>>>,
) -> anyhow::Result<()> {
    let mut len_buf = [0u8; 4];

    loop {
        let mut shutdown_rx_guard = shutdown_notification_rx.lock().await;
        tokio::select! {
            result = shutdown_rx_guard.recv() => {
                if result.is_some() {
                    log!("📨 Shutdown notification received in main client handler");

                    let shutdown_cmd = ipc::Command {
                        type_: CommandType::ShutdownUI,
                        update: None,
                    };

                    if let Ok(payload) = rmp_serde::to_vec_named(&shutdown_cmd) {
                        if let Err(e) = server.write_all(&(payload.len() as u32).to_le_bytes()).await {
                            log!("❌ Failed to write length: {}", e);
                        } else if let Err(e) = server.write_all(&payload).await {
                            log!("❌ Failed to write payload: {}", e);
                        } else if let Err(e) = server.flush().await {
                            log!("❌ Failed to flush: {}", e);
                        } else {
                            log!("✅ ShutdownUI command sent to companion app");

                            log!("⏳ Waiting for client to disconnect...");

                            let disconnect_timeout = tokio::time::sleep(std::time::Duration::from_secs(2));
                            tokio::pin!(disconnect_timeout);

                            tokio::select! {
                                read_result = server.read_exact(&mut len_buf) => {
                                    if read_result.is_err() {
                                        log!("✅ Client disconnected cleanly");
                                    } else {
                                        log!("⚠️ Client sent unexpected data during shutdown");
                                    }
                                }
                                // Oder Timeout
                                _ = &mut disconnect_timeout => {
                                    log!("⏱️ Client disconnect timeout reached");
                                }
                            }
                        }
                    }

                    // Jetzt Recorder herunterfahren
                    let _ = shutdown_tx.send(true);
                    return Err(anyhow::anyhow!("Shutdown"));
                }
            }

            // Normal client messages
            result = server.read_exact(&mut len_buf) => {
                if result.is_err() {
                    log!("Main client disconnected");
                    break;
                }

                let len = u32::from_le_bytes(len_buf) as usize;
                let mut data = vec![0u8; len];

                if server.read_exact(&mut data).await.is_err() {
                    log!("Read error, main client disconnected");
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
                    CommandType::StartRecording => {
                        let _ = event_tx.send(RecorderEvent::StartRecording(None));
                    }

                    CommandType::StopRecording => {
                        let _ = event_tx.send(RecorderEvent::StopRecording);
                    }

                    CommandType::UpdateSetting => {
                        if let Some(u) = cmd.update {
                            log!("⚙️ Update setting: {}={}", u.key, u.value);
                            settings::update_setting(|s| {
                                if let Err(e) = s.update(&u.key, &u.value) {
                                    log!("Failed to update setting: {}", e);
                                }
                            })
                                .await;
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

async fn handle_control_client(
    mut server: NamedPipeServer,
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    shutdown_notification_tx: mpsc::UnboundedSender<()>,
) -> anyhow::Result<()> {
    let mut len_buf = [0u8; 4];

    loop {
        if server.read_exact(&mut len_buf).await.is_err() {
            log!("Control client disconnected");
            break;
        }

        let len = u32::from_le_bytes(len_buf) as usize;
        let mut data = vec![0u8; len];

        if server.read_exact(&mut data).await.is_err() {
            log!("Read error, control client disconnected");
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
            CommandType::Shutdown => {
                log!("🛑 Shutdown command received from control pipe");

                let _ = event_tx.send(RecorderEvent::StopRecording);

                let _ = shutdown_notification_tx.send(());

                return Err(anyhow::anyhow!("Shutdown requested"));
            }

            _ => {}
        }
    }

    Ok(())
}