use std::{
	collections::HashMap,
	fmt,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex, Weak,
	},
	time::Duration,
};

use async_trait::async_trait;
use tokio::sync::Mutex as TokioMutex;
use tracing::warn;

use crate::type_map::TMap;

pub trait Event: fmt::Debug + Clone + Send + Sync + 'static {}

#[async_trait]
pub trait EventSubscriber<E>: Send + Sync + 'static {
	async fn handle_event(&mut self, event: E);
}

#[async_trait]
impl<E, F> EventSubscriber<E> for F
where
	E: Event,
	F: Fn(E) + Send + Sync + 'static,
{
	async fn handle_event(&mut self, event: E) {
		(self)(event);
	}
}

type EventSubscriptions<E> = HashMap<usize, EventSubscription<E>>;

#[derive(Debug, Clone, Default)]
pub struct PubSubBroker {
	inner: Arc<InnerPubSubBroker>,
}

#[derive(Debug, Default)]
struct InnerPubSubBroker {
	subscription_sequence: AtomicUsize,
	subscriptions: Mutex<TMap>,
}

impl PubSubBroker {
	/// Subscribes to an event type.
	#[must_use]
	pub fn subscribe<E>(&self, subscriber: impl EventSubscriber<E>) -> EventSubscriptionHandle
	where
		E: Event,
	{
		let mut subscriptions =
			self.inner.subscriptions.lock().expect("lock should not be poisoned");

		if !subscriptions.contains::<EventSubscriptions<E>>() {
			subscriptions.insert::<EventSubscriptions<E>>(HashMap::new());
		}
		let subscription_id = self.inner.subscription_sequence.fetch_add(1, Ordering::Relaxed);

		let subscription =
			EventSubscription { subscriber: Arc::new(TokioMutex::new(Box::new(subscriber))) };
		let typed_subscriptions = subscriptions
			.get_mut::<EventSubscriptions<E>>()
			.expect("subscription map should exist");
		typed_subscriptions.insert(subscription_id, subscription);

		EventSubscriptionHandle {
			subscription_id,
			broker: Arc::downgrade(&self.inner),
			drop_me: |subscription_id, broker| {
				let mut subscriptions =
					broker.subscriptions.lock().expect("lock should not be poisoned");
				if let Some(typed_subscriptions) = subscriptions.get_mut::<EventSubscriptions<E>>()
				{
					typed_subscriptions.remove(&subscription_id);
				}
			},
		}
	}

	/// Publishes an event.
	pub fn publish<E>(&self, event: E)
	where
		E: Event,
	{
		let subscriptions = self.inner.subscriptions.lock().expect("lock should not be poisoned");

		if let Some(typed_subscriptions) = subscriptions.get::<EventSubscriptions<E>>() {
			for subscription in typed_subscriptions.values() {
				let event = event.clone();
				let subscriber_clone = subscription.subscriber.clone();
				let handle_event_fut = async move {
					if tokio::time::timeout(Duration::from_secs(1), async {
						subscriber_clone.lock().await.handle_event(event).await
					})
					.await
					.is_err()
					{
						warn!("`{}` event handler timed out", std::any::type_name::<E>());
					}
				};
				tokio::spawn(handle_event_fut);
			}
		}
	}
}

struct EventSubscription<E> {
	subscriber: Arc<TokioMutex<Box<dyn EventSubscriber<E>>>>,
}

#[derive(Clone)]
pub struct EventSubscriptionHandle {
	subscription_id: usize,
	broker: Weak<InnerPubSubBroker>,
	drop_me: fn(usize, &InnerPubSubBroker),
}

impl EventSubscriptionHandle {
	pub fn cancel(self) {}

	/// By default, dropping a subscription handle cancels the subscription.
	/// `forever` consumes the handle and avoids cancelling the subscription on drop.
	pub fn forever(mut self) {
		self.broker = Weak::new();
	}
}

impl Drop for EventSubscriptionHandle {
	fn drop(&mut self) {
		if let Some(broker) = self.broker.upgrade() {
			(self.drop_me)(self.subscription_id, &broker);
		}
	}
}

#[cfg(test)]
mod tests {

	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};

	use super::*;

	#[derive(Debug, Clone)]
	struct MyEvent {
		value: usize,
	}

	impl Event for MyEvent {}

	#[derive(Debug, Clone)]
	struct MySubscriber {
		counter: Arc<AtomicUsize>,
	}

	#[async_trait]
	impl EventSubscriber<MyEvent> for MySubscriber {
		async fn handle_event(&mut self, event: MyEvent) {
			self.counter.store(event.value, Ordering::Relaxed);
		}
	}

	#[tokio::test]
	async fn test_event_broker() {
		let event_broker = PubSubBroker::default();
		let counter = Arc::new(AtomicUsize::new(0));
		let subscriber = MySubscriber { counter: counter.clone() };
		let subscription_handle = event_broker.subscribe(subscriber);

		let event = MyEvent { value: 42 };
		event_broker.publish(event);

		tokio::time::sleep(Duration::from_millis(1)).await;
		assert_eq!(counter.load(Ordering::Relaxed), 42);

		subscription_handle.cancel();

		let event = MyEvent { value: 1337 };
		event_broker.publish(event);

		tokio::time::sleep(Duration::from_millis(1)).await;
		assert_eq!(counter.load(Ordering::Relaxed), 42);
	}

	#[tokio::test]
	async fn test_event_broker_handle_drop() {
		let event_broker = PubSubBroker::default();
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		drop(event_broker.subscribe(move |event: MyEvent| {
			tx.send(event.value).unwrap();
		}));
		event_broker.publish(MyEvent { value: 42 });
		assert!(rx.recv().await.is_none());
	}

	#[tokio::test]
	async fn test_event_broker_handle_cancel() {
		let event_broker = PubSubBroker::default();
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		event_broker
			.subscribe(move |event: MyEvent| {
				tx.send(event.value).unwrap();
			})
			.cancel();
		event_broker.publish(MyEvent { value: 42 });
		assert!(rx.recv().await.is_none());
	}

	#[tokio::test]
	async fn test_event_broker_handle_forever() {
		let event_broker = PubSubBroker::default();
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		event_broker
			.subscribe(move |event: MyEvent| {
				tx.send(event.value).unwrap();
			})
			.forever();
		event_broker.publish(MyEvent { value: 42 });
		assert_eq!(rx.recv().await, Some(42));
	}
}
