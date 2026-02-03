use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

use crate::{ffmpeg_log, log};
use super::tonemap::ToneMapRenderer;

pub struct VideoEncoder {
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    ffmpeg_process: std::process::Child,
    native_width: u32,
    native_height: u32,
    is_hdr: bool,
    tonemap: Option<ToneMapRenderer>,
}

impl VideoEncoder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        pixel_format: DirectXPixelFormat,
        is_hdr: bool,
        tonemap: Option<ToneMapRenderer>,
    ) -> Result<Self> {
        let args = build_ffmpeg_args(
            output_path,
            width,
            height,
            native_width,
            native_height,
            fps,
            game_audio,
            microphone,
            game_audio_device,
            microphone_device,
            pixel_format,
            is_hdr,
            tonemap.is_some(),
        );

        log!("🔧 FFmpeg Args: ffmpeg {}", args.join(" "));

        let mut process = std::process::Command::new("ffmpeg")
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Spawn thread to log FFmpeg output
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
            native_width,
            native_height,
            is_hdr,
            tonemap,
        })
    }

    pub fn encode_frame(&mut self, texture: &ID3D11Texture2D) -> Result<()> {
        unsafe {
            // Convert HDR to SDR if needed
            let frame_to_send = if self.is_hdr && self.tonemap.is_some() {
                self.tonemap
                    .as_ref()
                    .unwrap()
                    .tonemap(&self.d3d_device, &self.d3d_context, texture)?
            } else {
                texture.clone()
            };

            // Get texture description
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            frame_to_send.GetDesc(&mut desc);

            // Determine bytes per pixel
            let bytes_per_pixel = if desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM {
                4 // SDR
            } else {
                8 // HDR R16G16B16A16Float
            };

            // Create staging texture for CPU access
            let staging_texture = create_staging_texture(&self.d3d_device, &desc)?;

            // Copy GPU texture to CPU-accessible staging texture
            self.d3d_context.CopyResource(&staging_texture, &frame_to_send);
            self.d3d_context.Flush();

            // Map texture for reading
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.d3d_context.Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            // Write to FFmpeg stdin
            if let Some(stdin) = self.ffmpeg_process.stdin.as_mut() {
                write_texture_data(
                    stdin,
                    &mapped,
                    self.native_width,
                    self.native_height,
                    bytes_per_pixel,
                )?;
            }

            self.d3d_context.Unmap(&staging_texture, 0);
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        // Close stdin to signal end of input
        drop(self.ffmpeg_process.stdin.take());

        log!("🔍 Finalize: stdin closed, waiting for ffmpeg process...");

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(10);

        // Check if already finished
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

        // Wait for FFmpeg to finish with timeout
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

fn create_staging_texture(
    device: &ID3D11Device,
    source_desc: &D3D11_TEXTURE2D_DESC,
) -> Result<ID3D11Texture2D> {
    unsafe {
        let mut desc = *source_desc;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;

        let mut staging_texture: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&desc, None, Some(&mut staging_texture))?;

        staging_texture.ok_or_else(|| anyhow::anyhow!("Failed to create staging texture"))
    }
}

fn write_texture_data(
    stdin: &mut std::process::ChildStdin,
    mapped: &D3D11_MAPPED_SUBRESOURCE,
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
) -> Result<()> {
    unsafe {
        let ptr = mapped.pData as *const u8;
        let row_pitch = mapped.RowPitch as usize;
        let row_size = width as usize * bytes_per_pixel;

        if row_pitch < row_size {
            return Err(anyhow::anyhow!(
                "Invalid row_pitch: {} < {}",
                row_pitch,
                row_size
            ));
        }

        for row in 0..height as usize {
            let start = ptr.add(row * row_pitch);
            let slice = std::slice::from_raw_parts(start, row_size);
            stdin.write_all(slice)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_ffmpeg_args(
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
    pixel_format: DirectXPixelFormat,
    is_hdr: bool,
    has_tonemap: bool,
) -> Vec<String> {
    let mut args = Vec::new();

    // Video input configuration
    add_video_input_args(&mut args, pixel_format, is_hdr, has_tonemap, native_width, native_height, fps);

    // Audio inputs
    let audio_inputs = add_audio_input_args(
        &mut args,
        game_audio,
        microphone,
        game_audio_device,
        microphone_device,
    );

    // Audio mapping and encoding
    add_audio_encoding_args(&mut args, audio_inputs);

    // Video filtering
    add_video_filter_args(&mut args, is_hdr, has_tonemap, width, height);

    // Video encoding
    add_video_encoding_args(&mut args);

    // Output file
    args.push(output_path.to_str().unwrap().to_string());

    args
}

fn add_video_input_args(
    args: &mut Vec<String>,
    pixel_format: DirectXPixelFormat,
    is_hdr: bool,
    has_tonemap: bool,
    native_width: u32,
    native_height: u32,
    fps: u32,
) {
    args.extend_from_slice(&[
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pixel_format".to_string(),
    ]);

    if is_hdr && !has_tonemap {
        // HDR without tonemap (FFmpeg will handle it)
        args.extend_from_slice(&[
            "rgbaf16le".to_string(),
            "-color_trc".to_string(),
            "smpte2084".to_string(),
            "-color_primaries".to_string(),
            "bt2020".to_string(),
            "-colorspace".to_string(),
            "bt2020nc".to_string(),
        ]);
    } else {
        // SDR or already tonemapped
        args.push("bgra".to_string());
    }

    args.extend_from_slice(&[
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

fn add_audio_input_args(
    args: &mut Vec<String>,
    game_audio: bool,
    microphone: bool,
    game_audio_device: Option<String>,
    microphone_device: Option<String>,
) -> Vec<String> {
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

    audio_inputs
}

fn add_audio_encoding_args(args: &mut Vec<String>, audio_inputs: Vec<String>) {
    if !audio_inputs.is_empty() {
        args.push("-map".to_string());
        args.push("0:v".to_string());

        for input in &audio_inputs {
            args.push("-map".to_string());
            args.push(input.clone());
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
}

fn add_video_filter_args(
    args: &mut Vec<String>,
    is_hdr: bool,
    has_tonemap: bool,
    width: u32,
    height: u32,
) {
    args.push("-vf".to_string());

    if is_hdr && !has_tonemap {
        // FFmpeg-based tonemapping
        args.push(format!(
            "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=reinhard:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p,scale=w={}:h={}:flags=lanczos",
            width, height
        ));
        log!("🌈 HDR Tonemapping aktiviert (FFmpeg Reinhard)");
    } else {
        args.push(format!(
            "scale=w={}:h={}:flags=lanczos,format=yuv420p",
            width, height
        ));

        if has_tonemap {
            log!("🌈 Tonemap Renderer aktiv (GPU-seitig)");
        }
    }
}

fn add_video_encoding_args(args: &mut Vec<String>) {
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
    ]);
}