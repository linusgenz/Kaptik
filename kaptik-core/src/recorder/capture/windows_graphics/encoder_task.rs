use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant}; // ⭐ NEU

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

        let fps = {
            let enc_lock = encoder.lock().unwrap();
            enc_lock.as_ref().unwrap().fps()
        };

        let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
        let mut next_frame_time = Instant::now();

        log!("🎬 Encoder pacing enabled: {} FPS", fps);

        let mut last_frame: Option<CapturedFrame> = None;

        loop {
            if *shutdown_rx.borrow() {
                log!("🛑 Encoder shutdown signal received");
                break;
            }

            let now = Instant::now();

            if now < next_frame_time {
                std::thread::sleep(next_frame_time - now);
                continue;
            }

            next_frame_time += frame_duration;

            while let Ok(newer) = frame_rx.try_recv() {
                last_frame = Some(newer);
            }

            let frame = match &last_frame {
                Some(f) => f,
                None => continue,
            };

            let mut enc_lock = encoder.lock().unwrap();
            if let Some(enc) = enc_lock.as_mut() {
                if let Err(e) = enc.encode_frame(&frame.texture) {
                    log!("⚠️ Error encoding: {}", e);
                    break;
                }
            }
        }

        log!("📹 Encoder task beendet");
    })
}
