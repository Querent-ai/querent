#[cfg(feature = "console")]
pub fn setup_logging_and_tracing() {
	println!("Console Subscriber is enabled!");
	console_subscriber::init();
}

#[cfg(not(feature = "console"))]
pub fn setup_logging_and_tracing() {
	use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "info,tower_http=debug".into()),
		)
		.with(tracing_subscriber::fmt::layer().with_thread_ids(true))
		.init();
}
