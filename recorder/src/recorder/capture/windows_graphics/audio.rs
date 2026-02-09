use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use windows::Win32::Media::Audio::WAVEFORMATEX;

use crate::log;
use crate::recorder::audio::{AudioMixer, AudioSample, WasapiCapture, devices};
use crate::recorder::capture::core::ffmpeg::FfmpegEncoder;
use crate::settings::Settings;

pub(crate) fn init_captures(
    settings: &Settings,
) -> Result<(
    Option<Arc<Mutex<WasapiCapture>>>,
    Option<WAVEFORMATEX>,
    Option<Arc<Mutex<WasapiCapture>>>,
    Option<WAVEFORMATEX>,
)> {
    let (game_audio_capture, game_audio_format) = if settings.game_audio || settings.system_sounds {
        if let Ok(device_id) = devices::get_game_audio_device(settings) {
            let capture = WasapiCapture::new(&device_id, true)?; // Loopback
            let format = *capture.get_format();
            (Some(Arc::new(Mutex::new(capture))), Some(format))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let (microphone_capture, microphone_format) = if settings.microphone {
        if let Ok(device_id) = devices::get_microphone_device(settings) {
            let capture = WasapiCapture::new(&device_id, false)?; // No loopback
            let format = *capture.get_format();
            (Some(Arc::new(Mutex::new(capture))), Some(format))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    Ok((
        game_audio_capture,
        game_audio_format,
        microphone_capture,
        microphone_format,
    ))
}

pub(crate) fn start_audio_tasks(
    encoder: Arc<Mutex<Option<FfmpegEncoder>>>,
    game_audio_capture: &Option<Arc<Mutex<WasapiCapture>>>,
    microphone_capture: &Option<Arc<Mutex<WasapiCapture>>>,
    output_volume: u8,
    microphone_volume: u8,
    audio_shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let has_game = game_audio_capture.is_some();
    let has_mic = microphone_capture.is_some();

    if !has_game && !has_mic {
        return Ok(());
    }

    let mixer = Arc::new(AudioMixer::new(
        has_game,
        has_mic,
        output_volume,
        microphone_volume,
    ));

    if let Some(game_audio) = game_audio_capture {
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

    if let Some(mic) = microphone_capture {
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
    let encoder_clone = Arc::clone(&encoder);
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
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        log!("Audio mixer task completed");
    });

    Ok(())
}
