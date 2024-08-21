use serde_json::json;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;

#[cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;

use crate::{UpdateResult, APP_HANDLE};

#[allow(dead_code)]
pub const UPDATER_WIN_NAME: &str = "rian_updater";

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateResultEvent(UpdateResult);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateEvent;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct PinnedFromWindowEvent {
    pinned: bool,
}

impl PinnedFromWindowEvent {
    pub fn pinned(&self) -> &bool {
        &self.pinned
    }
}

pub fn show_updater_window() {
    // let window = get_updater_window();
    // window.center().unwrap();
    // window.show().unwrap();

    let handle = APP_HANDLE.get().unwrap();
    CheckUpdateEvent::listen(handle, move |_event| {
        // let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            let builder = handle.updater_builder();
            let updater = builder.build().unwrap();

            match updater.check().await {
                Ok(Some(update)) => {
                    CheckUpdateResultEvent(UpdateResult {
                        version: update.version,
                        current_version: update.current_version,
                        body: update.body,
                    })
                    .emit(handle)
                    .unwrap();
                }
                Ok(None) => {
                    handle
                        .emit(
                            "update_result",
                            json!({
                                "result": None::<UpdateResult>
                            }),
                        )
                        .unwrap();
                }
                Err(_) => {}
            }
            // window_clone.unlisten(event.id)
        });
    });
}

#[allow(dead_code)]
pub fn get_updater_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(UPDATER_WIN_NAME) {
        Some(window) => {
            window.unminimize().unwrap();
            window.set_focus().unwrap();
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                UPDATER_WIN_NAME,
                tauri::WebviewUrl::App("updates".into()),
            )
            .title("Querent R!AN Updater")
            .fullscreen(false)
            .inner_size(500.0, 500.0)
            .min_inner_size(200.0, 200.0)
            .resizable(true)
            .skip_taskbar(true)
            .focused(true);

            return build_window(builder);
        }
    };

    window
}

pub fn build_window<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindow<R> {
    #[cfg(target_os = "macos")]
    {
        let window = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .transparent(true)
            .build()
            .unwrap();

        post_process_window(&window);

        window
    }

    #[cfg(not(target_os = "macos"))]
    {
        let window = builder.transparent(true).decorations(true).build().unwrap();

        post_process_window(&window);

        window
    }
}

pub fn post_process_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    window.set_visible_on_all_workspaces(true).unwrap();

    let _ = window.current_monitor();
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSWindowCollectionBehavior;
        use cocoa::base::id;

        let ns_win = window.ns_window().unwrap() as id;

        unsafe {
            // Disable the automatic creation of "Show Tab Bar" etc menu items on macOS
            NSWindow::setAllowsAutomaticWindowTabbing_(ns_win, cocoa::base::NO);

            let mut collection_behavior = ns_win.collectionBehavior();
            collection_behavior |=
                NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces;

            ns_win.setCollectionBehavior_(collection_behavior);
        }
    }
}
