use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u32,

    pub event_type: EventType,

    pub timestamp: f64,

    pub data: EventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Kill,
    Death,
    Assist,
    Objective,
    Multikill,
    Special,
    RoundStart,
    RoundEnd,
    GameEnd,
    Custom(String),
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EventType::Kill => "Kill",
            EventType::Death => "Death",
            EventType::Assist => "Assist",
            _ => {"Unknown"},
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub name: String,

    pub actor: Option<String>,

    pub target: Option<String>,

    pub participants: Vec<String>,

    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    pub kda: Option<(u32, u32, u32)>,

    pub map: Option<String>,

    pub team: Option<String>,

    pub extra: HashMap<String, String>,
}

impl GameEvent {
    pub fn new(event_id: u32, event_type: EventType, timestamp: f64, name: String) -> Self {
        Self {
            event_id,
            event_type,
            timestamp,
            data: EventData {
                name,
                actor: None,
                target: None,
                participants: Vec::new(),
                metadata: EventMetadata::default(),
            },
        }
    }

    pub fn with_actor(mut self, actor: String) -> Self {
        self.data.actor = Some(actor);
        self
    }

    pub fn with_target(mut self, target: String) -> Self {
        self.data.target = Some(target);
        self
    }

    pub fn with_participants(mut self, participants: Vec<String>) -> Self {
        self.data.participants = participants;
        self
    }

    pub fn with_kda(mut self, kills: u32, deaths: u32, assists: u32) -> Self {
        self.data.metadata.kda = Some((kills, deaths, assists));
        self
    }

    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.data.metadata = metadata;
        self
    }
}