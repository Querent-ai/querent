use colored::Colorize;
use common::RED_COLOR;
use node::{
	cli::{build_cli, busy_detector, setup_logging_and_tracing, CliCommand},
	serve::build_info::BuildInfo,
};
use once_cell::sync::OnceCell;
use opentelemetry::global;
use tokio::runtime::{Builder, Runtime};
use tracing::{error, info, trace};

pub fn tokio_runtime() -> Result<&'static Runtime, anyhow::Error> {
	static RUNTIME: OnceCell<Runtime> = OnceCell::new();

	RUNTIME.get_or_try_init(|| {
		Builder::new_multi_thread()
			.enable_all()
			.on_thread_unpark(busy_detector::thread_unpark)
			.on_thread_park(busy_detector::thread_park)
			.build()
			.map_err(|err| anyhow::anyhow!("Failed to create tokio runtime: {}", err))
	})
}

fn main() -> Result<(), anyhow::Error> {
	let runtime = tokio_runtime();
	match runtime {
		Ok(runtime) => {
			info!("⚙️ Querent Runtime initialized.");
			trace!("Starting main loop");
			let _ = runtime
				.block_on(main_impl())
				.map_err(|e| anyhow::anyhow!("Main loop failed: {:?}", e));
			info!("Querent node stopped.");
			Ok(())
		},
		Err(e) => {
			error!("Failed to initialize tokio runtime: {:?}", e);
			Err(e)
		},
	}
}

async fn main_impl() -> Result<(), anyhow::Error> {
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
	setup_logging_and_tracing(command.default_log_level(), true, build_info)
		.map_err(|e| anyhow::anyhow!("Failed to set up logging and tracing: {:?}", e))?;

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
