use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

use crate::{log, settings};
use crate::recorder::audio_devices;
use crate::settings::SETTINGS;
use super::d3d;
use super::encoder::VideoEncoder;
use super::tonemap::ToneMapRenderer;
use super::RecordingMetadata;

/// A single captured frame with timestamp
struct CapturedFrame {
    texture: ID3D11Texture2D,
    timestamp: std::time::Instant,
}

/// Active capture session
pub struct CaptureSession {
    capture_session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    encoder: Arc<Mutex<Option<VideoEncoder>>>,
    metadata: RecordingMetadata,
    output_path: PathBuf,
    frame_sender: mpsc::UnboundedSender<CapturedFrame>,
    _encoder_task: tokio::task::JoinHandle<()>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl CaptureSession {
    pub async fn create(
        window_title: &str,
        metadata: RecordingMetadata,
        output_path: PathBuf,
    ) -> Result<Self> {
        // Find window
        let hwnd = find_window_hwnd(window_title)?;

        // Create D3D11 device
        let (d3d_device, d3d_context) = d3d::create_d3d11_device()?;

        // Create WinRT device
        let device = d3d::create_direct3d_device(&d3d_device)?;

        // Create capture item
        let item = create_capture_item_for_window(hwnd)?;
        let item_size = item.Size()?;

        // Check HDR status
        let is_hdr = d3d::check_hdr_enabled()?;
        log!("🌈 HDR Detection: {}", if is_hdr { "HDR aktiv" } else { "SDR" });

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

        // Get settings
        let settings = SETTINGS.read().await;
        let (width, height) = (item_size.Width as u32, item_size.Height as u32);

        let fps = match settings.fps_limit {
            crate::settings::Fps::Fps30 => 30,
            crate::settings::Fps::Fps60 => 60,
            crate::settings::Fps::Fps120 => 120,
        };

        let game_audio = settings.game_audio;
        let microphone = settings.microphone;

        let game_audio_device = if settings.game_audio || settings.system_sounds {
            audio_devices::get_game_audio_device(&settings)
        } else {
            None
        };

        let microphone_device = if settings.microphone {
            audio_devices::get_microphone_device(&settings)
        } else {
            None
        };

        drop(settings);

        // Determine pixel format
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
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

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
            game_audio,
            microphone,
            game_audio_device,
            microphone_device,
            pixel_format,
            is_hdr,
            tonemap,
        )?)));

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
        })
    }

    pub async fn stop(self) -> Result<()> {
        // Stop capture
        self.capture_session.Close()?;
        self.frame_pool.Close()?;

        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Close frame channel
        drop(self.frame_sender);

        // Wait for encoder task
        match tokio::time::timeout(std::time::Duration::from_secs(5), self._encoder_task).await {
            Ok(_) => log!("✅ Encoder task finished"),
            Err(_) => log!("⚠️ Encoder task timeout (5s)"),
        }

        // Finalize encoder
        if let Some(encoder) = self.encoder.lock().unwrap().take() {
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

fn find_window_hwnd(window_title: &str) -> Result<HWND> {
    unsafe {
        let mut title_wide: Vec<u16> = window_title.encode_utf16().collect();
        title_wide.push(0);

        let hwnd = FindWindowW(None, PCWSTR::from_raw(title_wide.as_ptr()))?;

        if hwnd.0.is_null() {
            return Err(anyhow::anyhow!("Window not found: {}", window_title));
        }

        Ok(hwnd)
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