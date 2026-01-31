use std::process::Command;
use regex::Regex;
use crate::settings::Settings;

pub fn list_audio_devices() -> Vec<String> {
    let output = Command::new("ffmpeg")
        .args(&["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
        .output()
        .unwrap_or_else(|_| panic!("FFmpeg not found"));

    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut devices = Vec::new();
    let re = Regex::new(r#""(.+?)" \(audio\)"#).unwrap();
    for cap in re.captures_iter(&stderr) {
        devices.push(cap[1].to_string());
    }
    devices
}

pub fn get_game_audio_device(settings: &Settings) -> Option<String> {
    let devices = list_audio_devices();

    if let Some(ref saved) = settings.selected_game_audio_device {
        if devices.iter().any(|d| d == saved) {
            return Some(saved.clone());
        }
    }

    for name in &devices {
        let name_lower = name.to_lowercase();
        if name_lower.contains("stereo mix") || name_lower.contains("virtual-audio") || name_lower.contains("loopback") {
            return Some(name.clone());
        }
    }

    devices.first().cloned()
}

pub fn get_microphone_device(settings: &Settings) -> Option<String> {
    let devices = list_audio_devices();

    if let Some(ref saved) = settings.selected_microphone_device {
        if devices.iter().any(|d| d == saved) {
            return Some(saved.clone());
        }
    }

    for name in &devices {
        if name.to_lowercase().contains("microphone") {
            return Some(name.clone());
        }
    }

    devices.first().cloned()
}
