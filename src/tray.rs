use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eyre::Result;
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use once_cell::sync::Lazy;
use rustix::{
    process::{kill_process, Signal},
    thread::Pid,
};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::Icon;
use tray_icon::{
    menu::{
        accelerator::{Code, Modifiers},
        Menu, MenuEvent, MenuItem,
    },
    TrayIconBuilder, TrayIconEvent,
};

use crate::open_app;

pub static FARK_PROCESS: Lazy<Arc<Mutex<Vec<u32>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

const ICON: &[u8] = include_bytes!("../icon.png");

pub fn main() -> Result<()> {
    let icon = load_icon()?;

    let event_loop = EventLoopBuilder::new().build();
    let hotkey_manager = GlobalHotKeyManager::new()?;
    hotkey_manager.register(HotKey::new(Some(Modifiers::ALT), Code::KeyF))?;

    let menu = Menu::new();
    let open = MenuItem::new("Open", true, None);
    let exit = MenuItem::new("Exit", true, None);

    menu.append_items(&[&open, &exit])?;

    let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Fark")
            .with_icon(icon)
            .build()?,
    );

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();
    let hotkeys_channel = GlobalHotKeyEvent::receiver();

    event_loop.run(move |_, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        if let Ok(MenuEvent { id }) = menu_channel.try_recv() {
            if id == open.id() {
                // 打开 app
                open_app();
            } else {
                // 退出
                {
                    let process = FARK_PROCESS.clone();
                    let process = process.lock().unwrap();
                    for i in &*process {
                        let pid = Pid::from_raw(*i as i32).unwrap();
                        kill_process(pid, Signal::Term).ok();
                    }
                }

                tray_icon.take();
                *control_flow = ControlFlow::Exit;
            }
        }

        // FIXME: Does not work, see https://github.com/tauri-apps/tray-icon/issues/104
        if let Ok(event) = tray_channel.try_recv() {
            println!("tray event: {:?}", event);
        }

        if let Ok(event) = hotkeys_channel.try_recv() {
            if event.state() == HotKeyState::Pressed {
                open_app();
            }
        }

        thread::sleep(Duration::from_millis(100));
    });
}

fn load_icon() -> Result<Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image =
            image::load_from_memory_with_format(ICON, image::ImageFormat::Png)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Ok(tray_icon::Icon::from_rgba(
        icon_rgba,
        icon_width,
        icon_height,
    )?)
}
