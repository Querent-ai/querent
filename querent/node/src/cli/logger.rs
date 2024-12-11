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
