// events.rs

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};

use crate::domain::game_stats::KDA;

/// A generic in-game event captured during a recording session.
///
/// `RecordingEvent` is game-agnostic and represents actions such as kills,
/// objectives, or round-state changes. It contains a unique identifier,
/// classification, timestamp, and structured event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEvent {
    pub event_id: u32,
    pub event_type: EventType,
    /// Seconds elapsed since recording start.
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
            EventType::Objective => "Objective",
            EventType::Multikill => "Multikill",
            EventType::Special => "Special",
            EventType::RoundStart => "RoundStart",
            EventType::RoundEnd => "RoundEnd",
            EventType::GameEnd => "GameEnd",
            EventType::Custom(s) => s.as_str(),
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

/// Additional contextual information attached to a [`RecordingEvent`].
///
/// Designed to be extensible so integrations can store game-specific data
/// without changing the core event structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    pub kda: Option<KDA>,
    pub map: Option<String>,
    pub team: Option<String>,
    pub extra: HashMap<String, String>,
}

impl RecordingEvent {
    /// Creates a new `RecordingEvent` with empty event data.
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

    /// Sets the actor (event initiator).
    pub fn with_actor(mut self, actor: String) -> Self {
        self.data.actor = Some(actor);
        self
    }

    /// Sets the target (entity affected by the event).
    pub fn with_target(mut self, target: String) -> Self {
        self.data.target = Some(target);
        self
    }

    /// Sets additional participants involved in the event.
    pub fn with_participants(mut self, participants: Vec<String>) -> Self {
        self.data.participants = participants;
        self
    }

    /// Attaches a KDA snapshot to the event metadata.
    pub fn with_kda(mut self, kda: Option<KDA>) -> Self {
        self.data.metadata.kda = kda;
        self
    }

    /// Replaces the entire metadata object.
    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.data.metadata = metadata;
        self
    }
}