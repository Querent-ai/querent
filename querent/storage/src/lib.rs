pub mod storage;
use common::EventType;
use proto::semantics::{StorageConfig, StorageType};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
pub use storage::*;
use surrealdb::surrealdb::SurrealDB;
pub mod vector;
use tracing::info;
pub use vector::*;
pub mod graph;
pub use graph::*;
pub mod index;
pub use index::*;
pub mod secret;
pub use secret::*;
pub mod metastore;
pub mod surrealdb;
pub use metastore::*;

use diesel::result::{Error as DieselError, Error::QueryBuilderError};

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::deadpool::{Object as PooledConnection, Pool},
};
use std::{
	ops::{Deref, DerefMut},
	time::Duration,
};

pub use crate::{Storage, StorageError, StorageErrorKind, StorageResult};

pub async fn create_storages(
	storage_configs: &[StorageConfig],
	path_buf: PathBuf,
) -> anyhow::Result<(HashMap<EventType, Vec<Arc<dyn Storage>>>, Vec<Arc<dyn Storage>>)> {
	let mut event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>> = HashMap::new();
	let mut index_storages: Vec<Arc<dyn Storage>> = Vec::new();

	if storage_configs.len() == 0 {
		let surreal_db = create_default_storage(path_buf).await.map_err(|err| err)?;

		event_storages
			.entry(EventType::Vector)
			.or_insert_with(Vec::new)
			.push(surreal_db.clone());

		index_storages.push(surreal_db);
	}

	for storage_config in storage_configs {
		match storage_config {
			StorageConfig { postgres: Some(config), .. } => {
				if config.storage_type.is_none() {
					return Err(anyhow::anyhow!(
						"Invalid storage type: Postgres is only supported for index storage"
					));
				}
				if config.storage_type.clone().unwrap() == StorageType::Index {
					let postgres = PostgresStorage::new(config.clone()).await.map_err(|err| {
						log::error!("Postgres client creation failed: {:?}", err);
						err
					})?;

					postgres.check_connectivity().await?;
					let postgres = Arc::new(postgres);
					index_storages.push(postgres);
				} else if config.storage_type.clone().unwrap() == StorageType::Vector {
					let postgres = PGVector::new(config.clone()).await.map_err(|err| {
						log::error!("Postgres client creation failed: {:?}", err);
						err
					})?;

					postgres.check_connectivity().await?;
					let postgres = Arc::new(postgres);
					event_storages
						.entry(EventType::Vector)
						.or_insert_with(Vec::new)
						.push(postgres.clone());
				}
			},
			StorageConfig { milvus: Some(config), .. } => {
				let milvus = MilvusStorage::new(config.clone()).await.map_err(|err| {
					log::error!("Milvus client creation failed: {:?}", err);
					err
				})?;

				milvus.check_connectivity().await?;
				let milvus = Arc::new(milvus);
				event_storages.entry(EventType::Vector).or_insert_with(Vec::new).push(milvus);
			},
			StorageConfig { neo4j: Some(config), .. } => {
				let neo4j = Neo4jStorage::new(config.clone()).await.map_err(|err| {
					log::error!("Neo4j client creation failed: {:?}", err);
					err
				})?;

				neo4j.check_connectivity().await?;
				let neo4j = Arc::new(neo4j);
				event_storages.entry(EventType::Graph).or_insert_with(Vec::new).push(neo4j);
			},
			// Handle other storage types if necessary
			_ => {}, // Ignore or handle unexpected storage configs
		}
	}

	info!("Storages created successfully ✅");
	Ok((event_storages, index_storages))
}

pub async fn create_default_storage(path: std::path::PathBuf) -> anyhow::Result<Arc<dyn Storage>> {
	let surreal_db = SurrealDB::new(path).await.map_err(|err| {
		log::error!("Surreal client creation failed: {:?}", err);
		err
	})?;
	let _ = surreal_db.check_connectivity().await;
	let surreal_db = Arc::new(surreal_db);
	Ok(surreal_db)
}

pub async fn create_secret_store(
	path: std::path::PathBuf,
) -> anyhow::Result<Arc<dyn SecretStorage>> {
	let secret_store = SecretStore::new(path);
	let secret_store = Arc::new(secret_store);
	Ok(secret_store)
}

pub async fn create_metadata_store(path: std::path::PathBuf) -> anyhow::Result<Arc<dyn Storage>> {
	let metastore = MetaStore::new(path);
	metastore.check_connectivity().await?;
	let metastore = Arc::new(metastore);
	Ok(metastore)
}

pub type ActualDbPool = Pool<AsyncPgConnection>;
pub enum DbPool<'a> {
	Pool(&'a ActualDbPool),
	Conn(&'a mut AsyncPgConnection),
}

pub enum DbConn<'a> {
	Pool(PooledConnection<AsyncPgConnection>),
	Conn(&'a mut AsyncPgConnection),
}

pub async fn get_conn<'a, 'b: 'a>(pool: &'a mut DbPool<'b>) -> Result<DbConn<'a>, DieselError> {
	Ok(match pool {
		DbPool::Pool(pool) =>
			DbConn::Pool(pool.get().await.map_err(|e| QueryBuilderError(e.into()))?),
		DbPool::Conn(conn) => DbConn::Conn(conn),
	})
}

impl<'a> Deref for DbConn<'a> {
	type Target = AsyncPgConnection;

	fn deref(&self) -> &Self::Target {
		match self {
			DbConn::Pool(conn) => conn.deref(),
			DbConn::Conn(conn) => conn.deref(),
		}
	}
}

impl<'a> DerefMut for DbConn<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		match self {
			DbConn::Pool(conn) => conn.deref_mut(),
			DbConn::Conn(conn) => conn.deref_mut(),
		}
	}
}

// Allows functions that take `DbPool<'_>` to be called in a transaction by passing `&mut conn.into()`
impl<'a> From<&'a mut AsyncPgConnection> for DbPool<'a> {
	fn from(value: &'a mut AsyncPgConnection) -> Self {
		DbPool::Conn(value)
	}
}

impl<'a, 'b: 'a> From<&'a mut DbConn<'b>> for DbPool<'a> {
	fn from(value: &'a mut DbConn<'b>) -> Self {
		DbPool::Conn(value.deref_mut())
	}
}

impl<'a> From<&'a ActualDbPool> for DbPool<'a> {
	fn from(value: &'a ActualDbPool) -> Self {
		DbPool::Pool(value)
	}
}

pub const FETCH_LIMIT_MAX: i64 = 50;
pub const SITEMAP_LIMIT: i64 = 50000;
pub const SITEMAP_DAYS: i64 = 31;
const POOL_TIMEOUT: Option<Duration> = Some(Duration::from_secs(50));
