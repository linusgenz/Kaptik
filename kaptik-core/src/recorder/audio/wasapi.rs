use crate::log;
use anyhow::Result;
use std::os::raw::c_void;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use windows::Win32::Media::Audio::*;
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::WAVE_FORMAT_IEEE_FLOAT;
use windows::Win32::System::Com::*;
use windows::core::*;

pub struct AudioSample {
    pub data: Vec<u8>,
    pub timestamp: std::time::Instant,
}

pub struct WasapiCapture {
    format: WAVEFORMATEX,
    device_format: WAVEFORMATEX,
    device_id: String,
    is_loopback: bool,
    stop_tx: Option<std_mpsc::Sender<()>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl WasapiCapture {
    pub fn new(device_id: &str, is_loopback: bool) -> Result<Self> {
        // Get real format from device
        let device_format = unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let flow = if is_loopback { eRender } else { eCapture };

            let device: IMMDevice = if device_id.is_empty() {
                enumerator.GetDefaultAudioEndpoint(flow, eConsole)?
            } else {
                let device_id_wide: Vec<u16> = device_id.encode_utf16().chain(Some(0)).collect();
                enumerator.GetDevice(PCWSTR(device_id_wide.as_ptr()))?
            };

            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let format_ptr = client.GetMixFormat()?;
            let format = *format_ptr;
            CoTaskMemFree(Some(format_ptr as *const c_void));

            CoUninitialize();

            format
        };

        let sample_rate = device_format.nSamplesPerSec;
        let channels = device_format.nChannels;
        let bits = device_format.wBitsPerSample;
        let tag = device_format.wFormatTag;

        log!(
            "[WASAPI] Format: {} Hz, {} channels, {} bits, format tag: {}",
            sample_rate,
            channels,
            bits,
            tag
        );

        let output_format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 2,
            nSamplesPerSec: device_format.nSamplesPerSec,
            nAvgBytesPerSec: device_format.nSamplesPerSec * 2 * 2,
            nBlockAlign: 2 * 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };

        let out_rate = output_format.nSamplesPerSec;
        let out_channels = output_format.nChannels;

        log!(
            "[WASAPI] Output Format: {} Hz, {} channels, 16 bits PCM (Downmix from {} channels)",
            out_rate,
            out_channels,
            channels
        );

