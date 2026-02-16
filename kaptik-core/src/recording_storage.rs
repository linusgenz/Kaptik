use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use anyhow::Result;
use chrono::{DateTime, Local};
use crate::game_integration::events::GameEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordingData {
    pub metadata: RecordingMetadata,
    pub apm: APMData,
    pub events: Vec<GameEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KDA {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordingMetadata {
    pub recording_id: Uuid,
    pub game_name: String,
    pub character_name: Option<String>,
    pub kda: Option<KDA>,
    pub map_name: Option<String>,
    pub round_number: Option<u32>,
    pub timestamp: DateTime<Local>,
    pub recording_start: u64,
    pub duration_seconds: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct APMData {
    pub series: Vec<(f64, u32)>, // (second, APM)
    pub average_apm: Option<f64>,
    pub peak_apm: Option<u32>,
}

impl RecordingMetadata {
    pub fn with_game_state(
        game_name: String,
        character_name: Option<String>,
        map_name: Option<String>,
        round_number: Option<u32>,
    ) -> Self {
        Self {
            recording_id: Uuid::new_v4(),
            game_name,
            character_name,
            kda: None,
            map_name,
            round_number,
            timestamp: Local::now(),
            recording_start: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            duration_seconds: None,
        }
    }

    pub fn set_kda(&mut self, kda: KDA) {
        self.kda = Some(kda);
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

impl RecordingData {
    pub fn new(metadata: RecordingMetadata) -> Self {
        Self {
            metadata,
            apm: APMData {
                series: Vec::new(),
                average_apm: None,
                peak_apm: None,
            },
            events: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn set_apm_data(&mut self, series: Vec<(f64, u32)>) {
        if !series.is_empty() {
            let total: u32 = series.iter().map(|(_, apm)| apm).sum();
            let average = total as f64 / series.len() as f64;
            let peak = series.iter().map(|(_, apm)| *apm).max().unwrap_or(0);

            self.apm = APMData {
                series,
                average_apm: Some(average),
                peak_apm: Some(peak),
            };
        }
    }

    pub fn finalize(&mut self, duration: f64) {
        self.metadata.duration_seconds = Some(duration);
    }
}

// Storage functions
pub fn save_recording_data<P: AsRef<Path>>(
    data: &RecordingData,
    path: P,
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;
    encode::write(&mut file, data)?;
    Ok(())
}

pub fn load_recording_data<P: AsRef<Path>>(path: P) -> Result<RecordingData> {
    let file = File::open(path)?;
    let data: RecordingData = decode::from_read(file)?;
    Ok(data)
}

pub fn get_recording_path(recording_id: &Uuid) -> Result<PathBuf> {
    let mut dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("No local data dir found"))?;

    dir.push("Kaptik");
    dir.push("recordings");

    fs::create_dir_all(&dir)?;

    Ok(dir.join(format!("{}.msgpack", recording_id)))
}