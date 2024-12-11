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
	time::Instant,
};

use futures::{ready, Future};
use pin_project::{pin_project, pinned_drop};
use tower::{Layer, Service};

use crate::metrics::{
	counter_vec, gauge_vec, histogram_vec, HistogramVec, IntCounterVec, IntGaugeVec,
};

pub trait RpcName {
	fn rpc_name() -> &'static str;
}

#[derive(Clone)]
pub struct GrpcMetrics<S> {
	inner: S,
	requests_total: IntCounterVec<2>,
	requests_in_flight: IntGaugeVec<1>,
	request_duration_seconds: HistogramVec<2>,
}

impl<S, R> Service<R> for GrpcMetrics<S>
where
	S: Service<R>,
	R: RpcName,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = ResponseFuture<S::Future>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: R) -> Self::Future {
		let start = Instant::now();
		let rpc_name = R::rpc_name();
		let inner = self.inner.call(request);

		self.requests_in_flight.with_label_values([rpc_name]).inc();

		ResponseFuture {
			inner,
			start,
			rpc_name,
			status: "canceled",
			requests_total: self.requests_total.clone(),
			requests_in_flight: self.requests_in_flight.clone(),
			request_duration_seconds: self.request_duration_seconds.clone(),
		}
	}
}

#[derive(Clone)]
pub struct GrpcMetricsLayer {
	requests_total: IntCounterVec<2>,
	requests_in_flight: IntGaugeVec<1>,
	request_duration_seconds: HistogramVec<2>,
}

impl GrpcMetricsLayer {
	pub fn new(subsystem: &'static str, kind: &'static str) -> Self {
		Self {
			requests_total: counter_vec(
				"grpc_requests_total",
				"Total number of gRPC requests processed.",
				subsystem,
				&[("kind", kind)],
				["rpc", "status"],
			),
			requests_in_flight: gauge_vec(
				"grpc_requests_in_flight",
				"Number of gRPC requests in-flight.",
				subsystem,
				&[("kind", kind)],
				["rpc"],
			),
			request_duration_seconds: histogram_vec(
				"grpc_request_duration_seconds",
				"Duration of request in seconds.",
				subsystem,
				&[("kind", kind)],
				["rpc", "status"],
			),
		}
	}
}

impl<S> Layer<S> for GrpcMetricsLayer {
	type Service = GrpcMetrics<S>;

	fn layer(&self, inner: S) -> Self::Service {
		GrpcMetrics {
			inner,
			requests_total: self.requests_total.clone(),
			requests_in_flight: self.requests_in_flight.clone(),
			request_duration_seconds: self.request_duration_seconds.clone(),
		}
	}
}

/// Response future for [`PrometheusMetrics`].
#[pin_project(PinnedDrop)]
pub struct ResponseFuture<F> {
	#[pin]
	inner: F,
	start: Instant,
	rpc_name: &'static str,
	status: &'static str,
	requests_total: IntCounterVec<2>,
	requests_in_flight: IntGaugeVec<1>,
	request_duration_seconds: HistogramVec<2>,
}

#[pinned_drop]
impl<F> PinnedDrop for ResponseFuture<F> {
	fn drop(self: Pin<&mut Self>) {
		let elapsed = self.start.elapsed().as_secs_f64();
		let label_values = [self.rpc_name, self.status];

		self.requests_total.with_label_values(label_values).inc();
		self.request_duration_seconds.with_label_values(label_values).observe(elapsed);
		self.requests_in_flight.with_label_values([self.rpc_name]).dec();
	}
}

impl<F, T, E> Future for ResponseFuture<F>
where
	F: Future<Output = Result<T, E>>,
{
	type Output = Result<T, E>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		let response = ready!(this.inner.poll(cx));
		*this.status = if response.is_ok() { "success" } else { "error" };
		Poll::Ready(Ok(response?))
	}
}
