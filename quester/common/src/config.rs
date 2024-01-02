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
