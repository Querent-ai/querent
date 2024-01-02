use querent_synapse::config::Config;

use crate::StorageConfig;

#[derive(Debug, Clone)]
pub struct QuesterConfig {
	/// The id of the qflow, a single qflow pipeline runs a single qflow.
	pub qflow_id: String,
	/// The configuration for Querent.
	pub qflow_config: Config,
	/// The Storage configuration for Quester
	pub storage_config: StorageConfig,
}

impl Default for QuesterConfig {
	fn default() -> Self {
		Self {
			qflow_id: "qflow".to_string(),
			qflow_config: Config::default(),
			storage_config: StorageConfig::Postgres(crate::PostgresConfig {
				url: "postgresql://user:password@host:port/database".to_string(),
			}),
		}
	}
}
