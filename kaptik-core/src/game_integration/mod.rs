use serde::{Deserialize, Serialize};
use std::any::Any;

pub mod events;
pub mod event_storage;
pub mod league_of_legends;
pub(crate) mod manager;

// Re-export wichtiger Typen
pub use events::GameEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameIntegration {
    pub game_name: String,
    pub exe_name: String,
    pub detect_round_state: bool,
    pub detect_character: bool,
    pub api_method: ApiMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiMethod {
    LocalApi { port: u16 },                       // z.B. Valorant, League
    MemoryReading { offsets: Vec<MemoryOffset> }, // z.B. CS:GO
    LogFileParsing { log_path: String },          // Fallback
    GameStateIntegration { config_path: String }, // CS:GO GSI
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOffset {
    pub name: String,
    pub base_address: String,
    pub offsets: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub is_in_round: bool,
    pub round_number: Option<u32>,
    pub character_name: Option<String>,
    pub map_name: Option<String>,
    pub team: Option<String>,
    pub score: Option<Score>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub team1: u32,
    pub team2: u32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            is_in_round: false,
            round_number: None,
            character_name: None,
            map_name: None,
            team: None,
            score: None,
        }
    }
}

/// Trait für alle Spiel-Integrationen
#[async_trait::async_trait]
pub trait GameIntegrationTrait: Send + Sync {
    /// Initialisierung der Integration
    async fn initialize(&mut self) -> anyhow::Result<()>;

    /// Aktuellen Spielzustand abrufen
    async fn get_game_state(&self) -> anyhow::Result<GameState>;

    /// Prüfen ob aktuell eine Runde läuft
    async fn is_in_round(&self) -> bool;

    async fn get_character_name(&self) -> anyhow::Result<Option<String>>;

    fn get_game_name(&self) -> &str;

    async fn get_new_events(&self) -> anyhow::Result<Option<Vec<GameEvent>>> {
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any;
}