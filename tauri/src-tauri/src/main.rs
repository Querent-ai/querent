// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::{initialize_runtimes, RuntimesConfig};
use node::cli::busy_detector;
use proto::NodeConfig;

fn main() {
    // start the runtimes
    busy_detector::set_enabled(true);
    let node_config = NodeConfig::default();

    let runtimes_config = RuntimesConfig::default();
    initialize_runtimes(runtimes_config).expect("failed to initialize runtimes");
    app_lib::run(node_config);
}
