#![windows_subsystem = "windows"]

use kaptik_core::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    unsafe {
        let _ = windows::Win32::System::Console::AllocConsole();
    }

    let _ = std::process::Command::new("kaptik-ui.exe").spawn();

    app::run().await
}
