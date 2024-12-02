use clap::{ArgMatches, Command};
pub use common::{initialize_runtimes, RuntimesConfig};
use rian_core::MAX_DATA_SIZE_IN_MEMORY;
use tokio::signal;
use tracing::{debug, info};

use crate::{cli::load_node_config, config_cli_arg, serve_quester};

pub fn build_serve_command() -> Command {
	Command::new("serve")
		.about("Starts a Querent node.")
		.long_about("Starts a Querent node with all services enabled by default.")
		.arg(config_cli_arg())
}

#[derive(Debug, Eq, PartialEq)]
pub struct Serve {
	node_config_uri: String,
}

pub mod busy_detector {
	use std::{
		sync::atomic::{AtomicBool, AtomicU64, Ordering},
		time::Instant,
	};

	use once_cell::sync::Lazy;
	use tracing::debug;
	static TIME_REF: Lazy<Instant> = Lazy::new(Instant::now);
	static ENABLED: AtomicBool = AtomicBool::new(false);

	const ALLOWED_DELAY_MICROS: u64 = 5000;
	const DEBUG_SUPPRESSION_MICROS: u64 = 30_000_000;

	thread_local!(static LAST_UNPARK_TIMESTAMP: AtomicU64 = AtomicU64::new(0));
	static NEXT_DEBUG_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
	static SUPPRESSED_DEBUG_COUNT: AtomicU64 = AtomicU64::new(0);

	pub fn set_enabled(enabled: bool) {
		ENABLED.store(enabled, Ordering::Relaxed);
	}

	pub fn thread_unpark() {
		LAST_UNPARK_TIMESTAMP.with(|time| {
			let now = Instant::now().checked_duration_since(*TIME_REF).unwrap_or_default();
			time.store(now.as_micros() as u64, Ordering::Relaxed);
		})
	}

	pub fn thread_park() {
		if !ENABLED.load(Ordering::Relaxed) {
			return;
		}

		LAST_UNPARK_TIMESTAMP.with(|time| {
			let now = Instant::now().checked_duration_since(*TIME_REF).unwrap_or_default();
			let now = now.as_micros() as u64;
			let delta = now - time.load(Ordering::Relaxed);
			if delta > ALLOWED_DELAY_MICROS {
				emit_debug(delta, now);
			}
		})
	}

	fn emit_debug(delta: u64, now: u64) {
		if NEXT_DEBUG_TIMESTAMP
			.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next_debug| {
				if next_debug < now {
					Some(now + DEBUG_SUPPRESSION_MICROS)
				} else {
					None
				}
			})
			.is_err()
		{
			// a debug was emited recently, don't emit log for this one
			SUPPRESSED_DEBUG_COUNT.fetch_add(1, Ordering::Relaxed);
			return;
		}

		let suppressed = SUPPRESSED_DEBUG_COUNT.swap(0, Ordering::Relaxed);
		if suppressed == 0 {
			debug!("thread wasn't parked for {delta}µs, is the runtime too busy?");
		} else {
			debug!(
				"thread wasn't parked for {delta}µs, is the runtime too busy? ({suppressed} \
                 similar messages suppressed)"
			);
		}
	}
}

impl Serve {
	pub fn parse_cli_args(mut matches: ArgMatches) -> anyhow::Result<Self> {
		let config_uri = matches.try_remove_one::<String>("config").unwrap_or_default();
		Ok(Serve { node_config_uri: config_uri.unwrap_or_default() })
	}

	pub async fn execute(&self) -> anyhow::Result<()> {
		debug!(args = ?self, "run-querent-service");
		busy_detector::set_enabled(true);
		let node_config = load_node_config(&self.node_config_uri).await.unwrap_or_default();
		let res = MAX_DATA_SIZE_IN_MEMORY.set(node_config.memory_capacity as usize);
		if res.is_err() {
			info!("MAX_DATA_SIZE_IN_MEMORY is already set");
		}
		let runtimes_config = RuntimesConfig::default();
		initialize_runtimes(runtimes_config)?;
		let shutdown_signal = Box::pin(async move {
			signal::ctrl_c()
				.await
				.expect("Registering a signal handler for SIGINT should not fail.");
		});
		info!("Starting Querent node");
		let serve_result = serve_quester(node_config, runtimes_config, shutdown_signal).await?;
		match serve_result.is_empty() {
			true => info!("Querent node has shut down"),
			false => info!("Querent node has shut down with errors"),
		}
		Ok(())
	}
}
