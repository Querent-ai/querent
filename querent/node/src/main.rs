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
