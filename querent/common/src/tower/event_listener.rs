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

use std::{
	pin::Pin,
	task::{Context, Poll},
};

use futures::{ready, Future};
use pin_project::pin_project;
use tower::{Layer, Service};

use crate::pubsub::{Event, PubSubBroker};

#[derive(Clone)]
pub struct EventListener<S> {
	inner: S,
	event_broker: PubSubBroker,
}

impl<S> EventListener<S> {
	pub fn new(inner: S, event_broker: PubSubBroker) -> Self {
		Self { inner, event_broker }
	}
}

impl<S, R> Service<R> for EventListener<S>
where
	S: Service<R>,
	R: Event,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = ResponseFuture<S::Future, R>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: R) -> Self::Future {
		let inner = self.inner.call(request.clone());
		ResponseFuture { inner, event_broker: self.event_broker.clone(), request: Some(request) }
	}
}

#[derive(Debug, Clone)]
pub struct EventListenerLayer {
	event_broker: PubSubBroker,
}

impl EventListenerLayer {
	pub fn new(event_broker: PubSubBroker) -> Self {
		Self { event_broker }
	}
}

impl<S> Layer<S> for EventListenerLayer {
	type Service = EventListener<S>;

	fn layer(&self, service: S) -> Self::Service {
		EventListener::new(service, self.event_broker.clone())
	}
}

/// Response future for [`EventListener`].
#[pin_project]
pub struct ResponseFuture<F, R> {
	#[pin]
	inner: F,
	event_broker: PubSubBroker,
	request: Option<R>,
}

impl<R, F, T, E> Future for ResponseFuture<F, R>
where
	R: Event,
	F: Future<Output = Result<T, E>>,
{
	type Output = Result<T, E>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		let response = ready!(this.inner.poll(cx));

		if response.is_ok() {
			this.event_broker.publish(this.request.take().expect("request should be set"));
		}
		Poll::Ready(Ok(response?))
	}
}

#[cfg(test)]
mod tests {
	use std::{
		sync::{
			atomic::{AtomicUsize, Ordering},
			Arc,
		},
		time::Duration,
	};

	use async_trait::async_trait;

	use super::*;
	use crate::pubsub::EventSubscriber;

	#[derive(Debug, Clone, Copy)]
	struct MyEvent {
		return_ok: bool,
	}

	impl Event for MyEvent {}

	struct MySubscriber {
		counter: Arc<AtomicUsize>,
	}

	#[async_trait]
	impl EventSubscriber<MyEvent> for MySubscriber {
		async fn handle_event(&mut self, _event: MyEvent) {
			self.counter.fetch_add(1, Ordering::Relaxed);
		}
	}

	#[tokio::test]
	async fn test_event_listener() {
		let event_broker = PubSubBroker::default();
		let counter = Arc::new(AtomicUsize::new(0));
		let subscriber = MySubscriber { counter: counter.clone() };
		let _subscription_handle = event_broker.subscribe::<MyEvent>(subscriber);

		let layer = EventListenerLayer::new(event_broker);

		let mut service = layer.layer(tower::service_fn(|request: MyEvent| async move {
			if request.return_ok {
				Ok(())
			} else {
				Err(())
			}
		}));
		let request = MyEvent { return_ok: false };
		service.call(request).await.unwrap_err();

		tokio::time::sleep(Duration::from_millis(1)).await;
		assert_eq!(counter.load(Ordering::Relaxed), 0);

		let request = MyEvent { return_ok: true };
		service.call(request).await.unwrap();

		tokio::time::sleep(Duration::from_millis(1)).await;
		assert_eq!(counter.load(Ordering::Relaxed), 1);
	}
}
