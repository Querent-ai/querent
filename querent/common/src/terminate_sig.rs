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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc, Mutex, Weak,
};

use tracing::debug;

#[derive(Clone, Default)]
pub struct TerimateSignal {
	inner: Arc<Inner>,
}

struct Inner {
	alive: AtomicBool,
	children: Mutex<Vec<Weak<Inner>>>,
}

impl Default for Inner {
	fn default() -> Self {
		Self { alive: AtomicBool::new(true), children: Mutex::default() }
	}
}

fn garbage_collect(children: &mut Vec<Weak<Inner>>) {
	let mut i = 0;
	while i < children.len() {
		if Weak::strong_count(&children[i]) == 0 {
			children.swap_remove(i);
		} else {
			i += 1;
		}
	}
}

impl TerimateSignal {
	pub fn is_alive(&self) -> bool {
		self.inner.alive.load(Ordering::Relaxed)
	}

	pub fn is_dead(&self) -> bool {
		!self.is_alive()
	}

	pub fn kill(&self) {
		self.inner.kill();
	}

	// Creates a child terimatesignal.
	//
	// If the parent kill switch is dead to begin with, the child will be dead too.
	pub fn child(&self) -> TerimateSignal {
		let mut lock = self.inner.children.lock().unwrap();
		let child_inner = Inner { alive: AtomicBool::new(self.is_alive()), ..Default::default() };
		garbage_collect(&mut lock);
		let child_inner_arc = Arc::new(child_inner);
		lock.push(Arc::downgrade(&child_inner_arc));
		TerimateSignal { inner: child_inner_arc }
	}
}

impl Inner {
	pub fn kill(&self) {
		debug!("kill-switch-activated");
		self.alive.store(false, Ordering::Relaxed);
		let mut lock = self.children.lock().unwrap();
		for weak in lock.drain(..) {
			if let Some(inner) = weak.upgrade() {
				inner.kill();
			}
		}
	}
}
#[cfg(test)]
mod tests {
	use super::TerimateSignal;

	#[test]
	fn test_terminate_sig() {
		let terminate_sig = TerimateSignal::default();
		assert!(terminate_sig.is_alive());
		assert!(!terminate_sig.is_dead());
		terminate_sig.kill();
		assert!(!terminate_sig.is_alive());
		assert!(terminate_sig.is_dead());
		terminate_sig.kill();
		assert!(!terminate_sig.is_alive());
		assert!(terminate_sig.is_dead());
	}

	#[test]
	fn test_terminate_sig_child() {
		let terminate_sig = TerimateSignal::default();
		let child_terminate_sig = terminate_sig.child();
		let child_terminate_sig2 = terminate_sig.child();
		assert!(child_terminate_sig.is_alive());
		assert!(child_terminate_sig2.is_alive());
		terminate_sig.kill();
		assert!(child_terminate_sig.is_dead());
		assert!(child_terminate_sig2.is_dead());
	}

	#[test]
	fn test_terminate_sig_grandchildren() {
		let terminate_sig = TerimateSignal::default();
		let child_terminate_sig = terminate_sig.child();
		let grandchild_terminate_sig = child_terminate_sig.child();
		assert!(terminate_sig.is_alive());
		assert!(child_terminate_sig.is_alive());
		assert!(grandchild_terminate_sig.is_alive());
		terminate_sig.kill();
		assert!(terminate_sig.is_dead());
		assert!(child_terminate_sig.is_dead());
		assert!(grandchild_terminate_sig.is_dead());
	}

	#[test]
	fn test_terminate_sig_to_quoque_me_fili() {
		let terminate_sig = TerimateSignal::default();
		let child_terminate_sig = terminate_sig.child();
		assert!(terminate_sig.is_alive());
		assert!(child_terminate_sig.is_alive());
		child_terminate_sig.kill();
		assert!(terminate_sig.is_alive());
		assert!(child_terminate_sig.is_dead());
	}
}
