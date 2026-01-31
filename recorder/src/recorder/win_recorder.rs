use super::RecordingMetadata;
use crate::game_integration::GameState;
use crate::recorder::audio_devices;
use crate::recorder::utils;
use crate::settings::SETTINGS;
use crate::{ffmpeg_log, log};
use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, mpsc};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::{
        Foundation::HWND,
        Graphics::{Direct3D::*, Direct3D11::*, Dxgi::*},
        System::WinRT::{Direct3D11::IDirect3DDxgiInterfaceAccess, Graphics::Capture::*},
        UI::WindowsAndMessaging::FindWindowW,
    },
    core::*,
};

pub struct WindowsCaptureRecorder {
    is_recording: Arc<RwLock<bool>>,
    current_session: Arc<RwLock<Option<CaptureSession>>>,
}

struct CaptureSession {
    capture_session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    encoder: Arc<Mutex<Option<VideoEncoder>>>,
    metadata: RecordingMetadata,
    output_path: PathBuf,
    frame_sender: mpsc::UnboundedSender<CapturedFrame>,
    _encoder_task: tokio::task::JoinHandle<()>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

struct CapturedFrame {
    texture: ID3D11Texture2D,
    timestamp: std::time::Instant,
}

struct VideoEncoder {
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    ffmpeg_process: std::process::Child,
    native_width: u32,
    native_height: u32,
    width: u32,
    height: u32,
    is_hdr: bool,
    pixel_format: DirectXPixelFormat,
}

impl WindowsCaptureRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(RwLock::new(false)),
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn is_recording(&self) -> bool {
        *self.is_recording.read().await
    }

    pub async fn start_recording(
        &self,
        window_title: &str,
        game_state: Option<GameState>,
    ) -> Result<()> {
        if *self.is_recording.read().await {
            return Err(anyhow::anyhow!("Bereits am Aufnehmen"));
        }

        let metadata = RecordingMetadata {
            game_name: extract_game_name(window_title),
            character_name: game_state.as_ref().and_then(|s| s.character_name.clone()),
            map_name: game_state.as_ref().and_then(|s| s.map_name.clone()),
            round_number: game_state.as_ref().and_then(|s| s.round_number),
            timestamp: chrono::Local::now(),
        };

        let filename = metadata.generate_filename();

        log!("🎬 Starte Windows.Graphics.Capture Recording: {}", filename);

        // 1. Finde das Fenster
        let hwnd = find_window_hwnd(window_title)?;
        log!("🪟 Window Handle gefunden: {:?}", hwnd);

        // 2. Erstelle D3D11 Device
        let (d3d_device, d3d_context) = create_d3d11_device()?;
        log!("🎨 Direct3D11 Device erstellt");

        // 3. Erstelle WinRT Device
        let device = create_direct3d_device(&d3d_device)?;
        log!("✨ WinRT Direct3D Device erstellt");

        // 4. Erstelle GraphicsCaptureItem
        let item = create_capture_item_for_window(hwnd)?;
        let item_size = item.Size()?;

        log!(
            "📦 Capture Item erstellt: {}x{}",
            item_size.Width,
            item_size.Height
        );

        // hdr check
        let is_hdr_enabled = utils::is_hdr_enabled(hwnd, &d3d_device);
        log!(
            "HDR: {}",
            if is_hdr_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        // Ausgabepfad
        let settings = SETTINGS.read().await;
        let mut output_path = PathBuf::from(&settings.video_path);
        if output_path.as_os_str().is_empty() {
            output_path = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));
            output_path.push("Kaptik");
        }

        std::fs::create_dir_all(&output_path)?;
        output_path.push(&filename);

        let (width, height) = match settings.resolution {
            crate::settings::Resolution::R720p => (1280, 720),
            crate::settings::Resolution::R1080p => (1920, 1080),
            crate::settings::Resolution::R1440p => (2560, 1440),
            crate::settings::Resolution::R4K => (3840, 2160),
            crate::settings::Resolution::Source => {
                (item_size.Width as u32, item_size.Height as u32)
            }
        };

        let fps = match settings.fps_limit {
            crate::settings::Fps::Fps30 => 30,
            crate::settings::Fps::Fps60 => 60,
            crate::settings::Fps::Fps120 => 120,
        };

