use api::{
    check_if_service_is_running, delete_collectors, describe_pipeline, get_collectors,
    get_drive_credentials, get_past_agns, get_running_agns, get_running_insight_analysts,
    get_update_result, has_rian_license_key, ingest_tokens, list_available_insights,
    list_past_insights, prompt_insight_analyst, send_discovery_retriever_request, set_collectors,
    set_rian_license_key, start_agn_fabric, stop_agn_fabric, stop_insight_analyst,
    trigger_insight_analyst,
};
use log::{error, info};
use node::{
    service::serve_quester_without_servers, shutdown_querent, tokio_runtime, QuerentServices,
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use proto::{semantics::SemanticPipelineRequest, InsightAnalystRequest, NodeConfig};
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use sysinfo::System;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::{Builder, Event};
use windows::{
    show_updater_window, CheckUpdateEvent, CheckUpdateResultEvent, PinnedFromWindowEvent,
};

mod api;
mod tray;
mod windows;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static CPU_VENDOR: Mutex<String> = Mutex::new(String::new());
pub static QUERENT_SERVICES: OnceCell<Arc<QuerentServices>> = OnceCell::new();
pub static UPDATE_RESULT: Mutex<Option<Option<UpdateResult>>> = Mutex::new(None);
pub static RUNNING_PIPELINE_ID: Mutex<Vec<(String, SemanticPipelineRequest)>> =
    Mutex::new(Vec::new());
pub static RUNNING_DISCOVERY_SESSION_ID: Mutex<String> = Mutex::new(String::new());
pub static RUNNING_INSIGHTS_SESSIONS: Mutex<Vec<(String, InsightAnalystRequest)>> =
    Mutex::new(Vec::new());

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    version: String,
    current_version: String,
    body: Option<String>,
}

#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
    let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    if trusted {
        info!("Application is trusted!");
    } else {
        error!("Application isn't trusted :(");
    }
    trusted
}

#[cfg(not(target_os = "macos"))]
fn query_accessibility_permissions() -> bool {
    true
}

#[derive(specta::Type)]
pub struct Custom(pub String);

#[allow(deprecated)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(node_config: NodeConfig) {
    info!("Starting R!AN 🧠 with config: {:?}", node_config);
    let mut sys = System::new();
    sys.refresh_cpu_all();
    if let Some(cpu) = sys.cpus().first() {
        *CPU_VENDOR.lock() = cpu.vendor_id().to_string();
    }

    let builder = Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            get_update_result,
            check_if_service_is_running,
            has_rian_license_key,
            set_rian_license_key,
            set_collectors,
            get_collectors,
            start_agn_fabric,
            get_running_agns,
            stop_agn_fabric,
            send_discovery_retriever_request,
            list_available_insights,
            list_past_insights,
            trigger_insight_analyst,
            get_running_insight_analysts,
            stop_insight_analyst,
            prompt_insight_analyst,
            get_past_agns,
            get_drive_credentials,
            delete_collectors,
            ingest_tokens,
            describe_pipeline
        ])
        .events(tauri_specta::collect_events![
            CheckUpdateEvent,
            CheckUpdateResultEvent,
            PinnedFromWindowEvent
        ])
        .ty::<Custom>()
        .constant("universalConstant", 42);

    #[cfg(debug_assertions)]
    {
        builder
            .export(
                Typescript::default()
                    .formatter(specta_typescript::formatter::prettier)
                    .header("/* eslint-disable */"),
                "../src/service/bindings.ts",
            )
            .expect("Failed to export TypeScript bindings");

        builder
            .export(
                specta_jsdoc::JSDoc::default()
                    .formatter(specta_typescript::formatter::prettier)
                    .header("/* eslint-disable */"),
                "../src/service/bindings-jsdoc.js",
            )
            .expect("Failed to export JSDoc bindings");
    }

    #[allow(unused_mut)]
    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            info!("{}, {argv:?}, {cwd}", app.package_info().name);
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
            tray::create_tray(&app_handle)?;
            app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            let runtime = tokio_runtime().expect("Failed to initialize Tokio runtime");

            start_querent_services(runtime, node_config.clone());

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

            schedule_update_checks(app_handle.clone());
            monitor_pinned_event(app_handle.clone());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Error while running tauri application");

    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular);

    app.run(move |app, event| match event {
        tauri::RunEvent::Exit { .. } => {
            if let Some(services) = QUERENT_SERVICES.get() {
                let services = services.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = shutdown_querent(&services).await {
                        error!("Failed to shut down QuerentServices: {:?}", err);
                    }
                });
            }
        }
        tauri::RunEvent::Ready => {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let builder = handle.updater_builder();
                let updater = builder.build().unwrap();

                match updater.check().await {
                    Ok(Some(update)) => {
                        *UPDATE_RESULT.lock() = Some(Some(UpdateResult {
                            version: update.version,
                            current_version: update.current_version,
                            body: update.body,
                        }));
                        tray::create_tray(&handle).unwrap();
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        show_updater_window();
                    }
                    Ok(None) => {
                        if let Some(Some(_)) = *UPDATE_RESULT.lock() {
                            *UPDATE_RESULT.lock() = Some(None);
                            tray::create_tray(&handle).unwrap();
                        } else {
                            *UPDATE_RESULT.lock() = Some(None);
                        }
                    }
                    Err(err) => {
                        error!("Update check failed: {:?}", err);
                    }
                }
            });
        }
        _ => {}
    });
}

fn start_querent_services(runtime: &tokio::runtime::Runtime, node_config: NodeConfig) {
    runtime.spawn(async move {
        match serve_quester_without_servers(node_config).await {
            Ok(services) => {
                if QUERENT_SERVICES.set(services).is_err() {
                    error!("Failed to set QuerentServices");
                }
            }
            Err(err) => {
                error!("Failed to start QuerentServices: {:?}", err);
            }
        }
    });
}

fn schedule_update_checks(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
            let builder = app_handle.updater_builder();
            let updater = builder.build().unwrap();

            match updater.check().await {
                Ok(Some(update)) => {
                    *UPDATE_RESULT.lock() = Some(Some(UpdateResult {
                        version: update.version,
                        current_version: update.current_version,
                        body: update.body,
                    }));
                    tray::create_tray(&app_handle).unwrap();
                }
                Ok(None) => {
                    if let Some(Some(_)) = *UPDATE_RESULT.lock() {
                        *UPDATE_RESULT.lock() = Some(None);
                        tray::create_tray(&app_handle).unwrap();
                    } else {
                        *UPDATE_RESULT.lock() = Some(None);
                    }
                }
                Err(err) => {
                    error!("Update check failed: {:?}", err);
                }
            }
        }
    });
}

fn monitor_pinned_event(app_handle: AppHandle) {
    PinnedFromWindowEvent::listen_any(&app_handle.clone(), move |event| {
        ALWAYS_ON_TOP.store(event.payload.pinned().clone(), Ordering::Release);
        tray::create_tray(&app_handle).unwrap();
    });
}
