use std::sync::Arc;

use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use crate::log;
use crate::recorder::RecorderEvent;
use tokio::sync::mpsc;

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

    let _tray = TrayIconBuilder::new()
        .with_tooltip("Kaptik Recorder")
        .with_menu(Box::new(menu))
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
