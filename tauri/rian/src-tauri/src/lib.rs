use api::{
    check_if_service_is_running, delete_collectors, describe_pipeline, get_collectors,
    get_drive_credentials, get_past_agns, get_running_agns, get_running_insight_analysts,
    get_update_result, has_rian_license_key, ingest_tokens, list_available_insights,
    list_past_insights, prompt_insight_analyst, send_discovery_retriever_request, set_collectors,
    set_rian_license_key, start_agn_fabric, stop_agn_fabric, stop_insight_analyst,
    trigger_insight_analyst,
};
use common::{get_querent_data_path, TerimateSignal};
use diesel::prelude::*;
use diesel::r2d2;
use log::{error, info};
use node::serve::service::QUERENT_SERVICES_ONCE;
use node::{serve_quester_without_servers, shutdown_querent};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use postgresql_embedded::blocking::PostgreSQL;
use postgresql_embedded::{Settings, VersionReq};
use proto::{semantics::SemanticPipelineRequest, InsightAnalystRequest, NodeConfig};
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{atomic::AtomicBool, atomic::Ordering};
use storage::{StorageError, StorageErrorKind};
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
pub static UPDATE_RESULT: Mutex<Option<Option<UpdateResult>>> = Mutex::new(None);
pub static RUNNING_PIPELINE_ID: Mutex<Vec<(String, SemanticPipelineRequest)>> =
    Mutex::new(Vec::new());
pub static RUNNING_DISCOVERY_SESSION_ID: Mutex<String> = Mutex::new(String::new());
pub static RUNNING_INSIGHTS_SESSIONS: Mutex<Vec<(String, InsightAnalystRequest)>> =
    Mutex::new(Vec::new());
pub const FETCH_LIMIT_MAX: i64 = 50;
pub const SITEMAP_LIMIT: i64 = 50000;
pub const SITEMAP_DAYS: i64 = 31;
lazy_static::lazy_static! {
    static ref PG_EMBED: Arc<Mutex<Option<PostgreSQL>>> = Arc::new(Mutex::new(None));
}

