// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use common::{initialize_runtimes, RuntimesConfig};
use node::{cli::busy_detector, tokio_runtime};
use proto::NodeConfig;

fn main() {
    let custom_runtime = tokio_runtime().expect("failed to initialize tokio runtime");
    // start the runtimes
    busy_detector::set_enabled(true);
    let node_config = NodeConfig::default();

    let runtimes_config = RuntimesConfig::default();
    initialize_runtimes(runtimes_config).expect("failed to initialize runtimes");
    tauri::async_runtime::set(custom_runtime.handle().clone());
    app_lib::run(node_config);
}
