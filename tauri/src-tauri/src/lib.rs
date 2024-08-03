use proto::NodeConfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(node_config: NodeConfig) {
    log::info!("Starting R!AN 🧠 with config: {:?}", node_config);
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
