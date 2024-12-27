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

use std::{any::TypeId, fmt, pin::Pin};

use futures::{stream, Stream, TryStreamExt};
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream, WatchStream};
use tracing::warn;

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send + Unpin + 'static>>;

/// A stream impl for code-generated services with streaming endpoints.
pub struct ServiceStream<T> {
	inner: BoxStream<T>,
}

impl<T> ServiceStream<T>
where
	T: Send + 'static,
{
	pub fn new(inner: BoxStream<T>) -> Self {
		Self { inner }
	}

	pub fn empty() -> Self {
		Self { inner: Box::pin(stream::empty()) }
	}
}

impl<T> fmt::Debug for ServiceStream<T>
where
	T: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ServiceStream<{:?}>", TypeId::of::<T>())
	}
}

impl<T> Unpin for ServiceStream<T> {}

impl<T> ServiceStream<T>
where
	T: Send + 'static,
{
	pub fn new_bounded(capacity: usize) -> (mpsc::Sender<T>, Self) {
		let (sender, receiver) = mpsc::channel(capacity);
		(sender, receiver.into())
	}

	pub fn new_unbounded() -> (mpsc::UnboundedSender<T>, Self) {
		let (sender, receiver) = mpsc::unbounded_channel();
		(sender, receiver.into())
	}
}

impl<T> ServiceStream<T>
where
	T: Clone + Send + Sync + 'static,
{
	pub fn new_watch(init: T) -> (watch::Sender<T>, Self) {
		let (sender, receiver) = watch::channel(init);
		(sender, receiver.into())
	}
}

impl<T, E> ServiceStream<Result<T, E>>
where
	T: Send + 'static,
	E: Send + 'static,
{
	pub fn map_err<F, U>(self, f: F) -> ServiceStream<Result<T, U>>
	where
		F: FnMut(E) -> U + Send + 'static,
		U: Send + 'static,
	{
		ServiceStream { inner: Box::pin(self.inner.map_err(f)) }
	}
}

impl<T> Stream for ServiceStream<T> {
	type Item = T;

	fn poll_next(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.inner).poll_next(cx)
	}
}

impl<T> From<mpsc::Receiver<T>> for ServiceStream<T>
where
	T: Send + 'static,
{
	fn from(receiver: mpsc::Receiver<T>) -> Self {
		Self { inner: Box::pin(ReceiverStream::new(receiver)) }
	}
}

impl<T> From<mpsc::UnboundedReceiver<T>> for ServiceStream<T>
where
	T: Send + 'static,
{
	fn from(receiver: mpsc::UnboundedReceiver<T>) -> Self {
		Self { inner: Box::pin(UnboundedReceiverStream::new(receiver)) }
	}
}

impl<T> From<watch::Receiver<T>> for ServiceStream<T>
where
	T: Clone + Send + Sync + 'static,
{
	fn from(receiver: watch::Receiver<T>) -> Self {
		Self { inner: Box::pin(WatchStream::new(receiver)) }
	}
}

/// Adapts a server-side tonic::Streaming into a ServiceStream of `Result<T, tonic::Status>`. Once
/// an error is encountered, the stream will be closed and subsequent calls to `poll_next` will
/// return `None`.
impl<T> From<tonic::Streaming<T>> for ServiceStream<Result<T, tonic::Status>>
where
	T: Send + 'static,
{
	fn from(streaming: tonic::Streaming<T>) -> Self {
		Self { inner: Box::pin(streaming) }
	}
}

/// Adapts a client-side tonic::Streaming into a ServiceStream of `T`. Once an error is encountered,
/// the stream will be closed and subsequent calls to `poll_next` will return `None`.
impl<T> From<tonic::Streaming<T>> for ServiceStream<T>
where
	T: Send + 'static,
{
	fn from(streaming: tonic::Streaming<T>) -> Self {
		let message_stream = stream::unfold(streaming, |mut streaming| {
			Box::pin(async {
				match streaming.message().await {
					Ok(Some(message)) => Some((message, streaming)),
					Ok(None) => None,
					Err(error) => {
						warn!(error=?error, "gRPC transport error");
						None
					},
				}
			})
		});
		Self { inner: Box::pin(message_stream) }
	}
}

#[cfg(any(test, feature = "testsuite"))]
impl<T> From<Vec<T>> for ServiceStream<T>
where
	T: Send + 'static,
{
	fn from(values: Vec<T>) -> Self {
		Self { inner: Box::pin(stream::iter(values)) }
	}
}
