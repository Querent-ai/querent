pub mod cli_main;
pub use cli_main::*;
pub mod logger;
use common::NodeConfig;
pub use logger::*;
pub mod service;
use anyhow::Context;
pub use service::*;
use tokio::fs;
use tracing::info;

/// Loads a node config located at `config_uri` with the default storage configuration.
pub async fn load_node_config(config_uri: &String) -> anyhow::Result<NodeConfig> {
	// Read the content of the configuration file
	let config_content = fs::read(config_uri)
		.await
		.with_context(|| format!("failed to read node config file at `{}`", config_uri))?;

	// Deserialize YAML content into NodeConfig struct
	let config: NodeConfig = serde_yaml::from_slice(&config_content)
		.with_context(|| format!("failed to parse node config YAML at `{}`", config_uri))?;

	// Log information about the loaded config
	info!(config_uri=%config_uri, config=?config, "loaded node config");

	Ok(config)
}
