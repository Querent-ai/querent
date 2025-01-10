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

use colored::Colorize;
use common::RED_COLOR;
use node::{
	cli::{build_cli, setup_logging_and_tracing, CliCommand},
	serve::build_info::BuildInfo,
	tokio_runtime,
};
use opentelemetry::global;

fn main() -> Result<(), anyhow::Error> {
	let runtime = tokio_runtime();
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install ring as the default crypto provider");
	match runtime {
		Ok(runtime) => {
			let _ = runtime
				.block_on(main_impl())
				.map_err(|e| anyhow::anyhow!("Main loop failed: {:?}", e));
			Ok(())
		},
		Err(e) => {
			log::error!("Failed to initialize tokio runtime: {:?}", e);
			Err(e)
		},
	}
}

async fn main_impl() -> Result<(), anyhow::Error> {
	setup_logging_and_tracing();
	#[cfg(feature = "openssl-support")]
	openssl_probe::init_ssl_cert_env_vars();

	let about_text = about_text();
	let build_info = BuildInfo::get();
	let version = format!(
		"{} ({} {})",
		build_info.version, build_info.commit_short_hash, build_info.build_date
	);
	log::info!("Starting Querent RIAN Immersive Intelligence Node 🧠 {}", version);
	let app = build_cli().about(about_text).version(version);
	let matches = app.get_matches();
	let command = match CliCommand::parse_cli_args(matches) {
		Ok(command) => command,
		Err(err) => {
			eprintln!("Failed to parse command arguments: {err:?}");
			std::process::exit(1);
		},
	};

	let return_code: i32 = if let Err(err) = command.execute().await {
		eprintln!("{} Command failed: {:?}\n", "✘".color(RED_COLOR), err);
		1
	} else {
		0
	};

	global::shutdown_tracer_provider();
	std::process::exit(return_code)
}

/// Return the about text with telemetry info.
fn about_text() -> String {
	let about_text = String::from(
        "Querent: Asynchronous data dynamo for multi-model deep neural networks workflows.\n  Find more information at https://querent.xyz/docs\n\n",
    );
	about_text
}
