use crate::{windows::show_updater_window, ALWAYS_ON_TOP, UPDATE_RESULT};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Runtime,
};

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct PinnedFromTrayEvent {
    pinned: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct PinnedFromWindowEvent {
    pinned: bool,
}

impl PinnedFromWindowEvent {
    pub fn pinned(&self) -> &bool {
        &self.pinned
    }
}

pub static TRAY_EVENT_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let check_for_updates_i = MenuItem::with_id(
        app,
        "check_for_updates",
        "Check for Updates...",
        true,
        None::<String>,
    )?;
    if let Some(Some(_)) = *UPDATE_RESULT.lock() {
        check_for_updates_i
            .set_text("💡 New version available!")
            .unwrap();
    }
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<String>)?;
    let hide_i = PredefinedMenuItem::hide(app, Some("Hide"))?;
    let pin_i = MenuItem::with_id(app, "pin", "Pin", true, None::<String>)?;
    if ALWAYS_ON_TOP.load(Ordering::Acquire) {
        pin_i.set_text("Unpin").unwrap();
    }
    let quit_i = PredefinedMenuItem::quit(app, Some("Quit"))?;
    let separator_i = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &check_for_updates_i,
            &show_i,
            &hide_i,
            &pin_i,
            &separator_i,
            &quit_i,
        ],
    )?;

    let tray = app.tray_by_id("tray").unwrap();
    tray.set_menu(Some(menu.clone()))?;
    if TRAY_EVENT_REGISTERED.load(Ordering::Acquire) {
        return Ok(());
    }
    TRAY_EVENT_REGISTERED.store(true, Ordering::Release);
    tray.on_menu_event(move |app, event| match event.id.as_ref() {
        "check_for_updates" => {
            show_updater_window();
        }
        "quit" => app.exit(0),
        _ => {}
    });
    tray.set_show_menu_on_left_click(false)?;

    Ok(())
}
