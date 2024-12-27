// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent

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
	fmt::Debug,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

use tokio::sync::{Mutex, MutexGuard};

// Event locks have two clients: publishers and sources.
//
// Event publishers must acquire the lock and ensure that the lock is alive before publishing.
//
// When a partition reassignment occurs, sources must (i) acquire, then (ii) kill, and finally (iii)
// release the lock before propagating a new lock via message passing to the downstream consumers.
#[derive(Clone, Default)]
pub struct EventLock {
	inner: Arc<EventLockInner>,
}

impl PartialEq for EventLock {
	fn eq(&self, other: &Self) -> bool {
		std::ptr::eq(self.inner.as_ref(), other.inner.as_ref())
	}
}

impl Debug for EventLock {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		fmt.debug_struct("EventLock").field("is_alive", &self.is_alive()).finish()
	}
}

struct EventLockInner {
	alive: AtomicBool,
	mutex: Mutex<()>,
}

impl Default for EventLockInner {
	fn default() -> Self {
		Self { alive: AtomicBool::new(true), mutex: Mutex::default() }
	}
}

impl EventLock {
	pub async fn acquire(&self) -> Option<MutexGuard<'_, ()>> {
		let guard = self.inner.mutex.lock().await;
		if self.is_dead() {
			return None;
		}
		Some(guard)
	}

	pub fn is_alive(&self) -> bool {
		self.inner.alive.load(Ordering::Relaxed)
	}

	pub fn is_dead(&self) -> bool {
		!self.is_alive()
	}

	pub async fn kill(&self) {
		let _guard = self.inner.mutex.lock().await;
		self.inner.alive.store(false, Ordering::Relaxed);
	}
}

#[derive(Debug, PartialEq)]
pub struct NewEventLock(pub EventLock);

#[derive(Debug)]
pub struct NewEventsToken(pub String);

#[cfg(test)]
mod tests {

	use std::time::Duration;

	use tokio::time::timeout;

	use super::*;

	#[tokio::test]
	async fn test_publish_lock() {
		let lock = EventLock::default();
		assert!(lock.is_alive());

		let guard = lock.acquire().await.unwrap();
		assert!(timeout(Duration::from_millis(50), lock.kill()).await.is_err());
		drop(guard);

		lock.kill().await;
		assert!(lock.is_dead());
		assert!(lock.acquire().await.is_none());
	}
}
