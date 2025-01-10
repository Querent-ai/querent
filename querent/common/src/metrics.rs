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
	borrow::{Borrow, Cow},
	collections::HashMap,
};

use prometheus::{Encoder, HistogramOpts, Opts, TextEncoder};
pub use prometheus::{
	Histogram, HistogramTimer, HistogramVec as PrometheusHistogramVec, IntCounter,
	IntCounterVec as PrometheusIntCounterVec, IntGauge, IntGaugeVec as PrometheusIntGaugeVec,
};

pub struct OwnedPrometheusLabels<const N: usize> {
	labels: [Cow<'static, str>; N],
}

impl<const N: usize> OwnedPrometheusLabels<N> {
	pub fn new(labels: [Cow<'static, str>; N]) -> Self {
		Self { labels }
	}

	pub fn borrow_labels(&self) -> [&str; N] {
		let mut labels = [""; N];

		for (i, label) in self.labels.iter().enumerate() {
			labels[i] = label.borrow();
		}
		labels
	}
}

pub trait PrometheusLabels<const N: usize> {
	fn labels(&self) -> OwnedPrometheusLabels<N>;
}

#[derive(Clone)]
pub struct HistogramVec<const N: usize> {
	underlying: PrometheusHistogramVec,
}

impl<const N: usize> HistogramVec<N> {
	pub fn with_label_values(&self, label_values: [&str; N]) -> Histogram {
		self.underlying.with_label_values(&label_values)
	}
}

#[derive(Clone)]
pub struct IntCounterVec<const N: usize> {
	underlying: PrometheusIntCounterVec,
}

impl<const N: usize> IntCounterVec<N> {
	pub fn with_label_values(&self, label_values: [&str; N]) -> IntCounter {
		self.underlying.with_label_values(&label_values)
	}
}

#[derive(Clone)]
pub struct IntGaugeVec<const N: usize> {
	underlying: PrometheusIntGaugeVec,
}

impl<const N: usize> IntGaugeVec<N> {
	pub fn with_label_values(&self, label_values: [&str; N]) -> IntGauge {
		self.underlying.with_label_values(&label_values)
	}
}

pub fn new_counter(name: &str, description: &str, namespace: &str) -> IntCounter {
	let counter_opts = Opts::new(name, description).namespace(namespace);
	let counter = IntCounter::with_opts(counter_opts).expect("Failed to create counter");
	prometheus::register(Box::new(counter.clone())).expect("Failed to register counter");
	counter
}

pub fn counter_vec<const N: usize>(
	name: &str,
	help: &str,
	subsystem: &str,
	const_labels: &[(&str, &str)],
	label_names: [&str; N],
) -> IntCounterVec<N> {
	let owned_const_labels: HashMap<String, String> = const_labels
		.iter()
		.map(|(label_name, label_value)| (label_name.to_string(), label_value.to_string()))
		.collect();
	let counter_opts = Opts::new(name, help)
		.namespace("quickwit")
		.subsystem(subsystem)
		.const_labels(owned_const_labels);
	let underlying = PrometheusIntCounterVec::new(counter_opts, &label_names)
		.expect("failed to create counter vec");

	let collector = Box::new(underlying.clone());
	prometheus::register(collector).expect("failed to register counter vec");

	IntCounterVec { underlying }
}

pub fn new_gauge(name: &str, description: &str, namespace: &str) -> IntGauge {
	let gauge_opts = Opts::new(name, description).namespace(namespace);
	let gauge = IntGauge::with_opts(gauge_opts).expect("Failed to create gauge");
	prometheus::register(Box::new(gauge.clone())).expect("Failed to register gauge");
	gauge
}

pub fn new_histogram(name: &str, description: &str, namespace: &str) -> Histogram {
	let histogram_opts = HistogramOpts::new(name, description).namespace(namespace);
	let histogram = Histogram::with_opts(histogram_opts).expect("Failed to create histogram");
	prometheus::register(Box::new(histogram.clone())).expect("Failed to register counter");
	histogram
}

pub fn histogram_vec<const N: usize>(
	name: &str,
	help: &str,
	subsystem: &str,
	const_labels: &[(&str, &str)],
	label_names: [&str; N],
) -> HistogramVec<N> {
	let owned_const_labels: HashMap<String, String> = const_labels
		.iter()
		.map(|(label_name, label_value)| (label_name.to_string(), label_value.to_string()))
		.collect();
	let histogram_opts = HistogramOpts::new(name, help)
		.namespace("quickwit")
		.subsystem(subsystem)
		.const_labels(owned_const_labels);
	let underlying = PrometheusHistogramVec::new(histogram_opts, &label_names)
		.expect("failed to create histogram vec");

	let collector = Box::new(underlying.clone());
	prometheus::register(collector).expect("failed to register histogram vec");

	HistogramVec { underlying }
}

pub fn gauge_vec<const N: usize>(
	name: &str,
	help: &str,
	subsystem: &str,
	const_labels: &[(&str, &str)],
	label_names: [&str; N],
) -> IntGaugeVec<N> {
	let owned_const_labels: HashMap<String, String> = const_labels
		.iter()
		.map(|(label_name, label_value)| (label_name.to_string(), label_value.to_string()))
		.collect();
	let gauge_opts = Opts::new(name, help)
		.namespace("quickwit")
		.subsystem(subsystem)
		.const_labels(owned_const_labels);
	let underlying =
		PrometheusIntGaugeVec::new(gauge_opts, &label_names).expect("failed to create gauge vec");

	let collector = Box::new(underlying.clone());
	prometheus::register(collector).expect("failed to register counter vec");

	IntGaugeVec { underlying }
}

pub struct GaugeGuard(&'static IntGauge);

impl GaugeGuard {
	pub fn from_gauge(gauge: &'static IntGauge) -> Self {
		gauge.inc();
		Self(gauge)
	}
}

impl Drop for GaugeGuard {
	fn drop(&mut self) {
		self.0.dec();
	}
}

pub fn metrics_text_payload() -> String {
	let metric_families = prometheus::gather();
	let mut buffer = Vec::new();
	let encoder = TextEncoder::new();
	let _ = encoder.encode(&metric_families, &mut buffer); // TODO avoid ignoring the error.
	String::from_utf8_lossy(&buffer).to_string()
}
