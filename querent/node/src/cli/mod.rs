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

pub mod cli_main;
pub use cli_main::*;
pub mod logger;
pub use logger::*;
use proto::config::NodeConfig;
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
