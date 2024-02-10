use colored::Colorize;
use common::RED_COLOR;
use node::{
	cli::{build_cli, setup_logging_and_tracing, CliCommand},
	serve::build_info::BuildInfo,
};
use opentelemetry::global;
use querent_synapse::{
	querent::{py_runtime_init, QuerentError},
	tokio_runtime,
};
use tracing::{error, info, trace};

fn main() -> Result<(), QuerentError> {
	let runtime = tokio_runtime();
	match runtime {
		Ok(runtime) => {
			// Initialize the Python runtime
			let python_runtime_res = py_runtime_init();
			match python_runtime_res {
				Ok(_) => {
					info!("Python runtime initialized.");
					trace!("Starting main loop");
					let _ = runtime
						.block_on(main_impl())
						.map_err(|e| QuerentError::internal(e.to_string()));
				},
				Err(e) => {
					error!("Failed to initialize Python runtime: {}", e);
					return Err(QuerentError::internal(format!(
						"Failed to initialize Python runtime: {}",
						e
					)));
				},
			}
			Ok(())
		},
		Err(err) => Err(err),
	}
}

async fn main_impl() -> Result<(), QuerentError> {
	#[cfg(feature = "openssl-support")]
	openssl_probe::init_ssl_cert_env_vars();

	let about_text = about_text();
	let build_info = BuildInfo::get();
	let version = format!(
		"{} ({} {})",
		build_info.version, build_info.commit_short_hash, build_info.build_date
	);
	let app = build_cli().about(about_text).version(version);
	let matches = app.get_matches();
	let command = match CliCommand::parse_cli_args(matches) {
		Ok(command) => command,
		Err(err) => {
			eprintln!("Failed to parse command arguments: {err:?}");
			std::process::exit(1);
		},
	};
	setup_logging_and_tracing(command.default_log_level(), true, build_info).map_err(|e| {
		QuerentError::internal(format!("Failed to set up logging and tracing: {}", e))
	})?;

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
