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
	any::type_name,
	fmt,
	pin::Pin,
	task::{Context, Poll},
};

use futures::Future;
use pin_project::pin_project;
use tokio::time::Sleep;
use tower::{
	retry::{Policy, Retry},
	Layer,
};
use tracing::debug;

use crate::retry::{RetryParams, Retryable};

/// Retry layer copy/pasted from `tower::retry::RetryLayer`
/// but which implements `Clone`.
impl<P, S> Layer<S> for RetryLayer<P>
where
	P: Clone,
{
	type Service = Retry<P, S>;

	fn layer(&self, service: S) -> Self::Service {
		let policy = self.policy.clone();
		Retry::new(policy, service)
	}
}

#[derive(Clone, Debug)]
pub struct RetryLayer<P> {
	policy: P,
}

impl<P> RetryLayer<P> {
	/// Create a new [`RetryLayer`] from a retry policy
	pub fn new(policy: P) -> Self {
		RetryLayer { policy }
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RetryPolicy {
	num_attempts: usize,
	retry_params: RetryParams,
}

impl From<RetryParams> for RetryPolicy {
	fn from(retry_params: RetryParams) -> Self {
		Self { num_attempts: 0, retry_params }
	}
}

#[pin_project]
pub struct RetryFuture {
	retry_policy: RetryPolicy,
	#[pin]
	sleep_fut: Sleep,
}

impl Future for RetryFuture {
	type Output = RetryPolicy;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		this.sleep_fut.poll(cx).map(|_| *this.retry_policy)
	}
}

impl<R, T, E> Policy<R, T, E> for RetryPolicy
where
	R: Clone,
	E: fmt::Debug + Retryable,
{
	type Future = RetryFuture;

	fn retry(&self, _request: &R, result: Result<&T, &E>) -> Option<Self::Future> {
		match result {
			Ok(_) => None,
			Err(error) => {
				let num_attempts = self.num_attempts + 1;

				if !error.is_retryable() || num_attempts >= self.retry_params.max_attempts {
					None
				} else {
					let delay = self.retry_params.compute_delay(num_attempts);
					debug!(
						num_attempts=%num_attempts,
						delay_millis=%delay.as_millis(),
						error=?error,
						"{} request failed, retrying.", type_name::<R>()
					);
					let retry_policy = Self { num_attempts, retry_params: self.retry_params };
					let sleep_fut = tokio::time::sleep(delay);
					let retry_fut = RetryFuture { retry_policy, sleep_fut };
					Some(retry_fut)
				}
			},
		}
	}

	fn clone_request(&self, request: &R) -> Option<R> {
		Some(request.clone())
	}
}

#[cfg(test)]
mod tests {
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex,
	};

	use futures::future::{ready, Ready};
	use tower::{Layer, Service, ServiceExt};

	use super::*;

	#[derive(Debug, Eq, PartialEq)]
	pub enum Retry<E> {
		Permanent(E),
		Transient(E),
	}

	impl<E> Retryable for Retry<E> {
		fn is_retryable(&self) -> bool {
			match self {
				Retry::Permanent(_) => false,
				Retry::Transient(_) => true,
			}
		}
	}

	#[derive(Debug, Clone, Default)]
	struct HelloService;

	type HelloResults = Arc<Mutex<Vec<Result<(), Retry<()>>>>>;

	#[derive(Debug, Clone, Default)]
	struct HelloRequest {
		num_attempts: Arc<AtomicUsize>,
		results: HelloResults,
	}

	impl Service<HelloRequest> for HelloService {
		type Response = ();
		type Error = Retry<()>;
		type Future = Ready<Result<(), Retry<()>>>;

		fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, request: HelloRequest) -> Self::Future {
			request.num_attempts.fetch_add(1, Ordering::Relaxed);
			let result = request
				.results
				.lock()
				.expect("The lock should not be poisoned.")
				.pop()
				.unwrap_or(Err(Retry::Permanent(())));
			ready(result)
		}
	}

	#[tokio::test]
	async fn test_retry_policy() {
		let retry_policy = RetryPolicy::from(RetryParams::for_test());
		let retry_layer = RetryLayer::new(retry_policy);
		let mut retry_hello_service = retry_layer.layer(HelloService);

		let hello_request =
			HelloRequest { results: Arc::new(Mutex::new(vec![Ok(())])), ..Default::default() };
		retry_hello_service
			.ready()
			.await
			.unwrap()
			.call(hello_request.clone())
			.await
			.unwrap();
		assert_eq!(hello_request.num_attempts.load(Ordering::Relaxed), 1);

		let hello_request = HelloRequest {
			results: Arc::new(Mutex::new(vec![Ok(()), Err(Retry::Transient(()))])),
			..Default::default()
		};
		retry_hello_service
			.ready()
			.await
			.unwrap()
			.call(hello_request.clone())
			.await
			.unwrap();
		assert_eq!(hello_request.num_attempts.load(Ordering::Relaxed), 2);

		let hello_request = HelloRequest {
			results: Arc::new(Mutex::new(vec![
				Err(Retry::Transient(())),
				Err(Retry::Transient(())),
				Err(Retry::Transient(())),
			])),
			..Default::default()
		};
		retry_hello_service
			.ready()
			.await
			.unwrap()
			.call(hello_request.clone())
			.await
			.unwrap_err();
		assert_eq!(hello_request.num_attempts.load(Ordering::Relaxed), 4);

		let hello_request = HelloRequest::default();
		retry_hello_service
			.ready()
			.await
			.unwrap()
			.call(hello_request.clone())
			.await
			.unwrap_err();
		assert_eq!(hello_request.num_attempts.load(Ordering::Relaxed), 1);
	}
}
