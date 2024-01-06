use std::{
	pin::Pin,
	task::{Context, Poll},
	time::Instant,
};

use futures::{ready, Future};
use pin_project::pin_project;
use tower::{Layer, Service};

use crate::metrics::{
	new_counter_vec, new_histogram_vec, HistogramVec, IntCounterVec, OwnedPrometheusLabels,
	PrometheusLabels,
};

#[derive(Clone)]
pub struct PrometheusMetrics<const N: usize, S> {
	inner: S,
	requests_total: IntCounterVec<N>,
	request_errors_total: IntCounterVec<N>,
	request_duration_seconds: HistogramVec<N>,
}

impl<const N: usize, S, R> Service<R> for PrometheusMetrics<N, S>
where
	S: Service<R>,
	R: PrometheusLabels<N>,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = ResponseFuture<N, S::Future>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: R) -> Self::Future {
		let labels = request.labels();
		self.requests_total.with_label_values(labels.borrow_labels()).inc();
		let start = Instant::now();
		let inner = self.inner.call(request);
		ResponseFuture {
			inner,
			start,
			labels,
			request_errors_total: self.request_errors_total.clone(),
			request_duration_seconds: self.request_duration_seconds.clone(),
		}
	}
}

#[derive(Clone)]
pub struct PrometheusMetricsLayer<const N: usize> {
	requests_total: IntCounterVec<N>,
	request_errors_total: IntCounterVec<N>,
	request_duration_seconds: HistogramVec<N>,
}

impl<const N: usize> PrometheusMetricsLayer<N> {
	pub fn new(namespace: &'static str, label_names: [&'static str; N]) -> Self {
		Self {
			requests_total: new_counter_vec(
				"requests_total",
				"Total number of requests",
				namespace,
				label_names,
			),
			request_errors_total: new_counter_vec(
				"request_errors_total",
				"Total number of failed requests",
				namespace,
				label_names,
			),
			request_duration_seconds: new_histogram_vec(
				"request_duration_seconds",
				"Duration of request in seconds",
				namespace,
				label_names,
			),
		}
	}
}

impl<const N: usize, S> Layer<S> for PrometheusMetricsLayer<N> {
	type Service = PrometheusMetrics<N, S>;

	fn layer(&self, inner: S) -> Self::Service {
		PrometheusMetrics {
			inner,
			requests_total: self.requests_total.clone(),
			request_errors_total: self.request_errors_total.clone(),
			request_duration_seconds: self.request_duration_seconds.clone(),
		}
	}
}

/// Response future for [`PrometheusMetrics`].
#[pin_project]
pub struct ResponseFuture<const N: usize, F> {
	#[pin]
	inner: F,
	start: Instant,
	labels: OwnedPrometheusLabels<N>,
	request_errors_total: IntCounterVec<N>,
	request_duration_seconds: HistogramVec<N>,
}

impl<const N: usize, F, T, E> Future for ResponseFuture<N, F>
where
	F: Future<Output = Result<T, E>>,
{
	type Output = Result<T, E>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		let response = ready!(this.inner.poll(cx));
		let elapsed = this.start.elapsed();

		if response.is_err() {
			this.request_errors_total.with_label_values(this.labels.borrow_labels()).inc();
		}
		this.request_duration_seconds
			.with_label_values(this.labels.borrow_labels())
			.observe(elapsed.as_secs_f64());

		Poll::Ready(Ok(response?))
	}
}
