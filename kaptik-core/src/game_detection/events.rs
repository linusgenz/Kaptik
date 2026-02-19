// events.rs

use super::process::GameProcess;

/// Events emitted by [`super::GameDetector`] when process state changes.
#[derive(Debug, Clone)]
pub enum DetectionEvent {
    /// A new game process matching the heuristics was found.
    GameStarted(GameProcess),
    /// A previously detected process is no longer running.
    /// Carries the executable name (same as `GameProcess::name`).
    GameStopped(String),
    /// The game window gained focus.
    GameFocused(String),
    /// The game window lost focus.
    GameUnfocused(String),
}
