use std::path::PathBuf;

use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use crate::log;
use crate::recorder::RecorderEvent;
use tokio::sync::mpsc;
use tray_icon::Icon;

fn load_icon() -> anyhow::Result<Icon> {
    let image = if cfg!(debug_assertions) {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("..");
        path.push("assets");
        path.push("kaptik_logo_transparent32x32.png");

        image::open(&path)?
    } else {
        image::load_from_memory(include_bytes!(
            "../../assets/kaptik_logo_transparent32x32.png"
        ))?
    }
        .into_rgba8();

    let (width, height) = image.dimensions();

    Ok(Icon::from_rgba(
        image.into_raw(),
        width,
        height,
    )?)
}

pub fn run_tray(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
) -> anyhow::Result<()> {
    let event_loop = EventLoop::new();

    let open_ui = MenuItem::new("Open UI", true, None);
    let exit = MenuItem::new("Exit", true, None);

    let menu = Menu::new();
    menu.append(&open_ui)?;
    menu.append(&exit)?;

    let menu_events = MenuEvent::receiver();

    let icon = match load_icon() {
        Ok(icon) => icon,
        Err(err) => {
            log!("❌ Failed to load tray icon: {err:?}");

            loop {
                std::thread::park();
            }
        }
    };

    let _tray = TrayIconBuilder::new()
        .with_tooltip("Kaptik Recorder")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()?;

    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(menu_event) = menu_events.try_recv() {
            if menu_event.id == open_ui.id() {
                let _ = std::process::Command::new("kaptik-ui.exe").spawn();
            }

            if menu_event.id == exit.id() {
                log!("🛑 Shutdown command received");

                let _ = event_tx.send(RecorderEvent::StopRecording);
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
