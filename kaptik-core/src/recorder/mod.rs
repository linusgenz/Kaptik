use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod audio;
pub(crate) mod capture;

use crate::game_detection;

#[derive(Debug, Clone)]
pub enum RecorderEvent {
    GameDetected(game_detection::GameProcess),
    GameStopped(String),
    StartRecording(),
    StopRecording,
}

#[derive(Debug, Default)]
pub struct RecordingState {
    pub(crate) is_recording: bool,
    pub(crate) current_game: Option<String>,
    pub(crate) active_games: Vec<game_detection::GameProcess>,
}
