use crate::log;
use crate::recorder::capture::core::tonemap::{HdrNitsMode, TonemapAlgorithm};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub static SETTINGS: Lazy<Arc<RwLock<Settings>>> =
    Lazy::new(|| Arc::new(RwLock::new(Settings::load())));

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub dark_mode: bool,
    pub video_path: String,
    pub resolution: Resolution,
    pub fps_limit: Fps,
    pub game_audio: bool,
    pub microphone: bool,
    pub system_sounds: bool,
    pub auto_record: bool,

    pub selected_game_audio_device: String,
    pub selected_microphone_device: String,
    pub output_volume: u8,
    pub microphone_volume: u8,

    pub tonemap_algorithm: TonemapAlgorithm,
    pub hdr_nits_mode: HdrNitsMode,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            dark_mode: false,
            video_path: "".to_string(),
            resolution: Resolution::Resolution1080p,
            fps_limit: Fps::Fps60,
            game_audio: true,
            microphone: true,
            system_sounds: false,
            auto_record: true,
            selected_game_audio_device: "".to_string(),
            selected_microphone_device: "".to_string(),
            output_volume: 50,
            microphone_volume: 50,
            tonemap_algorithm: TonemapAlgorithm::default(),
            hdr_nits_mode: HdrNitsMode::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
pub enum Resolution {
    Resolution720p,
    Resolution1080p,
    Resolution1440p,
    Resolution4K,
    ResolutionSource,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Fps {
    Fps30 = 0,
    Fps60 = 1,
    Fps120 = 2,
}

impl Settings {
    pub fn path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap();
        dir.push("Kaptik");
        fs::create_dir_all(&dir).unwrap();
        dir.push("settings.toml");
        dir
    }

    pub fn load() -> Self {
        let path = Settings::path();

        if path.exists() {
            log!("Loaded settings file: {}", path.display());
            let content = fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_else(|_| Settings::default())
        } else {
            Settings::default()
        }
    }

    pub fn save(&self) {
        let toml_str = toml::to_string_pretty(&self).unwrap();
        fs::write(Settings::path(), toml_str).unwrap();
    }

    pub fn update(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "dark_mode" => {
                self.dark_mode = value
                    .parse::<bool>()
                    .map_err(|_| format!("Invalid boolean for {}: {}", key, value))?;
            }
            "video_path" => {
                self.video_path = value.to_string();
            }
            "resolution" => {
                self.resolution = serde_plain::from_str(value)
                    .map_err(|_| format!("Invalid resolution: {}", value))?;
            }
            "fps_limit" => {
                self.fps_limit =
                    serde_plain::from_str(value).map_err(|_| format!("Invalid FPS: {}", value))?;
            }
            "game_audio" => {
                self.game_audio = value
                    .parse::<bool>()
                    .map_err(|_| format!("Invalid boolean for {}: {}", key, value))?;
            }
            "microphone" => {
                self.microphone = value
                    .parse::<bool>()
                    .map_err(|_| format!("Invalid boolean for {}: {}", key, value))?;
            }
            "system_sounds" => {
                self.system_sounds = value
                    .parse::<bool>()
                    .map_err(|_| format!("Invalid boolean for {}: {}", key, value))?;
            }
            "tonemap_algorithm" => {
                self.tonemap_algorithm = serde_plain::from_str(value)
                    .map_err(|_| format!("Invalid tonemap algorithm: {}", value))?;
            }
            "hdr_nits_mode" => {
                self.hdr_nits_mode = serde_plain::from_str(value)
                    .map_err(|_| format!("Invalid HDR nits mode: {}", value))?;
            }
            "output_device" => {
                self.selected_game_audio_device = value.to_string();
            }
            "input_device" => {
                self.selected_microphone_device = value.to_string();
            }
            "output_volume" => {
                self.output_volume = value
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid integer for {}: {}", key, value))?;
            }
            "microphone_volume" => {
                self.microphone_volume = value
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid integer for {}: {}", key, value))?;
            }
            _ => return Err(format!("Unknown setting key: {}", key)),
        }
        Ok(())
    }
}

pub async fn get_setting<F, R>(f: F) -> R
where
    F: FnOnce(&Settings) -> R,
{
    let settings = SETTINGS.read().await;
    f(&settings)
}

pub async fn update_setting<F>(f: F)
where
    F: FnOnce(&mut Settings),
{
    let mut settings = SETTINGS.write().await;
    f(&mut settings);
    settings.save();
}