const DB_NAME: &str = "querent_rian_standalone_v1";

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
pub fn run(node_config: NodeConfig) {
    info!("Starting R!AN 🧠 with config: {:?}", node_config);
    let mut sys = System::new();
    let terimante_sig = TerimateSignal::default();
    let sig_clone = terimante_sig.clone();
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
    // Start embedded querent services
    let querent_data_path = get_querent_data_path();
    start_postgres_sync(querent_data_path.clone()).expect("Failed to start embedded Postgres");
    if PG_EMBED.lock().is_none() {
        error!("Failed to start embedded Postgres");
        return;
    }
    let _psql = PG_EMBED.lock().take().unwrap();
    let url = _psql.settings().url(DB_NAME);
    info!("Postgres started at: {}", url);
    let app = tauri::Builder::default()
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
            // start querent services
            tauri::async_runtime::spawn(start_querent_services(
                node_config.clone(),
                sig_clone,
                Some(url),
            ));
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
    app.run(move |app: &AppHandle, event: tauri::RunEvent| match event {
        tauri::RunEvent::Exit { .. } => {
            if let Some(services) = QUERENT_SERVICES_ONCE.get() {
                let services = services.clone();
                terimante_sig.kill();
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

    _psql.stop().expect("Failed to stop embedded Postgres");
    drop(_psql);
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
                Ok(_) => {
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

async fn start_querent_services(
    node_config: NodeConfig,
    terminate_sig: TerimateSignal,
    embedded_url: Option<String>,
) {
    match serve_quester_without_servers(node_config, terminate_sig.clone(), embedded_url, None)
        .await
    {
        // if error occurs, we should terminate the app
        Ok(_) => {
            println!("QuerentServices started successfully");
        }
        Err(err) => {
            error!("Failed to start QuerentServices: {:?}", err);
            terminate_sig.kill();
        }
    }
}

pub fn start_postgres_sync(path: PathBuf) -> Result<(), StorageError> {
    let mut data_dir = path.clone();
    data_dir.push("rian_postgres_standalone");
    // create the data directory if it does not exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;
    }
    let mut password_dir = path.clone();
    password_dir.push("rian_postgres_password");
    // create the password directory if it does not exist
    if !password_dir.exists() {
        std::fs::create_dir_all(&password_dir).map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;
    }
    let passwword_file_name = ".pgpass";
    let mut settings = Settings::new();
    settings.version = VersionReq::parse("=16.4.0").map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;
    settings.temporary = false;
    settings.data_dir = data_dir.clone();
    settings.username = "postgres".to_string();
    settings.password = "postgres".to_string();
    settings.password_file = password_dir.join(passwword_file_name);
    let mut postgresql = postgresql_embedded::blocking::PostgreSQL::new(settings);
    // Synchronously set up PostgreSQL
    postgresql.setup().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;

    // Install PostgreSQL extensions synchronously
    let postgresql_settings = postgresql.settings().clone();
    postgresql_extensions::blocking::install(
        &postgresql_settings,
        "tensor-chord",
        "pgvecto.rs",
        &VersionReq::parse("=0.3.0").map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?,
    )
    .map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;

    // Start PostgreSQL server synchronously
    // Check if PostgreSQL is already running; stop it if it is.
    if is_postgres_running(&data_dir) {
        postgresql.stop().map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;
    }
    match postgresql.start() {
        Ok(_) => log::info!("PostgreSQL started successfully."),
        Err(e) => {
            eprintln!("Failed to start PostgreSQL: {:?}", e);
            return Err(StorageError {
                kind: StorageErrorKind::Internal,
                source: Arc::new(anyhow::Error::from(e)),
            });
        }
    }
    // Check if the database exists
    let exists = postgresql
        .database_exists(DB_NAME)
        .map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;

    // Create the database synchronously
    if !exists {
        postgresql
            .create_database(DB_NAME)
            .map_err(|e| StorageError {
                kind: StorageErrorKind::Internal,
                source: Arc::new(anyhow::Error::from(e)),
            })?;
    }
    // if exists we dont need to do below steps that would mean it has been done before
    if exists {
        // store the postgresql instance
        *PG_EMBED.lock() = Some(postgresql);
        return Ok(());
    }
    // Restart PostgreSQL server to apply extension changes
    postgresql.stop().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;

    postgresql.start().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;
    let database_url = postgresql.settings().url(DB_NAME);
    let manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url.clone());
    let pool = r2d2::Pool::builder()
        .max_size(10)
        .build(manager)
        .map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;

    // Configue shared_preload_libraries = 'vectors.so' in postgresql.conf
    // and restart the server
    let conn = &mut pool.get().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;
    diesel::sql_query("ALTER SYSTEM SET shared_preload_libraries = 'vectors.so'")
        .execute(conn)
        .map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;
    diesel::sql_query("ALTER SYSTEM SET search_path = '$user', public, vectors")
        .execute(conn)
        .map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;
    // close the pool
    drop(pool);

    // restart the postgresql server
    postgresql.stop().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;
    postgresql.start().map_err(|e| StorageError {
        kind: StorageErrorKind::Internal,
        source: Arc::new(anyhow::Error::from(e)),
    })?;
    // check if the database is created
    postgresql
        .database_exists(DB_NAME)
        .map_err(|e| StorageError {
            kind: StorageErrorKind::Internal,
            source: Arc::new(anyhow::Error::from(e)),
        })?;

    // store the postgresql instance
    *PG_EMBED.lock() = Some(postgresql);
    Ok(())
}

fn is_postgres_running(data_dir: &PathBuf) -> bool {
    let pid_file = data_dir.join("postmaster.pid");
    pid_file.exists()
}
