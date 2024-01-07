use common::metrics::{new_counter, IntCounter};
use once_cell::sync::Lazy;

pub struct RestMetrics {
	pub http_requests_total: IntCounter,
}

impl Default for RestMetrics {
	fn default() -> Self {
		RestMetrics {
			http_requests_total: new_counter(
				"http_requests_total",
				"Total number of HTTP requests received",
				"quickwit",
			),
		}
	}
}

/// Serve counters exposes a bunch a set of metrics about the request received to quickwit.
pub static SERVE_METRICS: Lazy<RestMetrics> = Lazy::new(RestMetrics::default);
