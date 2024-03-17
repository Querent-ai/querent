pub mod storage;
use proto::semantics::{StorageConfig, StorageType};
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
pub use storage::*;
pub mod vector;
use tracing::info;
pub use vector::*;
pub mod graph;
pub use graph::*;
pub mod index;
pub use index::*;

pub async fn create_storages(
	storage_configs: &[StorageConfig],
) -> anyhow::Result<(HashMap<EventType, Arc<dyn Storage>>, Vec<Arc<dyn Storage>>)> {
	let mut event_storages: HashMap<EventType, Arc<dyn Storage>> = HashMap::new();
	let mut index_storages: Vec<Arc<dyn Storage>> = Vec::new();

	for storage_config in storage_configs {
		match storage_config {
			StorageConfig { postgres: Some(config), .. } => {
				if config.storage_type != StorageType::Index as i32 {
					return Err(anyhow::anyhow!(
						"Invalid storage type: Postgres is only supported for index storage"
					));
				}
				let postgres = PostgresStorage::new(config.clone()).await.map_err(|err| {
					log::error!("Postgres client creation failed: {:?}", err);
					err
				})?;

				postgres.check_connectivity().await?;
				let postgres = Arc::new(postgres);
				index_storages.push(postgres);
			},
			StorageConfig { milvus: Some(config), .. } => {
				let milvus = MilvusStorage::new(config.clone()).await.map_err(|err| {
					log::error!("Milvus client creation failed: {:?}", err);
					err
				})?;

				milvus.check_connectivity().await?;
				let milvus = Arc::new(milvus);
				event_storages.insert(EventType::Vector, milvus);
			},
			StorageConfig { neo4j: Some(config), .. } => {
				let neo4j = Neo4jStorage::new(config.clone()).await.map_err(|err| {
					log::error!("Neo4j client creation failed: {:?}", err);
					err
				})?;

				neo4j.check_connectivity().await?;
				let neo4j = Arc::new(neo4j);
				event_storages.insert(EventType::Graph, neo4j);
			},
			// Handle other storage types if necessary
			_ => {}, // Ignore or handle unexpected storage configs
		}
	}

	info!("Storages created successfully ✅");
	Ok((event_storages, index_storages))
}