        let game_audio = settings.game_audio;
        let system_sounds = settings.system_sounds;
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

        // Create frame pool
        let pixel_format = if is_hdr_enabled {
            DirectXPixelFormat::R16G16B16A16Float
        } else {
            DirectXPixelFormat::B8G8R8A8UIntNormalized
        };

        let frame_pool =
            Direct3D11CaptureFramePool::CreateFreeThreaded(&device, pixel_format, 2, item_size)?;

        log!("🎞️ Frame Pool erstellt");

        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<CapturedFrame>();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

        // start ffmpeg encoder
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
            is_hdr_enabled,
            pixel_format
        )?)));
        log!("🎬 FFmpeg Encoder gestartet");

        // register frame handler
        let frame_tx_clone = frame_tx.clone();

        frame_pool.FrameArrived(&TypedEventHandler::new(
            move |pool: Ref<Direct3D11CaptureFramePool>, _| -> windows::core::Result<()> {
                let frame = pool.unwrap().TryGetNextFrame()?;
                let surface = frame.Surface()?;

                let texture = surface_to_texture(&surface)
                    .map_err(|e| Error::new::<&str>(HRESULT(-2147024809), &e.to_string()))?;

                // send frame to encoder
                let _ = frame_tx_clone.send(CapturedFrame {
                    texture,
                    timestamp: std::time::Instant::now(),
                });
                Ok(())
            },
        ))?;

        // Encoder task (processes frames asynchronously)
        let encoder_for_task = Arc::clone(&encoder);
        let encoder_task = tokio::task::spawn_blocking(move || {
            loop {
                // Prüfe auf shutdown
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

            log!("📹 Encoder task beendet - keine weiteren Frames");
        });

        // Start capture session
        let capture_session = frame_pool.CreateCaptureSession(&item)?;
        capture_session.StartCapture()?;
        log!("▶️ Capture session started");

        // Save session
        *self.is_recording.write().await = true;
        *self.current_session.write().await = Some(CaptureSession {
            capture_session,
            frame_pool,
            encoder,
            metadata,
            output_path: output_path.clone(),
            frame_sender: frame_tx,
            _encoder_task: encoder_task,
            shutdown_tx,
        });

        log!("✅ Recording started: {}", output_path.display());

        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<()> {
        if !*self.is_recording.read().await {
            return Ok(());
        }

        let recording = {
            let mut session = self.current_session.write().await;
            session.take()
        };

        if let Some(recording) = recording {
            recording.capture_session.Close()?;

            recording.frame_pool.Close()?;

            // send shutdown signal
            let _ = recording.shutdown_tx.send(true);

            // Close frame channel
            drop(recording.frame_sender);

            match tokio::time::timeout(std::time::Duration::from_secs(5), recording._encoder_task)
                .await
            {
                Ok(_) => log!("✅ Encoder task finished"),
                Err(_) => log!("⚠️ Encoder task timeout (5s)"),
            }

            if let Some(encoder) = recording.encoder.lock().unwrap().take() {
                encoder.finalize()?;
                log!("✅ Encoder finalized");
            }

            *self.is_recording.write().await = false;

            log!(
                "✅ Capture saved under: {}",
                recording.output_path.display()
            );
        }

        Ok(())
    }

    pub async fn get_current_metadata(&self) -> Option<RecordingMetadata> {
        let session = self.current_session.read().await;
        session.as_ref().map(|s| s.metadata.clone())
    }
}

fn find_window_hwnd(window_title: &str) -> Result<HWND> {
    unsafe {
        let mut title_wide: Vec<u16> = window_title.encode_utf16().collect();
        title_wide.push(0);

        let hwnd = FindWindowW(None, PCWSTR::from_raw(title_wide.as_ptr()))?;

        if hwnd.0.is_null() {
            return Err(anyhow::anyhow!("Fenster nicht gefunden: {}", window_title));
        }

        Ok(hwnd)
    }
}

fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    unsafe {
        let mut device = None;
        let mut context = None;

        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE(std::ptr::null_mut()),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;

        Ok((
            device.ok_or_else(|| anyhow::anyhow!("Device creation failed"))?,
            context.ok_or_else(|| anyhow::anyhow!("Context creation failed"))?,
        ))
    }
}

fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice> {
    unsafe {
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
        Ok(inspectable.cast()?)
    }
}

fn create_capture_item_for_window(hwnd: HWND) -> windows::core::Result<GraphicsCaptureItem> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;

    unsafe { interop.CreateForWindow(hwnd) }
}

fn surface_to_texture(
    surface: &windows::Graphics::DirectX::Direct3D11::IDirect3DSurface,
) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let texture: ID3D11Texture2D = access.GetInterface()?;
        Ok(texture)
    }
}

//
// Video Encoder
//

impl VideoEncoder {
    fn new(
        d3d_device: ID3D11Device,
        d3d_context: ID3D11DeviceContext,
        output_path: &PathBuf,
        width: u32,
        height: u32,
        native_width: u32,
        native_height: u32,
        fps: u32,
        game_audio: bool,
        microphone: bool,
        game_audio_device: Option<String>,
        microphone_device: Option<String>,
        is_hdr_enabled: bool,
        pixel_format: DirectXPixelFormat,
    ) -> Result<Self> {
        let mut args = Vec::new();

        if is_hdr_enabled {
            args.extend_from_slice(&[
                "-f".to_string(),
                "rawvideo".to_string(),
                "-pixel_format".to_string(),
                "rgba64le".to_string(), // 16-bit HDR
                "-colorspace".to_string(),
                "bt2020nc".to_string(),
                "-color_primaries".to_string(),
                "bt2020".to_string(),
                "-color_trc".to_string(),
                "smpte2084".to_string(),
                "-color_range".to_string(),
                "pc".to_string(),

                "-video_size".to_string(),
                format!("{}x{}", native_width, native_height),
                "-framerate".to_string(),
                fps.to_string(),
                "-i".to_string(),
                "pipe:0".to_string(),
            ]);
        } else {
            args.extend_from_slice(&[
                "-f".to_string(),
                "rawvideo".to_string(),
                "-pixel_format".to_string(),
                "bgra".to_string(),
                "-color_range".to_string(),
                "pc".to_string(),
                "-video_size".to_string(),
                format!("{}x{}", native_width, native_height),
                "-framerate".to_string(),
                fps.to_string(),
                "-i".to_string(),
                "pipe:0".to_string(),
            ]);
        }

        // Dynamische Audio-Eingaben
        let mut audio_inputs = Vec::new();

        if game_audio {
            if let Some(device) = game_audio_device {
                args.extend_from_slice(&[
                    "-f".to_string(),
                    "dshow".to_string(),
                    "-rtbufsize".to_string(),
                    "50M".to_string(),
                    "-thread_queue_size".to_string(),
                    "512".to_string(),
                    "-i".to_string(),
                    format!("audio={}", device),
                ]);
                audio_inputs.push(format!("{}:a", audio_inputs.len() + 1));
            }
        }

        if microphone {
            if let Some(device) = microphone_device {
                args.extend_from_slice(&[
                    "-f".to_string(),
                    "dshow".to_string(),
                    "-rtbufsize".to_string(),
                    "50M".to_string(),
                    "-thread_queue_size".to_string(),
                    "512".to_string(),
                    "-i".to_string(),
                    format!("audio={}", device),
                ]);
                audio_inputs.push(format!("{}:a", audio_inputs.len() + 1));
            }
        }

        // Audio-Mapping
        if !audio_inputs.is_empty() {
            args.push("-map".to_string());
            args.push("0:v".to_string());

            for inp in &audio_inputs {
                args.push("-map".to_string());
                args.push(inp.clone());
            }

            args.extend_from_slice(&[
                "-acodec".to_string(),
                "aac".to_string(),
                "-ab".to_string(),
                "192k".to_string(),
                "-ac".to_string(),
                "2".to_string(),
                "-async".to_string(),
                "1".to_string(),
            ]);
        } else {
            args.push("-an".to_string());
        }

        if is_hdr_enabled {
            args.push("-init_hw_device".to_string());
            args.push("vulkan".to_string());
            args.push("-vf".to_string());
            args.push(format!(
                "zscale=transfer=smpte2084:primaries=bt2020:matrix=bt2020nc,\
     zscale=transfer=linear,\
     tonemap=hable:desat=0,\
     zscale=transfer=bt709:primaries=bt709:matrix=bt709,\
     scale=w={}:h={}:flags=lanczos,\
     format=yuv420p",
                width, height
            ));

        } else {
            // SDR fallback
            args.push("-vf".to_string());
            args.push(format!(
                "scale=w={}:h={}:flags=lanczos,format=yuv420p",
                width, height
            ));
        }

        // Video Encoder
        args.extend_from_slice(&[
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-crf".to_string(),
            "18".to_string(),
            "-profile:v".to_string(),
            "high".to_string(),
            "-level".to_string(),
            "4.2".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-colorspace".to_string(),
            "bt709".to_string(),
            "-color_primaries".to_string(),
            "bt709".to_string(),
            "-color_trc".to_string(),
            "bt709".to_string(),
            "-color_range".to_string(),
            "tv".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            "-shortest".to_string(),
            output_path.to_str().unwrap().to_string(),
        ]);

        log!("🔧 FFmpeg Args: ffmpeg {}", args.join(" "));

        let mut process = std::process::Command::new("ffmpeg")
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stderr) = process.stderr.take() {
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        ffmpeg_log!("{}", line);
                    }
                }
            });
        }

        Ok(Self {
            d3d_device,
            d3d_context,
            ffmpeg_process: process,
            width,
            height,
            native_width,
            native_height,
            is_hdr: is_hdr_enabled,
            pixel_format
        })
    }

    fn encode_frame(&mut self, texture: &ID3D11Texture2D) -> Result<()> {
        unsafe {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);

            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;

            let mut staging_texture: Option<ID3D11Texture2D> = None;
            self.d3d_device
                .CreateTexture2D(&desc, None, Some(&mut staging_texture))?;
            let staging_texture = staging_texture.unwrap();

            // GPU → CPU copy
            self.d3d_context.CopyResource(&staging_texture, texture);

            // Map
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.d3d_context
                .Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            if let Some(stdin) = self.ffmpeg_process.stdin.as_mut() {
                let row_pitch = mapped.RowPitch as usize;
                let bytes_per_pixel = if self.is_hdr { 8 } else { 4 };
                let width_bytes = (self.native_width as usize) * bytes_per_pixel;

                let ptr = mapped.pData as *const u8;

                for row in 0..self.native_height as usize {
                    let start = ptr.add(row * row_pitch);
                    let slice = std::slice::from_raw_parts(start, width_bytes);
                    stdin.write_all(slice)?;
                }
            }

            self.d3d_context.Unmap(&staging_texture, 0);
        }
        Ok(())
    }

    fn finalize(mut self) -> Result<()> {
        // close stdin
        drop(self.ffmpeg_process.stdin.take());

        log!("🔍 Finalize: stdin closed, waiting for ffmpeg process...");

        let start = std::time::Instant::now();

        match self.ffmpeg_process.try_wait() {
            Ok(Some(status)) => {
                log!("✅ FFmpeg already finished: {}", status);
                return Ok(());
            }
            Ok(None) => {
                log!("🔍 FFmpeg still running, waiting...");
            }
            Err(e) => {
                log!("⚠️ Error checking ffmpeg status: {}", e);
            }
        }

        let timeout = std::time::Duration::from_secs(10);

        loop {
            match self.ffmpeg_process.try_wait() {
                Ok(Some(status)) => {
                    log!("✅ FFmpeg finished after {:?}: {}", start.elapsed(), status);
                    if !status.success() {
                        log!("⚠️ FFmpeg exit status: {}", status);
                    }
                    return Ok(());
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        log!("⚠️ FFmpeg timeout after {:?}, killing process...", timeout);
                        let _ = self.ffmpeg_process.kill();
                        let _ = self.ffmpeg_process.wait();
                        return Err(anyhow::anyhow!("FFmpeg timeout"));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    log!("⚠️ Error waiting for ffmpeg: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}

fn extract_game_name(window_title: &str) -> String {
    window_title
        .split(&['-', '(', ')', '™', '®'][..])
        .next()
        .unwrap_or(window_title)
        .trim()
        .replace(" ", "_")
}
