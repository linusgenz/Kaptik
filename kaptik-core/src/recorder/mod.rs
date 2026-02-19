
mod audio;
pub(crate) mod capture;

use crate::game_detection::GameProcess;

#[derive(Debug, Clone)]
pub enum RecorderEvent {
    GameDetected(GameProcess),
    GameStopped(String),
    StartRecording,
    StopRecording,
}

#[derive(Debug, Default)]
pub struct GameTracker {
    /// The executable name of the game currently being recorded, if any.
    pub(crate) current_game: Option<String>,
    /// All game processes that are currently detected as running.
    pub(crate) active_games: Vec<GameProcess>,
}

impl GameTracker {
    pub fn has_active_games(&self) -> bool {
        !self.active_games.is_empty()
    }

    pub fn first_active_game(&self) -> Option<&GameProcess> {
        self.active_games.first()
    }
}