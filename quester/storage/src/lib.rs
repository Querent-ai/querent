pub mod storage;
use std::{collections::HashMap, sync::Arc};

use common::{
	storage_config::{MilvusConfig, Neo4jConfig, PostgresConfig},
	StorageConfig, StorageConfigs, StorageType,
};
use querent_synapse::callbacks::EventType;
pub use storage::*;
pub mod vector;
pub use vector::*;
pub mod graph;
pub use graph::*;
pub mod index;
pub use index::*;

pub async fn create_storages(
	storage_configs: &StorageConfigs,
) -> anyhow::Result<(HashMap<EventType, Arc<dyn Storage>>, Vec<Arc<dyn Storage>>)> {
	let mut event_storages: HashMap<EventType, Arc<dyn Storage>> = HashMap::new();
	let mut index_storages: Vec<Arc<dyn Storage>> = Vec::new();
	for storage_config in storage_configs.0.iter() {
		match storage_config {
			StorageConfig::Postgres(backend) => {
				if backend.storage_type != StorageType::Index {
					return Err(anyhow::anyhow!("Postgres storage type must be index"));
				}
				let postgres_config =
					PostgresConfig { url: backend.config.get("url").unwrap().to_string() };
				let postgres = PostgresStorage::new(postgres_config).await.map_err(|err| {
					log::error!("Postgres client creation failed: {:?}", err);
					err
				});

				match postgres {
					Ok(postgres) => {
						let postgres = Arc::new(postgres);
						index_storages.push(postgres.clone());
					},
					Err(err) => {
						log::error!("Postgres client creation failed: {:?}", err);
					},
				}
			},
			StorageConfig::Milvus(backend) => {
				let vector_store_config = MilvusConfig {
					url: backend.config.get("url").unwrap().to_string(),
					username: backend.config.get("username").unwrap().to_string(),
					password: backend.config.get("password").unwrap().to_string(),
				};

				let milvus = MilvusStorage::new(vector_store_config).await.map_err(|err| {
					log::error!("Milvus client creation failed: {:?}", err);
					err
				});
				match milvus {
					Ok(milvus) => {
						let milvus = Arc::new(milvus);
						event_storages.insert(EventType::Vector, milvus.clone());
					},
					Err(err) => {
						log::error!("Milvus client creation failed: {:?}", err);
					},
				}
			},
			StorageConfig::Neo4j(backend) => {
				let graph_storage_config = Neo4jConfig {
					db_name: backend.config.get("db_name").unwrap().to_string(),
					url: backend.config.get("url").unwrap().to_string(),
					username: backend.config.get("username").unwrap().to_string(),
					password: backend.config.get("password").unwrap().to_string(),
					max_connections: backend
						.config
						.get("max_connections")
						.unwrap_or(&"10".to_string())
						.parse::<usize>()
						.unwrap(),
					fetch_size: backend
						.config
						.get("fetch_size")
						.unwrap_or(&"1000".to_string())
						.parse::<usize>()
						.unwrap(),
				};
				let neo4j = Neo4jStorage::new(graph_storage_config).await.map_err(|err| {
					log::error!("Neo4j client creation failed: {:?}", err);
					err
				});

				match neo4j {
					Ok(neo4j) => {
						let neo4j = Arc::new(neo4j);
						event_storages.insert(EventType::Graph, neo4j.clone());
					},
					Err(err) => {
						log::error!("Neo4j client creation failed: {:?}", err);
					},
				}
			},
		}
	}
	Ok((event_storages, index_storages))
}
