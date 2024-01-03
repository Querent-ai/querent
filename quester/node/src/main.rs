/// This is the main entry point for the node binary.
use pyo3::PyErr;
use querent_synapse::{
	querent::{py_runtime_init, QuerentError},
	tokio_runtime,
};

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

fn main() -> Result<(), QuerentError> {
	let runtime = tokio_runtime();
	match runtime {
		Ok(runtime) => {
			// Initialize the Python runtime
			let python_runtime_res = py_runtime_init();
			match python_runtime_res {
				Ok(_) => {
					println!("Python runtime initialized.");
					let _ = runtime
						.block_on(main_impl())
						.map_err(|e| QuerentError::internal(e.to_string()));
				},
				Err(e) =>
					return Err(QuerentError::internal(format!(
						"Failed to initialize Python runtime: {}",
						e
					))),
			}
			Ok(())
		},
		Err(err) => Err(QuerentError::internal(err.to_string())),
	}
}

async fn main_impl() -> Result<(), PyErr> {
	Ok(())
}
