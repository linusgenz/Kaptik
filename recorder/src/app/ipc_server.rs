use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{RwLock, mpsc};

use crate::ipc;
use crate::log;
use crate::recorder::{RecorderEvent, RecordingState};
use crate::settings;

pub fn spawn(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    state: Arc<RwLock<RecordingState>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log!("🔌 IPC server thread started");

        if let Err(e) = run_ipc_server(event_tx, state).await {
            log!("❌ IPC Server Fehler: {}", e);
        }
    })
}

async fn run_ipc_server(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    _state: Arc<RwLock<RecordingState>>,
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

        if let Err(e) = handle_client(server, event_tx.clone()).await {
            log!("Client Handler error: {}", e);
        }

        log!("❌ App disconnected, waiting for new connection...");
    }
}

async fn handle_client(
    mut server: NamedPipeServer,
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
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

        if server.read_exact(&mut data).await.is_err() {
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
                let _ = event_tx.send(RecorderEvent::StartRecording(None));
            }
            ipc::CommandType::StopRecording => {
                let _ = event_tx.send(RecorderEvent::StopRecording);
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
