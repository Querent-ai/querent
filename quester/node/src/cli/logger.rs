use std::env;

use anyhow::Context;
use opentelemetry::{global, sdk::propagation::TraceContextPropagator};
use tracing::Level;
use tracing_subscriber::{fmt::time::UtcTime, prelude::*, EnvFilter};

use crate::BuildInfo;

pub fn setup_logging_and_tracing(
	level: Level,
	ansi_colors: bool,
	_build_info: &BuildInfo,
) -> anyhow::Result<()> {
	let env_filter = env::var("RUST_LOG")
		.map(|_| EnvFilter::from_default_env())
		.or_else(|_| EnvFilter::try_new(format!("querent={level}")))
		.context("Failed to set up tracing env filter.")?;
	global::set_text_map_propagator(TraceContextPropagator::new());
	let registry = tracing_subscriber::registry().with(env_filter);
	let event_format = tracing_subscriber::fmt::format().with_target(true).with_timer(
		// We do not rely on the Rfc3339 implementation, because it has a nanosecond precision.
		// See discussion here: https://github.com/time-rs/time/discussions/418
		UtcTime::new(
			time::format_description::parse(
				"[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
			)
			.expect("Time format invalid."),
		),
	);
	registry
		.with(
			tracing_subscriber::fmt::layer()
				.event_format(event_format)
				.with_ansi(ansi_colors),
		)
		.try_init()
		.context("Failed to set up tracing.")?;

	Ok(())
}
