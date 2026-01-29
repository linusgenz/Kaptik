use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            dark_mode: false,
            video_path: "".to_string(),
            resolution: Resolution::R1080p,
            fps_limit: Fps::Fps60,
            game_audio: true,
            microphone: true,
            system_sounds: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Resolution {
    R720p,
    R1080p,
    R1440p,
    R4K,
    Custom,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Fps {
    Fps30,
    Fps60,
    Fps120,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            dark_mode: false,
            video_path: "".to_string(),
            resolution: Resolution::R1080p,
            fps_limit: Fps::Fps60,
            game_audio: true,
            microphone: true,
            system_sounds: false,
        }
    }

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
            println!("Loaded settings file: {}", path.display());
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
                self.resolution = match value {
                    "R720p" => Resolution::R720p,
                    "R1080p" => Resolution::R1080p,
                    "R1440p" => Resolution::R1440p,
                    "R4K" => Resolution::R4K,
                    "Custom" => Resolution::Custom,
                    _ => return Err(format!("Invalid resolution: {}", value)),
                };
            }
            "fps_limit" => {
                self.fps_limit = match value {
                    "Fps30" => Fps::Fps30,
                    "Fps60" => Fps::Fps60,
                    "Fps120" => Fps::Fps120,
                    _ => return Err(format!("Invalid FPS: {}", value)),
                };
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
            _ => return Err(format!("Unknown setting key: {}", key)),
        }
        Ok(())
    }
}
