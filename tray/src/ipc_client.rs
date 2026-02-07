use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};
use tokio::net::windows::named_pipe::ClientOptions;
use crate::ipc::Command;

const PIPE_NAME: &str = r"\\.\pipe\kaptik_control_pipe";

pub async fn send_command(cmd: &Command) -> anyhow::Result<()> {
    let mut retries = 5;

    loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(mut pipe) => {
                let payload = rmp_serde::to_vec(cmd)?;
                let len = payload.len() as u32;

                pipe.write_all(&len.to_le_bytes()).await?;
                pipe.write_all(&payload).await?;
                pipe.flush().await?;
                return Ok(());
            }
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    anyhow::bail!("Failed to connect to recorder pipe: {}", e);
                }
                sleep(Duration::from_millis(200)).await;
            }
        }
    }
}
