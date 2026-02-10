#![windows_subsystem = "windows"]

mod app;
mod game_detection;
mod game_integration;
mod ipc;
mod logger;
mod recorder;
mod settings;
mod tray;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    unsafe {
        let _ = windows::Win32::System::Console::AllocConsole();
    }

    let _ = std::process::Command::new("kaptik-ui.exe").spawn();

    app::run().await
}
