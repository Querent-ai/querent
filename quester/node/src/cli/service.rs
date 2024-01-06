use clap::{ArgMatches, Command};
use common::{initialize_runtimes, RuntimesConfig};
use tokio::signal;
use tracing::debug;

use crate::{cli::load_node_config, serve_quester};

pub fn build_serve_command() -> Command {
	Command::new("serve")
		.about("Starts a Querent node.")
		.long_about("Starts a Querent node with all services enabled by default.")
}

#[derive(Debug, Eq, PartialEq)]
pub struct Serve {
	node_config_uri: String,
}

impl Serve {
	pub fn parse_cli_args(mut matches: ArgMatches) -> anyhow::Result<Self> {
		let config_uri = matches
			.remove_one::<String>("node-config")
			.expect("`node config` should be a required arg.");
		Ok(Serve { node_config_uri: config_uri })
	}

	pub async fn execute(&self) -> anyhow::Result<()> {
		debug!(args = ?self, "run-quester-service");
		querent_synapse::busy_detector::set_enabled(true);
		let node_config = load_node_config(&self.node_config_uri).await?;

		let runtimes_config = RuntimesConfig::default();
		initialize_runtimes(runtimes_config)?;
		let shutdown_signal = Box::pin(async move {
			signal::ctrl_c()
				.await
				.expect("Registering a signal handler for SIGINT should not fail.");
		});
		let serve_result = serve_quester(node_config, runtimes_config, shutdown_signal).await;
		let _return_code = match serve_result {
			Ok(_) => 0,
			Err(_) => 1,
		};
		serve_result?;
		Ok(())
	}
}
