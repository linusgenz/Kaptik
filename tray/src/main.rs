#![windows_subsystem = "windows"]

mod ipc;
mod ipc_client;

use ipc::*;

use std::sync::Arc;
use tokio::runtime::Runtime;

use tray_icon::{
    menu::{Menu, MenuItem, MenuEvent},
    TrayIconBuilder,
};
use tao::event_loop::{ControlFlow, EventLoop};
use crate::ipc_client::send_command;

fn main() -> anyhow::Result<()> {
    // Tokio Runtime für IPC
    let rt = Arc::new(Runtime::new()?);

    // Tao EventLoop (nur damit die App lebt)
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

    // Tao EventLoop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(menu_event) = menu_events.try_recv() {
            if menu_event.id == open_ui.id() {
                let _ = std::process::Command::new("kaptik-ui.exe").spawn();
            }

            if menu_event.id == exit.id() {
                let rt = rt.clone();

                rt.block_on(async {
                    let _ = send_command(&Command {
                        type_: CommandType::Shutdown,
                        update: None,
                    })
                        .await;
                });

                std::thread::sleep(std::time::Duration::from_millis(100));

                std::process::exit(0);
            }
        }
    });
}