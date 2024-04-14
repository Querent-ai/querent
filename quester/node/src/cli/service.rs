use clap::{ArgMatches, Command};
use common::{initialize_runtimes, RuntimesConfig};
use tokio::signal;
use tracing::{debug, info};

use crate::{cli::load_node_config, config_cli_arg, serve_quester};

pub fn build_serve_command() -> Command {
	Command::new("serve")
		.about("Starts a Querent node.")
		.long_about("Starts a Querent node with all services enabled by default.")
		.arg(config_cli_arg())
}

#[derive(Debug, Eq, PartialEq)]
pub struct Serve {
	node_config_uri: String,
}

impl Serve {
	pub fn parse_cli_args(mut matches: ArgMatches) -> anyhow::Result<Self> {
		let config_uri = matches.remove_one::<String>("config").expect("`node config required");
		Ok(Serve { node_config_uri: config_uri })
	}

	pub async fn execute(&self) -> anyhow::Result<()> {
		debug!(args = ?self, "run-querent-service");
		querent_synapse::busy_detector::set_enabled(true);
		let node_config = load_node_config(&self.node_config_uri).await?;

		let runtimes_config = RuntimesConfig::default();
		initialize_runtimes(runtimes_config)?;
		let shutdown_signal = Box::pin(async move {
			signal::ctrl_c()
				.await
				.expect("Registering a signal handler for SIGINT should not fail.");
		});
		info!("Starting Querent node");
		let serve_result = serve_quester(node_config, runtimes_config, shutdown_signal).await?;
		match serve_result.is_empty() {
			true => info!("Querent node has shut down"),
			false => info!("Querent node has shut down with errors"),
		}
		Ok(())
	}
}
