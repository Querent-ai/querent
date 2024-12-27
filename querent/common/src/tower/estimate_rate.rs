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
	marker::PhantomData,
	task::{Context, Poll},
	time::Instant,
};

use tower::{
	load::{completion::TrackCompletionFuture, CompleteOnResponse},
	Layer, Service,
};

use super::{Cost, RateEstimator};

pub struct Handle<T: RateEstimator> {
	started_at: Instant,
	work: u64,
	estimator: T,
}

impl<T> Drop for Handle<T>
where
	T: RateEstimator,
{
	fn drop(&mut self) {
		let ended_at = Instant::now();
		self.estimator.update(self.started_at, ended_at, self.work);
	}
}

/// Estimates the quantity of work the underlying service can handle over a period of time.
///
/// Each request is decorated with a `Handle` that measures the time necessary to process the
/// request and, on drop, updates the rate estimator on which it holds a reference.
#[derive(Debug, Clone)]
pub struct EstimateRate<S, T> {
	service: S,
	estimator: T,
}

impl<S, T> EstimateRate<S, T>
where
	T: RateEstimator,
{
	/// Creates a new rate estimator.
	pub fn new(service: S, estimator: T) -> Self {
		Self { service, estimator }
	}

	fn handle(&self, work: u64) -> Handle<T> {
		Handle { started_at: Instant::now(), work, estimator: self.estimator.clone() }
	}
}

impl<S, R, T> Service<R> for EstimateRate<S, T>
where
	S: Service<R>,
	R: Cost,
	T: RateEstimator,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = TrackCompletionFuture<S::Future, CompleteOnResponse, Handle<T>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&mut self, request: R) -> Self::Future {
		let handle = self.handle(request.cost());
		TrackCompletionFuture::new(
			CompleteOnResponse::default(),
			handle,
			self.service.call(request),
		)
	}
}

/// Estimates the quantity of work the underlying
/// service can handle over a period of time.
#[derive(Debug, Clone)]
pub struct EstimateRateLayer<R, T> {
	estimator: T,
	_phantom: PhantomData<R>,
}

impl<R, T> EstimateRateLayer<R, T> {
	/// Creates new estimate rate layer.
	pub fn new(estimator: T) -> Self {
		Self { estimator, _phantom: PhantomData }
	}
}

impl<S, R, T> Layer<S> for EstimateRateLayer<R, T>
where
	S: Service<R>,
	R: Cost,
	T: RateEstimator,
{
	type Service = EstimateRate<S, T>;

	fn layer(&self, service: S) -> Self::Service {
		EstimateRate::new(service, self.estimator.clone())
	}
}

#[cfg(test)]
mod tests {
	use std::{
		sync::{
			atomic::{AtomicU64, Ordering},
			Arc,
		},
		time::Duration,
	};

	use tower::ServiceExt;

	use super::*;
	use crate::tower::Rate;

	struct Request;

	impl Cost for Request {
		fn cost(&self) -> u64 {
			42
		}
	}

	#[derive(Debug, Clone, Default)]
	struct DummyEstimator {
		work: Arc<AtomicU64>,
		duration_micros: Arc<AtomicU64>,
	}

	impl Rate for DummyEstimator {
		fn work(&self) -> u64 {
			self.work.load(Ordering::Relaxed)
		}

		fn period(&self) -> Duration {
			Duration::from_micros(self.duration_micros.load(Ordering::Relaxed))
		}
	}

	impl RateEstimator for DummyEstimator {
		fn update(&mut self, started_at: Instant, ended_at: Instant, work: u64) {
			self.work.store(work, Ordering::Relaxed);
			self.duration_micros
				.store((ended_at - started_at).as_micros() as u64, Ordering::Relaxed);
		}
	}

	#[tokio::test]
	async fn test_estimate_rate() {
		let estimator = DummyEstimator::default();
		let mut service = EstimateRate::new(
			tower::service_fn(|_: Request| async move { Ok::<_, ()>(()) }),
			estimator.clone(),
		);
		service.ready().await.unwrap().call(Request).await.unwrap();
		assert_eq!(service.estimator.work(), 42);
	}
}
