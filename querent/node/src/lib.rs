pub mod cli;
pub mod serve;
use std::sync::Arc;

use clap::Arg;
use cli::busy_detector;
use once_cell::sync::OnceCell;
use proto::config::DEFAULT_CONFIG_PATH;
pub use serve::*;
use tokio::runtime::{Builder, Runtime};

pub type EnvFilterReloadFn = Arc<dyn Fn(&str) -> anyhow::Result<()> + Send + Sync>;

fn config_cli_arg() -> Arg {
	Arg::new("config")
		.long("config")
		.help("Config file location")
		.env("QUERENT_CONFIG")
		.default_value(DEFAULT_CONFIG_PATH)
		.global(true)
		.display_order(1)
}

/// The main tokio runtime takes num_cores / 3 threads by default, and can be overridden by the
/// QUERENT_RUNTIME_NUM_THREADS environment variable.
fn get_main_runtime_num_threads() -> usize {
	let mut default_num_runtime_threads = num_cpus::get() / 3;
	if default_num_runtime_threads < 4 {
		default_num_runtime_threads = 4;
	}
	std::env::var("QUERENT_RUNTIME_NUM_THREADS")
		.ok()
		.and_then(|num_threads_str| num_threads_str.parse().ok())
		.unwrap_or(default_num_runtime_threads)
}

pub fn tokio_runtime() -> Result<&'static Runtime, anyhow::Error> {
	let main_runtime_num_threads: usize = get_main_runtime_num_threads();

	static RUNTIME: OnceCell<Runtime> = OnceCell::new();

	RUNTIME.get_or_try_init(|| {
		Builder::new_multi_thread()
			.enable_all()
			.on_thread_unpark(busy_detector::thread_unpark)
			.on_thread_park(busy_detector::thread_park)
			.worker_threads(main_runtime_num_threads)
			.build()
			.map_err(|err| anyhow::anyhow!("Failed to create tokio runtime: {}", err))
	})
}
