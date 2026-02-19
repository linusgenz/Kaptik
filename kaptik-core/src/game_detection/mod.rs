// mod.rs

mod detector;
mod events;
mod heuristics;
mod process;

pub use detector::GameDetector;
pub use events::DetectionEvent;
pub use process::GameProcess;
