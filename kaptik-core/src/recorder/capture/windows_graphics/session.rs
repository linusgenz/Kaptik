use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureSession};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::core::*;

use crate::recorder::audio::WasapiCapture;
use crate::recorder::capture::core::d3d;
use crate::recorder::capture::core::ffmpeg::FfmpegEncoder;
use crate::recorder::capture::core::tonemap::ToneMapRenderer;
use crate::recorder::capture::core::utils;
use crate::settings::SETTINGS;
use crate::{log, settings};
use crate::recording_storage::RecordingMetadata;
use super::frame::CapturedFrame;
use super::{audio, encoder_task, interop};

pub struct CaptureSession {
    capture_session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    encoder: Arc<Mutex<Option<FfmpegEncoder>>>,
    metadata: RecordingMetadata,
    output_path: PathBuf,
    frame_sender: mpsc::UnboundedSender<CapturedFrame>,
    _encoder_task: tokio::task::JoinHandle<()>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    game_audio_capture: Option<Arc<Mutex<WasapiCapture>>>,
    microphone_capture: Option<Arc<Mutex<WasapiCapture>>>,
    audio_shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl CaptureSession {
    pub async fn create(
        window_title: &str,
        metadata: RecordingMetadata,
        output_path: PathBuf,
    ) -> Result<Self> {
        // Find window
        let hwnd = utils::find_window_hwnd(window_title)?;

        let (d3d_device, d3d_context) = d3d::create_d3d11_device()?;

        let device = d3d::create_direct3d_device(&d3d_device)?;

        let item = interop::create_capture_item_for_window(hwnd)?;
        let item_size = item.Size()?;

        // Check HDR status
        let is_hdr = d3d::check_hdr_enabled()?;
        log!(
            "🌈 HDR Detection: {}",
            if is_hdr { "HDR active" } else { "SDR" }
        );

        let settings = SETTINGS.read().await;
        let tonemap_algorithm = settings.tonemap_algorithm;
        let output_volume = settings.output_volume;
        let microphone_volume = settings.microphone_volume;

        let (width, height) = (item_size.Width as u32, item_size.Height as u32);

        let fps = match &settings.fps_limit {
            settings::Fps::Fps30 => 30,
            settings::Fps::Fps60 => 60,
            settings::Fps::Fps120 => 120,
        };

        let (game_audio_capture, game_audio_format, microphone_capture, microphone_format) =
            audio::init_captures(&settings)?;

        drop(settings);

        // Create tonemap renderer if HDR is active
        let tonemap = if is_hdr {
            Some(ToneMapRenderer::new(
                &d3d_device,
                width,
                height,
                tonemap_algorithm,
            )?)
        } else {
            None
        };

        let pixel_format = if is_hdr {
            DirectXPixelFormat::R16G16B16A16Float
        } else {
            DirectXPixelFormat::B8G8R8A8UIntNormalized
        };
        log!("🎨 Pixel Format: {:?}", pixel_format);

        // Create frame pool
        let frame_pool =
            Direct3D11CaptureFramePool::CreateFreeThreaded(&device, pixel_format, 2, item_size)?;

        // Create channels
        let (frame_tx, frame_rx) = mpsc::unbounded_channel::<CapturedFrame>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (audio_shutdown_tx, audio_shutdown_rx) = tokio::sync::watch::channel(false);

        // Create encoder
        let encoder = Arc::new(Mutex::new(Some(FfmpegEncoder::new(
            d3d_device.clone(),
            d3d_context.clone(),
            &output_path,
            width,
            height,
            item_size.Width as u32,
            item_size.Height as u32,
            fps,
            is_hdr,
            tonemap,
            game_audio_format,
            microphone_format,
            metadata.recording_id
        )?)));

        audio::start_audio_tasks(
            Arc::clone(&encoder),
            &game_audio_capture,
            &microphone_capture,
            output_volume,
            microphone_volume,
            audio_shutdown_rx.clone(),
        )?;

        // Register frame arrived handler
        let frame_tx_clone = frame_tx.clone();
        frame_pool.FrameArrived(&TypedEventHandler::new(
            move |pool: Ref<Direct3D11CaptureFramePool>, _| -> windows::core::Result<()> {
                let frame = pool.unwrap().TryGetNextFrame()?;
                let surface = frame.Surface()?;

                let texture = interop::surface_to_texture(&surface)
                    .map_err(|e| Error::new::<&str>(HRESULT(-2147024809), &e.to_string()))?;

                let _ = frame_tx_clone.send(CapturedFrame {
                    texture,
                    timestamp: std::time::Instant::now(),
                });

                Ok(())
            },
        ))?;

        // Spawn encoder task
        let encoder_task =
            encoder_task::spawn_encoder_task(Arc::clone(&encoder), frame_rx, shutdown_rx);

        // Start capture
        let capture_session = frame_pool.CreateCaptureSession(&item)?;
        capture_session
            .SetIsBorderRequired(false)
            .expect("Could not set 'border required'");

        capture_session.StartCapture()?;
        log!("▶️ Capture session started");

        Ok(Self {
            capture_session,
            frame_pool,
            encoder,
            metadata,
            output_path,
            frame_sender: frame_tx,
            _encoder_task: encoder_task,
            shutdown_tx,
            game_audio_capture,
            microphone_capture,
            audio_shutdown_tx: Some(audio_shutdown_tx),
        })
    }

    pub async fn stop(self) -> Result<()> {
        if let Some(ref game_audio) = self.game_audio_capture {
            game_audio.lock().unwrap().stop()?;
            log!("[WASAPI] Game audio stopped");
        }

        if let Some(ref mic) = self.microphone_capture {
            mic.lock().unwrap().stop()?;
            log!("[WASAPI] microphone stopped");
        }

        // Signal audio shutdown
        if let Some(tx) = self.audio_shutdown_tx {
            let _ = tx.send(true);
        }

        self.capture_session.Close()?;
        self.frame_pool.Close()?;

        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Close frame channel
        drop(self.frame_sender);

        match tokio::time::timeout(std::time::Duration::from_secs(5), self._encoder_task).await {
            Ok(_) => {
                log!("✅ Encoder task finished");
            }
            Err(_) => {
                log!("⚠️ Encoder task timeout (5s)");
            }
        }

        let mut encoder_guard = self.encoder.lock().unwrap_or_else(|poisoned| {
            log!("⚠️ Encoder mutex poisoned during shutdown");
            poisoned.into_inner()
        });

        if let Some(encoder) = encoder_guard.take() {
            encoder.finalize()?;
            log!("✅ Encoder finalized");
        }

        Ok(())
    }

    pub fn metadata(&self) -> &RecordingMetadata {
        &self.metadata
    }

    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }
}
