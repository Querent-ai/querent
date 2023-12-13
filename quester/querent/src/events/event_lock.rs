use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
        fmt.debug_struct("EventLock")
            .field("is_alive", &self.is_alive())
            .finish()
    }
}

struct EventLockInner {
    alive: AtomicBool,
    mutex: Mutex<()>,
}

impl Default for EventLockInner {
    fn default() -> Self {
        Self {
            alive: AtomicBool::new(true),
            mutex: Mutex::default(),
        }
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
        assert!(timeout(Duration::from_millis(50), lock.kill())
            .await
            .is_err());
        drop(guard);

        lock.kill().await;
        assert!(lock.is_dead());
        assert!(lock.acquire().await.is_none());
    }
}
