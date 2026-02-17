// game_integration/mod.rs
use serde::{Deserialize, Serialize};
use std::any::Any;

pub mod events;
mod games;
pub(crate) mod manager;

use crate::domain::game_stats::KDA;
pub use events::GameEvent;

/// Always construct via [`GameName::from_display`] or [`GameName::from_window_title`]
/// so the slug is derived consistently.
/// Both representations of the game name.
/// - `game_name.display`   → shown in the UI  (e.g. `"League of Legends"`)
/// - `game_name.file_slug` → used in filenames (e.g. `"League_of_Legends"`)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameName {
    /// Pretty name shown in the UI (may contain spaces and Unicode).
    pub display: String,
    /// Filesystem-safe slug: spaces replaced by `_`, illegal chars stripped.
    pub file_slug: String,
}

impl GameName {
    /// Build from a known, clean display name (e.g. from an integration's
    /// `get_game_name()`).  The slug is derived automatically.
    pub fn from_display(display: impl Into<String>) -> Self {
        let display = display.into();
        let file_slug = Self::slugify(&display);
        Self { display, file_slug }
    }

    /// Build from a raw window title when no integration name is available.
    /// Strips common noise characters before slugifying, mirroring the old
    /// `extract_game_name` behaviour.
    pub fn from_window_title(title: &str) -> Self {
        let display = title
            .split(&['-', '(', ')', '™', '®'][..])
            .next()
            .unwrap_or(title)
            .trim()
            .to_string();
        let file_slug = Self::slugify(&display);
        Self { display, file_slug }
    }

    /// Convert a display name to a filesystem-safe slug:
    /// - Spaces → `_`
    /// - Characters illegal on Windows/macOS/Linux stripped: `\ / : * ? " < > |`
    /// - Leading/trailing whitespace removed first
    fn slugify(s: &str) -> String {
        s.trim()
            .chars()
            .map(|c| match c {
                ' ' => '_',
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c => c,
            })
            .collect::<String>()
            .split('_')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    }
}

impl std::fmt::Display for GameName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

/// Identifies a game by the executable that triggers detection **and** the
/// human-readable display name reported by the integration.
///
/// Using a struct instead of a bare `String` prevents the "activated by
/// `league of legends.exe`" vs. `"League of Legends"` mismatch that existed
/// before and makes intent clear at every call-site.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameIdentifier {
    /// Lower-cased executable name used as the HashMap key (e.g. `"league of legends.exe"`).
    pub exe_key: String,
    /// Human-readable name from the integration (e.g. `"League of Legends"`).
    pub display_name: String,
}

impl GameIdentifier {
    /// Build a [`GameIdentifier`] from raw strings, normalising the exe key to
    /// lowercase so callers don't have to think about casing.
    pub fn new(exe_name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            exe_key: exe_name.into().to_lowercase(),
            display_name: display_name.into(),
        }
    }

    /// Convenience constructor that derives the display name from the exe name
    /// by stripping the `.exe` suffix and title-casing the result.  Used when
    /// no integration is available for the detected game.
    pub fn from_exe(exe_name: impl Into<String>) -> Self {
        let raw = exe_name.into();
        let display = raw
            .trim_end_matches(".exe")
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            exe_key: raw.to_lowercase(),
            display_name: display,
        }
    }

    /// Convert to a [`GameName`], using the integration's display name as the
    /// source of truth for both representations.
    pub fn to_game_name(&self) -> GameName {
        GameName::from_display(&self.display_name)
    }
}

impl std::fmt::Display for GameIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub team1: u32,
    pub team2: u32,
}

/// Snapshot of the game state at a particular point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameState {
    pub is_in_round: bool,
    pub round_number: Option<u32>,
    pub character_name: Option<String>,
    pub kda: Option<KDA>,
    pub map_name: Option<String>,
    pub team: Option<String>,
    pub score: Option<Score>,
}

impl GameState {
    /// Returns `true` if this snapshot contains enough data to be worth
    /// persisting (avoids overwriting a richer cached state with an empty one).
    pub fn is_meaningful(&self) -> bool {
        self.is_in_round || self.kda.is_some()
    }
}

// ─────────────────────────────────────────────
// Integration trait
// ─────────────────────────────────────────────

/// Per-game integration contract.
///
/// Implementations are stored behind `Box<dyn GameIntegrationTrait>` inside
/// the manager, so all methods must be object-safe (hence `async_trait`).
#[async_trait::async_trait]
pub trait GameIntegrationTrait: Send + Sync {
    /// Called once when the game is first detected.  Implementations should
    /// connect to the game's local API and populate any internal state (e.g.
    /// the active player name).
    async fn initialize(&mut self) -> anyhow::Result<()>;

    /// Returns the current game state.  Must not block for more than a few
    /// hundred milliseconds – it is polled every second from the monitoring
    /// task.
    async fn get_game_state(&self) -> anyhow::Result<GameState>;

    /// Lightweight in-round check used by the auto-record wait loop.  Prefer
    /// this over `get_game_state().is_in_round` when only the boolean is
    /// needed.
    async fn is_in_round(&self) -> bool;

    /// Returns the current character name, or `None` if not yet
    /// determinable or not available.
    async fn get_character_name(&self) -> anyhow::Result<Option<String>>;

    /// Human-readable game name used for display and logging (e.g. `"League of
    /// Legends"`).  Must be cheap to call (no I/O).
    fn get_game_name(&self) -> &str;

    /// Build a [`GameIdentifier`] for this integration using the provided
    /// executable name.  The default implementation delegates to
    /// [`GameIdentifier::new`] and is correct for almost every case.
    fn identifier(&self, exe_name: &str) -> GameIdentifier {
        GameIdentifier::new(exe_name, self.get_game_name())
    }

    /// Synchronous KDA accessor – may return `None` if the value is only
    /// available asynchronously (the async path via [`get_game_state`] is then
    /// preferred).
    fn get_kda(&self) -> Option<KDA>;

    /// Drain any new in-game events since the last call.  Returns `Ok(None)`
    /// when the integration does not support event streaming.
    async fn get_new_events(&self) -> anyhow::Result<Option<Vec<GameEvent>>> {
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any;
}
