// process.rs

/// A running game process detected by [`super::GameDetector`].
#[derive(Debug, Clone)]
pub struct GameProcess {
    /// Executable filename (e.g. `"League of Legends.exe"`).
    pub name: String,
    /// Full path to the executable.
    pub exe_path: String,
    /// Windows process ID.
    pub pid: u32,
    /// Title of the game's main window.
    pub window_title: String,
    /// Whether the window currently covers the full monitor.
    pub is_fullscreen: bool,
}