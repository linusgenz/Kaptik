use anyhow::Result;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Media::Audio::WAVEFORMATEX;

use crate::recorder::capture::core::tonemap::ToneMapRenderer;
use crate::recorder::capture::core::wasapi::AudioSample;
use crate::{ffmpeg_log, log};

struct VideoFrame {
    data: Vec<u8>,
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
}

pub struct VideoEncoder {
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    native_width: u32,
    native_height: u32,
    is_hdr: bool,
    tonemap: Option<ToneMapRenderer>,

    frame_sender: Option<mpsc::SyncSender<VideoFrame>>,
    video_thread: Option<thread::JoinHandle<()>>,

    audio_buffer: VecDeque<Vec<u8>>,
    audio_writer: Option<BufWriter<File>>,
    audio_sample_rate: u32,
    audio_channels: u16,
    audio_samples_written: u64,

    temp_video_path: PathBuf,
    temp_audio_path: Option<PathBuf>,
    output_path: PathBuf,
    game_audio_format: Option<WAVEFORMATEX>,
    microphone_format: Option<WAVEFORMATEX>,

    fps: u32,
    frame_count: u64,
    last_log_time: std::time::Instant,
    recording_start: std::time::Instant,
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
        is_hdr: bool,
        tonemap: Option<ToneMapRenderer>,
        game_audio_format: Option<WAVEFORMATEX>,
        microphone_format: Option<WAVEFORMATEX>,
    ) -> Result<Self> {
        let mut config_dir = dirs::config_dir().unwrap();
        config_dir.push("Kaptik");
        let temp_video_path = config_dir.with_extension("temp.mp4");

        let video_args = build_video_ffmpeg_args(
            width,
            height,
            native_width,
            native_height,
            fps,
            is_hdr,
            tonemap.is_some(),
        );

        log!("🔧 FFmpeg Video Args: ffmpeg {}", video_args.join(" "));

        let buffer_size = (fps * 2) as usize;
        let (sender, receiver) = mpsc::sync_channel::<VideoFrame>(buffer_size);

        let video_args_with_output = {
            let mut args = video_args.clone();
            args.push(temp_video_path.to_str().unwrap().to_string());
            args
        };

        let video_thread = thread::spawn(move || {
            Self::video_encoding_thread(video_args_with_output, receiver);
        });

        let (audio_writer, temp_audio_path, audio_sample_rate, audio_channels) =
            if game_audio_format.is_some() || microphone_format.is_some() {
                let audio_path = config_dir.with_extension("temp.wav");
                let file = File::create(&audio_path)?;
                let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file);

                let format = game_audio_format.as_ref().unwrap();
                let sample_rate = format.nSamplesPerSec;
                let channels = format.nChannels;

                write_wav_header(&mut writer, sample_rate, channels)?;

                log!("🎵 Audio wird geschrieben nach: {:?} (WAV Format)", audio_path);
                log!("🎵 Audio Format: {} Hz, {} channels", sample_rate, channels);

                (Some(writer), Some(audio_path), sample_rate, channels)
            } else {
                (None, None, 0, 0)
            };

        Ok(Self {
            d3d_device,
            d3d_context,
            native_width,
            native_height,
            is_hdr,
            tonemap,
            frame_sender: Some(sender),
            video_thread: Some(video_thread),
            audio_buffer: VecDeque::new(),
            audio_writer,
            audio_sample_rate,
            audio_channels,
            audio_samples_written: 0,
            temp_video_path,
            temp_audio_path,
            output_path: output_path.clone(),
            game_audio_format,
            microphone_format,
            fps,
            frame_count: 0,
            last_log_time: std::time::Instant::now(),
            recording_start: std::time::Instant::now(),
        })
    }

    fn video_encoding_thread(args: Vec<String>, receiver: mpsc::Receiver<VideoFrame>) {
        log!("🎬 Video encoding thread gestartet");

        let mut process = match std::process::Command::new("ffmpeg")
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(p) => p,
            Err(e) => {
                log!("⚠️ FFmpeg Video process spawn failed: {}", e);
                return;
            }
        };

        if let Some(stderr) = process.stderr.take() {
            thread::spawn(move || {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    ffmpeg_log!("[VIDEO] {}", line);
                }
            });
        }

        let mut stdin = process.stdin.take().unwrap();
        let mut count = 0u64;

        while let Ok(frame) = receiver.recv() {
            if stdin.write_all(&frame.data).is_err() {
                break;
            }
            count += 1;
        }

        log!("🎬 Video encoding thread: {} frames geschrieben", count);
        drop(stdin);

        match process.wait() {
            Ok(status) => {
                log!("✅ Video FFmpeg beendet: {}", status);
            }
            Err(e) => {
                log!("⚠️ Video FFmpeg wait error: {}", e);
            }
        }
    }

    pub fn encode_frame(&mut self, texture: &ID3D11Texture2D) -> Result<()> {
        unsafe {
            let frame_to_send = if self.is_hdr && self.tonemap.is_some() {
                self.tonemap.as_ref().unwrap().tonemap(&self.d3d_device, &self.d3d_context, texture)?
            } else {
                texture.clone()
            };

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            frame_to_send.GetDesc(&mut desc);

            let bytes_per_pixel = if desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM { 4 } else { 8 };

            let staging_texture = create_staging_texture(&self.d3d_device, &desc)?;
            self.d3d_context.CopyResource(&staging_texture, &frame_to_send);
            self.d3d_context.Flush();

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.d3d_context.Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            let frame_data = extract_texture_data(&mapped, self.native_width, self.native_height, bytes_per_pixel)?;

            self.d3d_context.Unmap(&staging_texture, 0);

            let video_frame = VideoFrame {
                data: frame_data,
                width: self.native_width,
                height: self.native_height,
                bytes_per_pixel,
            };

            if let Some(sender) = &self.frame_sender {
                match sender.try_send(video_frame) {
                    Ok(_) => {
                        self.frame_count += 1;
                    }
                    Err(mpsc::TrySendError::Full(_)) => {
                        self.frame_count += 1;
                        if self.frame_count % 60 == 0 {
                            log!("⚠️ Video buffer voll! Frame #{} übersprungen", self.frame_count);
                        }
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        return Err(anyhow::anyhow!("Video encoding thread disconnected"));
                    }
                }
            }
        }

        Ok(())
    }

    // This is the method that session.rs calls
    pub fn write_audio_sample(&mut self, sample: &AudioSample) -> Result<()> {
        // Add to buffer
        self.audio_buffer.push_back(sample.data.clone());

        // Flush based on time
        self.flush_audio_buffer()?;

        Ok(())
    }

    fn flush_audio_buffer(&mut self) -> Result<()> {
        if self.audio_buffer.is_empty() {
            return Ok(());
        }

        let writer = match &mut self.audio_writer {
            Some(w) => w,
            None => return Ok(()),
        };

        // Time-based sync: write audio to match elapsed time
        let elapsed = self.recording_start.elapsed().as_secs_f64();
        let target_samples = (elapsed * self.audio_sample_rate as f64) as u64;
        let current_samples = self.audio_samples_written;

        let samples_needed = if target_samples > current_samples {
            target_samples - current_samples
        } else {
            0
        };

        // Only flush if we have a decent buffer OR we're behind schedule
        if samples_needed == 0 && self.audio_buffer.len() < 10 {
            return Ok(());
        }

        // Write samples from buffer
        while !self.audio_buffer.is_empty() {
            let chunk = self.audio_buffer.pop_front().unwrap();
            writer.write_all(&chunk)?;

            let samples_in_chunk = chunk.len() / (self.audio_channels as usize * 2);
            self.audio_samples_written += samples_in_chunk as u64;

            // Stop if we've written enough
            if samples_needed > 0 && self.audio_samples_written >= target_samples {
                break;
            }
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        log!("🔍 Finalisiere Encoding...");

        // Flush all remaining audio
        if let Some(writer) = &mut self.audio_writer {
            while let Some(chunk) = self.audio_buffer.pop_front() {
                writer.write_all(&chunk)?;
                let samples_in_chunk = chunk.len() / (self.audio_channels as usize * 2);
                self.audio_samples_written += samples_in_chunk as u64;
            }
            writer.flush()?;
        }

        log!("📊 Frames: {}, Audio Samples: {}", self.frame_count, self.audio_samples_written);

        if let Some(writer) = self.audio_writer.as_mut() {
            writer.flush()?;
            if let Some(audio_path) = &self.temp_audio_path {
                finalize_wav_header(audio_path, self.audio_samples_written, self.audio_channels)?;
                log!("✅ WAV-Datei finalisiert: {} samples", self.audio_samples_written);
            }
        }

        self.frame_sender.take();

        if let Some(handle) = self.video_thread.take() {
            log!("🎬 Warte auf Video-Encoding Thread...");
            handle.join().ok();
            log!("✅ Video-Encoding Thread beendet");
        }

        if !self.temp_video_path.exists() {
            return Err(anyhow::anyhow!("Temp video file not found"));
        }

        let video_size = std::fs::metadata(&self.temp_video_path)?.len();
        log!("✅ Temp video: {} bytes", video_size);

        if let Some(temp_audio_path) = &self.temp_audio_path {
            if temp_audio_path.exists() {
                let audio_size = std::fs::metadata(temp_audio_path)?.len();
                log!("🎵 Temp WAV: {} bytes", audio_size);

                if audio_size > 44 {
                    self.mux_audio_to_video(temp_audio_path)?;
                } else {
                    std::fs::copy(&self.temp_video_path, &self.output_path)?;
                }

                let _ = std::fs::remove_file(temp_audio_path);
            } else {
                std::fs::copy(&self.temp_video_path, &self.output_path)?;
            }

            let _ = std::fs::remove_file(&self.temp_video_path);
        } else {
            std::fs::rename(&self.temp_video_path, &self.output_path)?;
        }

        log!("✅ Encoding finalized");
        Ok(())
    }

    fn mux_audio_to_video(&self, temp_audio_path: &PathBuf) -> Result<()> {
        let args = vec![
            "-i", self.temp_video_path.to_str().unwrap(),
            "-i", temp_audio_path.to_str().unwrap(),
            "-map", "0:v", "-map", "1:a",
            "-c:v", "copy",
            "-c:a", "aac", "-b:a", "192k",
            "-async", "1",
            "-af", "aresample=async=1:first_pts=0",
            "-shortest", "-y",
            self.output_path.to_str().unwrap(),
        ];

        log!("🔧 FFmpeg Mux: {}", args.join(" "));

        let mut process = std::process::Command::new("ffmpeg")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stderr) = process.stderr.take() {
            thread::spawn(move || {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    ffmpeg_log!("[MUX] {}", line);
                }
            });
        }

        let start = std::time::Instant::now();
        loop {
            match process.try_wait() {
                Ok(Some(status)) => {
                    log!("✅ Mux finished: {}", status);
                    return Ok(());
                }
                Ok(None) => {
                    if start.elapsed() > std::time::Duration::from_secs(120) {
                        let _ = process.kill();
                        return Err(anyhow::anyhow!("Mux timeout"));
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

fn create_staging_texture(device: &ID3D11Device, source_desc: &D3D11_TEXTURE2D_DESC) -> Result<ID3D11Texture2D> {
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

fn extract_texture_data(mapped: &D3D11_MAPPED_SUBRESOURCE, width: u32, height: u32, bytes_per_pixel: usize) -> Result<Vec<u8>> {
    unsafe {
        let ptr = mapped.pData as *const u8;
        let row_pitch = mapped.RowPitch as usize;
        let row_size = width as usize * bytes_per_pixel;
        let mut data = Vec::with_capacity(height as usize * row_size);

        for row in 0..height as usize {
            let start = ptr.add(row * row_pitch);
            data.extend_from_slice(std::slice::from_raw_parts(start, row_size));
        }
        Ok(data)
    }
}

fn build_video_ffmpeg_args(width: u32, height: u32, native_width: u32, native_height: u32, fps: u32, is_hdr: bool, has_tonemap: bool) -> Vec<String> {
    let mut args = vec!["-f", "rawvideo", "-pixel_format"].into_iter().map(String::from).collect::<Vec<_>>();

    if is_hdr && !has_tonemap {
        args.extend(["rgbaf16le", "-color_trc", "smpte2084", "-color_primaries", "bt2020", "-colorspace", "bt2020nc"].iter().map(|s| s.to_string()));
    } else {
        args.push("bgra".to_string());
    }

    args.extend(["-color_range", "pc", "-video_size", &format!("{}x{}", native_width, native_height), "-framerate", &fps.to_string(), "-i", "pipe:0", "-vf"].iter().map(|s| s.to_string()));

    if is_hdr && !has_tonemap {
        args.push(format!("zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=reinhard:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p,scale=w={}:h={}:flags=lanczos", width, height));
    } else {
        args.push(format!("scale=w={}:h={}:flags=lanczos,format=yuv420p", width, height));
    }

    args.extend(["-c:v", "libx264", "-preset", "ultrafast", "-tune", "zerolatency", "-crf", "23", "-profile:v", "high", "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", "-y"].iter().map(|s| s.to_string()));
    args
}

fn write_wav_header(writer: &mut BufWriter<File>, sample_rate: u32, channels: u16) -> Result<()> {
    writer.write_all(b"RIFF")?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    writer.write_all(&(channels * 2).to_le_bytes())?;
    writer.write_all(&16u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&0u32.to_le_bytes())?;
    Ok(())
}

fn finalize_wav_header(path: &PathBuf, samples_written: u64, channels: u16) -> Result<()> {
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    let data_size = (samples_written * channels as u64 * 2) as u32;
    let file_size = data_size + 36;

    file.seek(SeekFrom::Start(4))?;
    file.write_all(&file_size.to_le_bytes())?;
    file.seek(SeekFrom::Start(40))?;
    file.write_all(&data_size.to_le_bytes())?;
    file.flush()?;

    log!("✅ WAV finalized: {} samples, {} bytes", samples_written, data_size);
    Ok(())
}