        Ok(Self {
            format: output_format,
            device_format,
            device_id: device_id.to_string(),
            is_loopback,
            stop_tx: None,
            thread_handle: None,
        })
    }

    pub fn start(&mut self, sender: mpsc::UnboundedSender<AudioSample>) -> Result<()> {
        let (stop_tx, stop_rx) = std_mpsc::channel();
        let device_id = self.device_id.clone();
        let is_loopback = self.is_loopback;
        let device_format = self.device_format;

        let handle = std::thread::spawn(move || unsafe {
            log!("[WASAPI] Capture thread started");

            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
            let flow = if is_loopback { eRender } else { eCapture };

            let device: IMMDevice = if device_id.is_empty() {
                match enumerator.GetDefaultAudioEndpoint(flow, eConsole) {
                    Ok(dev) => dev,
                    Err(e) => {
                        log!(
                            "⚠️ GetDefaultAudioEndpoint failed (flow={:?}): {:?}",
                            flow,
                            e
                        );
                        return;
                    }
                }
            } else {
                let device_id_wide: Vec<u16> = device_id.encode_utf16().chain(Some(0)).collect();
                match enumerator.GetDevice(PCWSTR(device_id_wide.as_ptr())) {
                    Ok(dev) => dev,
                    Err(e) => {
                        log!("⚠️ GetDevice failed for ID '{}': {:?}", device_id, e);
                        return;
                    }
                }
            };

            let result =
                WasapiCapture::capture_thread(device, sender, stop_rx, is_loopback, device_format);

            if let Err(e) = result {
                log!("⚠️ WASAPI Thread Fehler: {}", e);
            }

            CoUninitialize();
            log!("[WASAPI] Capture Thread beendet");
        });

        self.stop_tx = Some(stop_tx);
        self.thread_handle = Some(handle);
        Ok(())
    }

    unsafe fn capture_thread(
        device: IMMDevice,
        sender: mpsc::UnboundedSender<AudioSample>,
        stop_rx: std_mpsc::Receiver<()>,
        is_loopback: bool,
        device_format: WAVEFORMATEX,
    ) -> Result<()> { unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let format_ptr = client.GetMixFormat()?;

        let flags = if is_loopback {
            AUDCLNT_STREAMFLAGS_LOOPBACK
        } else {
            0
        };

        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            flags,
            10_000_000, // 1 sec
            0,
            format_ptr,
            None,
        )?;

        let capture_client: IAudioCaptureClient = client.GetService()?;

        client.Start()?;

        // Format-Info für Konvertierung
        let is_float = device_format.wFormatTag == WAVE_FORMAT_IEEE_FLOAT as u16
            || device_format.wFormatTag == WAVE_FORMAT_EXTENSIBLE as u16;
        let bytes_per_sample = (device_format.wBitsPerSample / 8) as usize;
        let input_channels = device_format.nChannels as usize;

        let format_tag = device_format.wFormatTag;

        log!(
            "Audio format: tag={}, Float={}, {} input channels → 2 stereo",
            format_tag,
            is_float,
            input_channels
        );

        loop {
            match stop_rx.try_recv() {
                Ok(_) | Err(std_mpsc::TryRecvError::Disconnected) => {
                    log!("🛑 Stop signal received");
                    break;
                }
                Err(std_mpsc::TryRecvError::Empty) => {}
            }

            if let Err(e) = Self::capture_loop(
                &capture_client,
                &sender,
                is_float,
                bytes_per_sample,
                input_channels,
            ) {
                log!("⚠️ WASAPI Capture error: {}", e);
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        // Cleanup
        client.Stop()?;
        CoTaskMemFree(Some(format_ptr as *const c_void));
        CoUninitialize();

        Ok(())
    }}

    fn capture_loop(
        capture_client: &IAudioCaptureClient,
        sender: &mpsc::UnboundedSender<AudioSample>,
        is_float: bool,
        bytes_per_sample: usize,
        input_channels: usize,
    ) -> Result<()> {
        unsafe {
            // WICHTIG: Nur EINMAL GetNextPacketSize callen pro Loop-Iteration
            let packet_length = capture_client.GetNextPacketSize()?;

            // Wenn kein Packet verfügbar ist, sofort returnen (nicht in while-loop!)
            if packet_length == 0 {
                return Ok(());
            }

            // NUR das eine verfügbare Packet holen
            let mut buffer: *mut u8 = std::ptr::null_mut();
            let mut frames_available: u32 = 0;
            let mut flags: u32 = 0;

            capture_client.GetBuffer(&mut buffer, &mut frames_available, &mut flags, None, None)?;

            if frames_available > 0 {
                let data = if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                    // Stille: 16-bit PCM Stereo zeros
                    vec![0u8; frames_available as usize * 2 * 2]
                } else if is_float {
                    // Float32 → Int16 + Downmix zu Stereo
                    Self::convert_float32_to_stereo_int16(
                        buffer,
                        frames_available as usize,
                        input_channels,
                    )
                } else if bytes_per_sample == 2 {
                    // 16-bit PCM → Downmix zu Stereo
                    Self::convert_pcm16_to_stereo(buffer, frames_available as usize, input_channels)
                } else {
                    // Anderes Format: Fallback auf Stille
                    log!(
                        "⚠️ Unbekanntes Audio-Format: {} bytes/sample",
                        bytes_per_sample
                    );
                    vec![0u8; frames_available as usize * 2 * 2]
                };

                let sample = AudioSample {
                    data,
                    timestamp: std::time::Instant::now(),
                };

                if sender.send(sample).is_err() {
                    log!("⚠️ Audio channel disconnected");
                    return Ok(());
                }
            }

            capture_client.ReleaseBuffer(frames_available)?;

            Ok(())
        }
    }

    unsafe fn convert_float32_to_stereo_int16(
        buffer: *mut u8,
        frames: usize,
        input_channels: usize,
    ) -> Vec<u8> { unsafe {
        let float_buffer =
            std::slice::from_raw_parts(buffer as *const f32, frames * input_channels);

        let mut output = Vec::with_capacity(frames * 2 * 2);

        for frame_idx in 0..frames {
            let base = frame_idx * input_channels;

            let (left, right) = match input_channels {
                1 => {
                    // Mono → beide Kanäle gleich
                    (float_buffer[base], float_buffer[base])
                }
                2 => {
                    // Nothing to do
                    (float_buffer[base], float_buffer[base + 1])
                }
                3 | 4 | 5 | 6 | 7 | 8 => {
                    // Multichannel (ITU-R BS.775 Standard)
                    let fl = float_buffer.get(base).copied().unwrap_or(0.0);
                    let fr = float_buffer.get(base + 1).copied().unwrap_or(0.0);
                    let fc = float_buffer.get(base + 2).copied().unwrap_or(0.0);
                    let lfe = float_buffer.get(base + 3).copied().unwrap_or(0.0);
                    let bl = float_buffer.get(base + 4).copied().unwrap_or(0.0);
                    let br = float_buffer.get(base + 5).copied().unwrap_or(0.0);

                    let left = fl + (fc * 0.707) + (bl * 0.707) + (lfe * 0.5);
                    let right = fr + (fc * 0.707) + (br * 0.707) + (lfe * 0.5);

                    // Normalize
                    let max_val = left.abs().max(right.abs());
                    if max_val > 1.0 {
                        (left / max_val, right / max_val)
                    } else {
                        (left, right)
                    }
                }
                _ => {
                    log!("⚠️ Unbekannte Channel-Config: {} Kanäle", input_channels);
                    (0.0, 0.0)
                }
            };

            // Float32 → Int16 mit Clipping
            let left_i16 = (left.clamp(-1.0, 1.0) * 32767.0) as i16;
            let right_i16 = (right.clamp(-1.0, 1.0) * 32767.0) as i16;

            output.extend_from_slice(&left_i16.to_le_bytes());
            output.extend_from_slice(&right_i16.to_le_bytes());
        }

        output
    }}

    unsafe fn convert_pcm16_to_stereo(
        buffer: *mut u8,
        frames: usize,
        input_channels: usize,
    ) -> Vec<u8> { unsafe {
        let int16_buffer =
            std::slice::from_raw_parts(buffer as *const i16, frames * input_channels);

        let mut output = Vec::with_capacity(frames * 2 * 2);

        for frame_idx in 0..frames {
            let base = frame_idx * input_channels;

            let (left, right) = match input_channels {
                1 => (int16_buffer[base], int16_buffer[base]),
                2 => (int16_buffer[base], int16_buffer[base + 1]),
                3 | 4 | 5 | 6 | 7 | 8 => {
                    let fl = int16_buffer.get(base).copied().unwrap_or(0) as f32 / 32767.0;
                    let fr = int16_buffer.get(base + 1).copied().unwrap_or(0) as f32 / 32767.0;
                    let fc = int16_buffer.get(base + 2).copied().unwrap_or(0) as f32 / 32767.0;
                    let lfe = int16_buffer.get(base + 3).copied().unwrap_or(0) as f32 / 32767.0;
                    let bl = int16_buffer.get(base + 4).copied().unwrap_or(0) as f32 / 32767.0;
                    let br = int16_buffer.get(base + 5).copied().unwrap_or(0) as f32 / 32767.0;

                    let left_f = fl + (fc * 0.707) + (bl * 0.707) + (lfe * 0.5);
                    let right_f = fr + (fc * 0.707) + (br * 0.707) + (lfe * 0.5);

                    let max_val = left_f.abs().max(right_f.abs());
                    let (left_norm, right_norm) = if max_val > 1.0 {
                        (left_f / max_val, right_f / max_val)
                    } else {
                        (left_f, right_f)
                    };

                    let left_i16 = (left_norm * 32767.0) as i16;
                    let right_i16 = (right_norm * 32767.0) as i16;

                    (left_i16, right_i16)
                }
                _ => {
                    log!(
                        "⚠️ Unknown PCM16 channel configuration: {} channels",
                        input_channels
                    );
                    (0, 0)
                }
            };

            output.extend_from_slice(&left.to_le_bytes());
            output.extend_from_slice(&right.to_le_bytes());
        }

        output
    }}

    pub fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());

            if let Some(handle) = self.thread_handle.take() {
                handle.join().ok();
            }
        }
        Ok(())
    }

    pub fn get_format(&self) -> &WAVEFORMATEX {
        &self.format
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
