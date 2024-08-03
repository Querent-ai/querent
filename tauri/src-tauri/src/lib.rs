use node::service::serve_quester_without_servers;
use node::QuerentServices;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use proto::NodeConfig;
use specta_typescript::Typescript;
use std::env;
use std::sync::atomic::AtomicBool;
use sysinfo::{CpuExt, System, SystemExt};
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;
use tauri_specta::Builder;
mod windows;
use windows::CheckUpdateEvent;
use windows::CheckUpdateResultEvent;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static CPU_VENDOR: Mutex<String> = Mutex::new(String::new());
pub static QUERENT_SERVICES: OnceCell<QuerentServices> = OnceCell::new();
pub static UPDATE_RESULT: Mutex<Option<Option<UpdateResult>>> = Mutex::new(None);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    version: String,
    current_version: String,
    body: Option<String>,
}

#[tauri::command]
#[specta::specta]
fn get_update_result() -> (bool, Option<UpdateResult>) {
    if UPDATE_RESULT.lock().is_none() {
        return (false, None);
    }
    return (true, UPDATE_RESULT.lock().clone().unwrap());
}

#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
    let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    if trusted {
        print!("Application is totally trusted!");
    } else {
        print!("Application isn't trusted :(");
    }
    trusted
}

#[cfg(not(target_os = "macos"))]
fn query_accessibility_permissions() -> bool {
    return true;
}

#[derive(specta::Type)]
pub struct Custom(pub String);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(node_config: NodeConfig) {
    log::info!("Starting R!AN 🧠 with config: {:?}", node_config);
    let mut sys = System::new();
    sys.refresh_cpu(); // Refreshing CPU information.
    if let Some(cpu) = sys.cpus().first() {
        let vendor_id = cpu.vendor_id().to_string();
        *CPU_VENDOR.lock() = vendor_id;
    }
    let builder = Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![get_update_result])
        .events(tauri_specta::collect_events![
            CheckUpdateEvent,
            CheckUpdateResultEvent,
        ])
        .ty::<Custom>()
        .constant("universalConstant", 42);
    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default()
                .formatter(specta_typescript::formatter::prettier)
                .header("/* eslint-disable */"),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_jsdoc::JSDoc::default()
                .formatter(specta_typescript::formatter::prettier)
                .header("/* eslint-disable */"),
            "../src/bindings-jsdoc.js",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);
            app.notification()
                .builder()
                .title("R!AN is already running!")
                .body("You can find it in the tray menu.")
                .show()
                .unwrap();
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silently"]),
        ))
        .plugin(tauri_plugin_process::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let app_handle = app.handle();
            APP_HANDLE.get_or_init(|| app.handle().clone());
            app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            if !query_accessibility_permissions() {
                if let Some(window) = app.get_webview_window("rian") {
                    window.minimize().unwrap();
                }
                app.notification()
                    .builder()
                    .title("Accessibility permissions")
                    .body("Please grant accessibility permissions to the app")
                    .icon("icon.png")
                    .show()
                    .unwrap();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
