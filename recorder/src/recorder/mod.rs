use chrono::Local;
use serde::{Deserialize, Serialize};

mod audio_devices;
pub(crate) mod capture;

use crate::game_detection;

#[derive(Debug, Clone)]
pub enum RecorderEvent {
    GameDetected(game_detection::GameProcess),
    GameStopped(String),
    StartRecording(Option<String>),
    StopRecording,
}

#[derive(Debug, Default)]
pub struct RecordingState {
    pub(crate) is_recording: bool,
    pub(crate) current_game: Option<String>,
    pub(crate) active_games: Vec<game_detection::GameProcess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub game_name: String,
    pub character_name: Option<String>,
    pub map_name: Option<String>,
    pub round_number: Option<u32>,
    pub timestamp: chrono::DateTime<Local>,
}

impl RecordingMetadata {
    pub fn new(game_name: String) -> Self {
        Self {
            game_name,
            character_name: None,
            map_name: None,
            round_number: None,
            timestamp: Local::now(),
        }
    }

    pub fn with_game_state(
        game_name: String,
        character_name: Option<String>,
        map_name: Option<String>,
        round_number: Option<u32>,
    ) -> Self {
        Self {
            game_name,
            character_name,
            map_name,
            round_number,
            timestamp: Local::now(),
        }
    }

    pub fn generate_filename(&self) -> String {
        let timestamp = self.timestamp.format("%Y-%m-%d_%H-%M-%S");

        let mut parts = vec![self.game_name.clone()];

        if let Some(ref character) = self.character_name {
            parts.push(character.clone());
        }

        if let Some(ref map) = self.map_name {
            parts.push(map.clone());
        }

        if let Some(round) = self.round_number {
            parts.push(format!("Round{}", round));
        }

        parts.push(timestamp.to_string());

        format!("{}.mp4", parts.join("_"))
    }
}