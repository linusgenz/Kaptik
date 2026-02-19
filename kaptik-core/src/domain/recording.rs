// recording.rs

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::events::RecordingEvent;
use crate::domain::game::GameName;
use crate::domain::game_stats::{GameOutcome, KDA};

/// The complete persisted data for a single recording session.
/// Serialized to MessagePack via the `storage` module.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordingData {
    pub metadata: RecordingMetadata,
    pub apm: APMData,
    pub events: Vec<RecordingEvent>,
}

impl RecordingData {
    pub fn new(metadata: RecordingMetadata) -> Self {
        Self {
            metadata,
            apm: APMData::default(),
            events: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: RecordingEvent) {
        self.events.push(event);
    }

    pub fn set_apm_data(&mut self, series: Vec<(f64, u32)>) {
        if series.is_empty() {
            return;
        }

        let total: u32 = series.iter().map(|(_, apm)| apm).sum();
        let average = total as f64 / series.len() as f64;
        let peak = series.iter().map(|(_, apm)| *apm).max().unwrap_or(0);

        self.apm = APMData {
            series,
            average_apm: Some(average),
            peak_apm: Some(peak),
        };
    }

    /// Set the final duration once recording has stopped.
    pub fn finalize(&mut self, duration_secs: f64) {
        self.metadata.duration_seconds = Some(duration_secs);
    }
}

/// Metadata attached to a recording. Embedded inside [`RecordingData`] and
/// also made available to the capture strategy during recording so it can
/// generate filenames before stopping.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordingMetadata {
    pub recording_id: Uuid,
    pub game_name: GameName,
    pub game_mode: Option<String>,
    pub game_outcome: Option<GameOutcome>,
    pub character_name: Option<String>,
    pub kda: Option<KDA>,
    pub map_name: Option<String>,
    pub round_number: Option<u32>,
    pub timestamp: DateTime<Local>,
    /// Unix timestamp of when the recording started (seconds).
    pub recording_start: u64,
    pub duration_seconds: Option<f64>,
}

impl RecordingMetadata {
    /// Creates metadata with only the game name; all optional fields are `None`.
    pub fn new(game_name: GameName) -> Self {
        Self {
            recording_id: Uuid::new_v4(),
            game_name,
            character_name: None,
            map_name: None,
            game_mode: None,
            game_outcome: None,
            kda: None,
            round_number: None,
            timestamp: Local::now(),
            recording_start: unix_now(),
            duration_seconds: None,
        }
    }

    /// Creates metadata pre-populated from a [`GameState`] snapshot.
    pub fn with_game_state(
        game_name: GameName,
        character_name: Option<String>,
        map_name: Option<String>,
        game_mode: Option<String>,
        round_number: Option<u32>,
    ) -> Self {
        Self {
            recording_id: Uuid::new_v4(),
            game_name,
            game_mode,
            game_outcome: None,
            character_name,
            kda: None,
            map_name,
            round_number,
            timestamp: Local::now(),
            recording_start: unix_now(),
            duration_seconds: None,
        }
    }

    pub fn set_kda(&mut self, kda: Option<KDA>) {
        self.kda = kda;
    }

    pub fn set_game_outcome(&mut self, game_outcome: Option<GameOutcome>) {
        self.game_outcome = game_outcome;
    }

    /// Generate a filesystem-safe filename for the recording video.
    /// Uses the slug representation of the game name.
    pub fn generate_filename(&self) -> String {
        let timestamp = self.timestamp.format("%Y-%m-%d_%H-%M-%S");

        let mut parts = vec![self.game_name.file_slug.clone()];

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

/// Actions-per-minute timeseries and summary statistics for a recording.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct APMData {
    /// `(elapsed_seconds, apm)` pairs, one per second of the recording.
    pub series: Vec<(f64, u32)>,
    pub average_apm: Option<f64>,
    pub peak_apm: Option<u32>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}