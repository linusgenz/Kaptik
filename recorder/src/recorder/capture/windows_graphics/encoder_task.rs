use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, watch};

use crate::log;
use crate::recorder::capture::core::ffmpeg::FfmpegEncoder;

use super::frame::CapturedFrame;

pub(crate) fn spawn_encoder_task(
    encoder: Arc<Mutex<Option<FfmpegEncoder>>>,
    mut frame_rx: mpsc::UnboundedReceiver<CapturedFrame>,
    shutdown_rx: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        loop {
            if *shutdown_rx.borrow() {
                log!("🛑 Encoder shutdown signal received");
                break;
            }

            match frame_rx.try_recv() {
                Ok(frame) => {
                    let mut enc_lock = encoder.lock().unwrap();
                    if let Some(enc) = enc_lock.as_mut() {
                        if let Err(e) = enc.encode_frame(&frame.texture) {
                            log!("⚠️ Error encoding: {}", e);
                            break;
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    log!("📹 Frame channel closed");
                    break;
                }
            }
        }

        log!("📹 Encoder task beendet");
    })
}
