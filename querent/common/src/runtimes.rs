// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use std::{
	collections::HashMap,
	sync::atomic::{AtomicUsize, Ordering},
};

use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

static RUNTIMES: OnceCell<HashMap<RuntimeType, tokio::runtime::Runtime>> = OnceCell::new();

/// Describes which runtime an actor should run on.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum RuntimeType {
	/// The blocking runtime runs blocking actors.
	/// This runtime is only used as a nice thread pool with
	/// the interface as tokio stasks.
	///
	/// This runtime should not be used to run tokio
	/// io operations.
	///
	/// Tasks are allowed to block for an arbitrary amount of time.
	Blocking,

	/// The non-blocking runtime is closer to what one would expect from
	/// a regular tokio runtime.
	///
	/// Task are expect to yield within 500 micros.
	NonBlocking,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimesConfig {
	/// Number of worker threads allocated to the non-blocking runtime.
	pub num_threads_non_blocking: usize,
	/// Number of worker threads allocated to the blocking runtime.
	pub num_threads_blocking: usize,
}

impl RuntimesConfig {
	#[cfg(any(test, feature = "testsuite"))]
	pub fn light_for_tests() -> RuntimesConfig {
		RuntimesConfig { num_threads_blocking: 1, num_threads_non_blocking: 1 }
	}

	pub fn with_num_cpus(num_cpus: usize) -> Self {
		// Non blocking task are supposed to be io intensive, and  not require many threads...
		let num_threads_non_blocking = if num_cpus > 6 { 2 } else { 1 };
		// On the other hand the blocking actors are cpu intensive. We allocate
		// almost all of the threads to them.
		let num_threads_blocking = (num_cpus - num_threads_non_blocking).max(1);
		RuntimesConfig { num_threads_non_blocking, num_threads_blocking }
	}
}

impl Default for RuntimesConfig {
	fn default() -> Self {
		let num_cpus = num_cpus::get();
		Self::with_num_cpus(num_cpus)
	}
}

fn start_runtimes(config: RuntimesConfig) -> HashMap<RuntimeType, Runtime> {
	let mut runtimes = HashMap::default();
	let blocking_runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(config.num_threads_blocking)
		.thread_name_fn(|| {
			static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
			let id = ATOMIC_ID.fetch_add(1, Ordering::AcqRel);
			format!("blocking-{id}")
		})
		.enable_all()
		.build()
		.unwrap();
	runtimes.insert(RuntimeType::Blocking, blocking_runtime);
	let non_blocking_runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(config.num_threads_non_blocking)
		.thread_name_fn(|| {
			static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
			let id = ATOMIC_ID.fetch_add(1, Ordering::AcqRel);
			format!("non-blocking-{id}")
		})
		.enable_all()
		.build()
		.unwrap();
	runtimes.insert(RuntimeType::NonBlocking, non_blocking_runtime);
	runtimes
}

pub fn initialize_runtimes(runtimes_config: RuntimesConfig) -> anyhow::Result<()> {
	RUNTIMES.get_or_init(|| start_runtimes(runtimes_config));
	Ok(())
}

impl RuntimeType {
	pub fn get_runtime_handle(self) -> tokio::runtime::Handle {
		RUNTIMES
            .get_or_init(|| {
                #[cfg(any(test, feature = "testsuite"))]
                {
                    tracing::warn!("starting Tokio actor runtimes for tests");
                    start_runtimes(RuntimesConfig::light_for_tests())
                }
                #[cfg(not(any(test, feature = "testsuite")))]
                {
                    panic!("Tokio runtimes not initialized. Please, report this issue on GitHub: https://github.com/querent-ai/querent/issues.");
                }
            })
            .get(&self)
            .unwrap()
            .handle()
            .clone()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_runtimes_config_default() {
		let runtime_default = RuntimesConfig::default();
		assert!(runtime_default.num_threads_non_blocking <= runtime_default.num_threads_blocking);
		assert!(runtime_default.num_threads_non_blocking <= 2);
	}

	#[test]
	fn test_runtimes_with_given_num_cpus_10() {
		let runtime = RuntimesConfig::with_num_cpus(10);
		assert_eq!(runtime.num_threads_blocking, 8);
		assert_eq!(runtime.num_threads_non_blocking, 2);
	}

	#[test]
	fn test_runtimes_with_given_num_cpus_3() {
		let runtime = RuntimesConfig::with_num_cpus(3);
		assert_eq!(runtime.num_threads_blocking, 2);
		assert_eq!(runtime.num_threads_non_blocking, 1);
	}
}
