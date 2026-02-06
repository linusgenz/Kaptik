use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use windows::core::*;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::*;

use crate::{log, settings};
use crate::recorder::audio_devices;
use crate::recorder::capture::audio_mixer::AudioMixer;
use crate::settings::SETTINGS;
use crate::recorder::capture::core::d3d;
use crate::recorder::capture::core::encoder::VideoEncoder;
use crate::recorder::capture::core::tonemap::ToneMapRenderer;
use crate::recorder::capture::core::wasapi::{WasapiCapture, AudioSample};
use crate::recorder::RecordingMetadata;
use crate::recorder::capture::core::utils;

struct CapturedFrame {
    texture: ID3D11Texture2D,
    timestamp: std::time::Instant,
}

pub struct CaptureSession {
    capture_session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    encoder: Arc<Mutex<Option<VideoEncoder>>>,
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

        let item = create_capture_item_for_window(hwnd)?;
        let item_size = item.Size()?;

        // Check HDR status
        let is_hdr = d3d::check_hdr_enabled()?;
        log!("🌈 HDR Detection: {}", if is_hdr { "HDR active" } else { "SDR" });

        // Create tonemap renderer if HDR is active
        let tonemap = if is_hdr {
            Some(ToneMapRenderer::new(
                &d3d_device,
                item_size.Width as u32,
                item_size.Height as u32,
                settings::get_setting(|s| s.tonemap_algorithm).await
            )?)
        } else {
            None
        };

        let settings = SETTINGS.read().await;
        let (width, height) = (item_size.Width as u32, item_size.Height as u32);

        let fps = match settings.fps_limit {
            settings::Fps::Fps30 => 30,
            settings::Fps::Fps60 => 60,
            settings::Fps::Fps120 => 120,
        };

        let game_audio = settings.game_audio;
        let microphone = settings.microphone;

        let (game_audio_capture, game_audio_format) = if game_audio || settings.system_sounds {
            if let Ok(device_id) = audio_devices::get_game_audio_device(&settings) {
                log!("Initialize WASAPI game audio: {}", device_id);
                let capture = WasapiCapture::new(&device_id, true)?; // Loopback
                let format = *capture.get_format();
                (Some(Arc::new(Mutex::new(capture))), Some(format))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let (microphone_capture, microphone_format) = if microphone {
            if let Ok(device_id) = audio_devices::get_microphone_device(&settings) {
                log!("Initialize WASAPI microphone: {}", device_id);
                let capture = WasapiCapture::new(&device_id, false)?; // No loopback
                let format = *capture.get_format();
                (Some(Arc::new(Mutex::new(capture))), Some(format))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        drop(settings);

        let pixel_format = if is_hdr {
            DirectXPixelFormat::R16G16B16A16Float
        } else {
            DirectXPixelFormat::B8G8R8A8UIntNormalized
        };
        log!("🎨 Pixel Format: {:?}", pixel_format);

        // Create frame pool
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            pixel_format,
            2,
            item_size,
        )?;

        // Create channels
        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<CapturedFrame>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (audio_shutdown_tx, audio_shutdown_rx) = tokio::sync::watch::channel(false);

        // Create encoder
        let encoder = Arc::new(Mutex::new(Some(VideoEncoder::new(
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
        )?)));

        let encoder_for_audio = Arc::clone(&encoder);

        let has_game = game_audio_capture.is_some();
        let has_mic = microphone_capture.is_some();

        if has_game || has_mic {
            let (game_vol, mic_vol) = settings::get_setting(|s| (s.output_volume, s.microphone_volume)).await;

            let mixer = Arc::new(AudioMixer::new(
                has_game,
                has_mic,
                game_vol,
                mic_vol,
            ));

            if let Some(ref game_audio) = game_audio_capture {
                let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioSample>();
                let game_audio_clone = Arc::clone(game_audio);
                let mixer_clone = Arc::clone(&mixer);

                game_audio_clone.lock().unwrap().start(audio_tx)?;
                log!("[WASAPI] Game Audio gestartet");

                let audio_shutdown_rx_clone = audio_shutdown_rx.clone();
                tokio::task::spawn_blocking(move || {
                    while let Some(sample) = audio_rx.blocking_recv() {
                        if *audio_shutdown_rx_clone.borrow() {
                            break;
                        }
                        mixer_clone.add_game_sample(sample);
                    }
                    log!("🎵 Game Audio capture task beendet");
                });
            }

            // Start Microphone capture if enabled
            if let Some(ref mic) = microphone_capture {
                let (mic_tx, mut mic_rx) = mpsc::unbounded_channel::<AudioSample>();
                let mic_clone = Arc::clone(mic);
                let mixer_clone = Arc::clone(&mixer);

                mic_clone.lock().unwrap().start(mic_tx)?;
                log!("[WASAPI] Microphone gestartet");

                let audio_shutdown_rx_clone = audio_shutdown_rx.clone();
                tokio::task::spawn_blocking(move || {
                    while let Some(sample) = mic_rx.blocking_recv() {
                        if *audio_shutdown_rx_clone.borrow() {
                            break;
                        }
                        mixer_clone.add_mic_sample(sample);
                    }
                    log!("🎵 Microphone capture task beendet");
                });
            }

            // Mixer Task - combines sources and writes to encoder
            let encoder_clone = Arc::clone(&encoder_for_audio);
            let mixer_clone = Arc::clone(&mixer);
            let audio_shutdown_rx_clone = audio_shutdown_rx.clone();

            tokio::task::spawn_blocking(move || {
                log!("Audio mixer task started");

                loop {
                    if *audio_shutdown_rx_clone.borrow() {
                        break;
                    }

                    if let Some(mixed_sample) = mixer_clone.get_next_mixed() {
                        let mut enc_lock = encoder_clone.lock().unwrap();
                        if let Some(enc) = enc_lock.as_mut() {
                            let _ = enc.write_audio_sample(&mixed_sample);
                        }
                    } else {
                        // No samples available yet, sleep briefly
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }

                log!("Audio mixer task completed");
            });
        }

        // Register frame arrived handler
        let frame_tx_clone = frame_tx.clone();
        frame_pool.FrameArrived(&TypedEventHandler::new(
            move |pool: Ref<Direct3D11CaptureFramePool>, _| -> windows::core::Result<()> {
                let frame = pool.unwrap().TryGetNextFrame()?;
                let surface = frame.Surface()?;

                let texture = surface_to_texture(&surface)
                    .map_err(|e| Error::new::<&str>(HRESULT(-2147024809), &e.to_string()))?;

                let _ = frame_tx_clone.send(CapturedFrame {
                    texture,
                    timestamp: std::time::Instant::now(),
                });

                Ok(())
            },
        ))?;

        // Spawn encoder task
        let encoder_for_task = Arc::clone(&encoder);
        let encoder_task = tokio::task::spawn_blocking(move || {
            loop {
                if *shutdown_rx.borrow() {
                    log!("🛑 Encoder shutdown signal received");
                    break;
                }

                match frame_rx.try_recv() {
                    Ok(frame) => {
                        let mut enc_lock = encoder_for_task.lock().unwrap();
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
        });

        // Start capture
        let capture_session = frame_pool.CreateCaptureSession(&item)?;
        capture_session.SetIsBorderRequired(false).expect("Could not set 'border required'");

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

        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self._encoder_task
        ).await {
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

fn create_capture_item_for_window(hwnd: HWND) -> windows::core::Result<GraphicsCaptureItem> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForWindow(hwnd) }
}

fn surface_to_texture(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let texture: ID3D11Texture2D = access.GetInterface()?;
        Ok(texture)
    }
}