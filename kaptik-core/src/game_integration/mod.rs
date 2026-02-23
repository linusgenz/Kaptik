// game_integration/mod.rs
use std::any::Any;
use crate::domain::events::RecordingEvent;
use crate::domain::game::{GameIdentifier, GameState};

mod games;
pub(crate) mod manager;

use crate::domain::game_stats::KDA;

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

    /// Drain any new in-game events since the last call.  Returns `Ok(None)`
    /// when the integration does not support event streaming.
    async fn get_new_events(&self) -> anyhow::Result<Option<Vec<RecordingEvent>>> {
        Ok(None)
    }

    /// Register a callback that the integration should invoke when the game
    /// session has ended and the recording should be stopped.  The default
    /// implementation is a no-op; integrations that can detect match-end
    /// without the process closing should override this.
    async fn set_stop_recording_callback(&self, _cb: std::sync::Arc<dyn Fn() + Send + Sync>) {}

    fn as_any(&self) -> &dyn Any;
}
