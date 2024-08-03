use log::error;
use node::service::serve_quester_without_servers;
use node::tokio_runtime;
use node::QuerentServices;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use proto::NodeConfig;
use specta_typescript::Typescript;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use sysinfo::{CpuExt, System, SystemExt};
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Builder;
use tauri_specta::Event;
use windows::PinnedFromWindowEvent;
mod windows;
use windows::CheckUpdateEvent;
use windows::CheckUpdateResultEvent;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static CPU_VENDOR: Mutex<String> = Mutex::new(String::new());
pub static QUERENT_SERVICES: OnceCell<Arc<QuerentServices>> = OnceCell::new();
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
    let result = UPDATE_RESULT.lock();
    if result.is_none() {
        (false, None)
    } else {
        (true, result.clone().unwrap())
    }
}

#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
    let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    if trusted {
        println!("Application is trusted!");
    } else {
        println!("Application isn't trusted :(");
    }
    trusted
}

#[cfg(not(target_os = "macos"))]
fn query_accessibility_permissions() -> bool {
    true
}

#[derive(specta::Type)]
pub struct Custom(pub String);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(node_config: NodeConfig) {
    log::info!("Starting R!AN 🧠 with config: {:?}", node_config);
    let mut sys = System::new();
    sys.refresh_cpu(); // Refreshing CPU information.
    if let Some(cpu) = sys.cpus().first() {
        *CPU_VENDOR.lock() = cpu.vendor_id().to_string();
    }

    let builder = Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![get_update_result])
        .events(tauri_specta::collect_events![
            CheckUpdateEvent,
            CheckUpdateResultEvent,
            PinnedFromWindowEvent,
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
        .expect("Failed to export TypeScript bindings");

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_jsdoc::JSDoc::default()
                .formatter(specta_typescript::formatter::prettier)
                .header("/* eslint-disable */"),
            "../src/bindings-jsdoc.js",
        )
        .expect("Failed to export TypeScript bindings");

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
            APP_HANDLE.set(app_handle.clone()).unwrap();
            app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            // Ensure Tokio runtime is initialized and used for async tasks
            let runtime = tokio_runtime().expect("failed to initialize tokio runtime");

            // Start QuerentServices asynchronously within the Tokio runtime
            let node_config_clone = node_config.clone();
            runtime.spawn(async move {
                match serve_quester_without_servers(node_config_clone).await {
                    Ok(services) => {
                        let set_services_res = QUERENT_SERVICES.set(services);

                        if set_services_res.is_err() {
                            error!("Failed to set QuerentServices");
                        }
                    }
                    Err(err) => {
                        error!("Failed to start QuerentServices: {:?}", err);
                    }
                }
            });
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

            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
                    let builder = handle.updater_builder();
                    let updater = builder.build().unwrap();

                    match updater.check().await {
                        Ok(Some(update)) => {
                            *UPDATE_RESULT.lock() = Some(Some(UpdateResult {
                                version: update.version,
                                current_version: update.current_version,
                                body: update.body,
                            }));
                        }
                        Ok(None) => {
                            if UPDATE_RESULT.lock().is_some() {
                                if let Some(Some(_)) = *UPDATE_RESULT.lock() {
                                    *UPDATE_RESULT.lock() = Some(None);
                                }
                            } else {
                                *UPDATE_RESULT.lock() = Some(None);
                            }
                        }
                        Err(_) => {}
                    }
                }
            });

            PinnedFromWindowEvent::listen_any(app_handle, move |event| {
                ALWAYS_ON_TOP.store(event.payload.pinned().clone(), Ordering::Release);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
