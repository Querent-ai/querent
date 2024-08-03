use node::QuerentServices;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use proto::NodeConfig;
use std::sync::atomic::AtomicBool;
use tauri::AppHandle;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static CPU_VENDOR: Mutex<String> = Mutex::new(String::new());
pub static QUERENT_SERVICES: OnceCell<QuerentServices> = OnceCell::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(node_config: NodeConfig) {
    log::info!("Starting R!AN 🧠 with config: {:?}", node_config);
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
