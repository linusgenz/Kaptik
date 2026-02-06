mod app;
mod game_detection;
mod game_integration;
mod ipc;
mod logger;
mod recorder;
mod settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
