use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::game_integration::events::GameEvent;

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordingEvents {
    pub game_name: String,

    pub recording_id: Uuid,

    pub recording_start: u64,

    pub events: Vec<GameEvent>,
}

impl RecordingEvents {
    pub fn new(game_name: String, recording_id: Uuid) -> Self {
        Self {
            game_name,
            recording_id,
            recording_start: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            events: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: GameEvent) {
        self.events.push(event);
    }
}

pub fn save_events_msgpack<P: AsRef<Path>>(
    events: &RecordingEvents,
    path: P,
) -> anyhow::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;
    encode::write(&mut file, events)?;
    Ok(())
}

pub fn load_events_msgpack<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<RecordingEvents> {
    let file = File::open(path)?;
    let events: RecordingEvents = decode::from_read(file)?;
    Ok(events)
}

pub fn get_events_path(recording_id: &Uuid) -> anyhow::Result<PathBuf> {
    let mut path = dirs::data_local_dir().expect("Cache dir not found");
    path.push("Kaptik");
    path.push("events");
    path.push(format!("{}.events", recording_id));
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_integration::events::{EventType, GameEvent};

    #[test]
    fn test_event_storage() {
        let recording_id = Uuid::new_v4();
        let mut recording = RecordingEvents::new(
            "League of Legends".to_string(),
            recording_id,
        );

        let event = GameEvent::new(1, EventType::Kill, 45.5, "ChampionKill".to_string())
            .with_actor("TestPlayer".to_string())
            .with_target("Enemy".to_string());

        recording.add_event(event);

        assert_eq!(recording.events.len(), 1);
    }
}