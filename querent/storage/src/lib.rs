pub mod storage;
use common::EventType;
use postgresql_embedded::{PostgreSQL, Settings, Status, VersionReq};
use proto::semantics::{StorageConfig, StorageType};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
pub use storage::*;
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
use diesel::result::{Error as DieselError, Error::QueryBuilderError};
pub use metastore::*;

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{
		deadpool::{Object as PooledConnection, Pool},
		AsyncDieselConnectionManager,
	},
	RunQueryDsl, SimpleAsyncConnection,
};
use std::{
	ops::{Deref, DerefMut},
	time::Duration,
};

pub use crate::{Storage, StorageError, StorageErrorKind, StorageResult};
const DB_NAME: &str = "querent_rian_node_v1";

pub async fn create_storages(
	storage_configs: &[StorageConfig],
	embedded_databasurl: Option<String>,
) -> anyhow::Result<(HashMap<EventType, Vec<Arc<dyn Storage>>>, Vec<Arc<dyn Storage>>)> {
	let mut event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>> = HashMap::new();
	let mut index_storages: Vec<Arc<dyn Storage>> = Vec::new();
	if storage_configs.len() == 0 {
		if embedded_databasurl.is_none() {
			return Err(anyhow::anyhow!("No storage configuration provided"));
		}
		let database_url = embedded_databasurl.unwrap();
		let pg_embed_db = create_default_storage(database_url).await.map_err(|err| err)?;

		event_storages
			.entry(EventType::Vector)
			.or_insert_with(Vec::new)
			.push(pg_embed_db.clone());

		index_storages.push(pg_embed_db);
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

pub async fn create_default_storage(database_url: String) -> anyhow::Result<Arc<dyn Storage>> {
	let embedded_db = PGEmbed::new(database_url).await.map_err(|err| {
		log::error!("embedded_db client creation failed: {:?}", err);
		err
	})?;
	let _ = embedded_db.check_connectivity().await;
	let embedded_db = Arc::new(embedded_db);
	Ok(embedded_db)
}

pub async fn start_postgres_embedded(path: PathBuf) -> Result<(PostgreSQL, String), StorageError> {
	let mut data_dir = path.clone();
	data_dir.push("rian_postgres_node");
	// create the data directory if it does not exist
	if !data_dir.exists() {
		std::fs::create_dir_all(&data_dir).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	}
	let mut password_dir = path.clone();
	password_dir.push("rian_postgres_password");
	// create the password directory if it does not exist
	if !password_dir.exists() {
		std::fs::create_dir_all(&password_dir).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	}
	let passwword_file_name = ".pgpass";
	let mut settings = Settings::new();
	settings.version = VersionReq::parse("=16.4.0").map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	settings.temporary = false;
	settings.data_dir = data_dir;
	settings.username = "postgres".to_string();
	settings.password = "postgres".to_string();
	settings.password_file = password_dir.join(passwword_file_name);
	let mut postgresql = PostgreSQL::new(settings);
	postgresql.setup().await.map_err(|e| StorageError {
		kind: StorageErrorKind::DatabaseInit,
		source: Arc::new(anyhow::Error::from(e)),
	})?;

	postgresql_extensions::install(
		postgresql.settings(),
		"portal-corp",
		"pgvector_compiled",
		&VersionReq::parse("=0.16.12").map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?,
	)
	.await
	.map_err(|e| StorageError {
		kind: StorageErrorKind::DatabaseExtension,
		source: Arc::new(anyhow::Error::from(e)),
	})?;

	postgresql.start().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	let exists = postgresql.database_exists(DB_NAME).await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	if !exists {
		postgresql.create_database(DB_NAME).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	}
	if exists {
		let db_url = postgresql.settings().url(DB_NAME);
		return Ok((postgresql, db_url));
	}
	// restart the postgresql server
	postgresql.stop().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	postgresql.start().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	let database_url = postgresql.settings().url(DB_NAME);
	// prepare async pool
	let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.clone());
	let pool = diesel_async::pooled_connection::deadpool::Pool::builder(manager)
		.max_size(10)
		.wait_timeout(POOL_TIMEOUT)
		.create_timeout(POOL_TIMEOUT)
		.recycle_timeout(POOL_TIMEOUT)
		.runtime(deadpool::Runtime::Tokio1)
		.build()
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	// pg vector says to configue shared_preload_libraries = 'vectors.so' in postgresql.conf
	// and restart the server
	let conn = &mut pool.get().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	diesel::sql_query("ALTER SYSTEM SET shared_preload_libraries = 'vectors.so'")
		.execute(conn)
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	diesel::sql_query("ALTER SYSTEM SET search_path = '$user', public, vectors")
		.execute(conn)
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
	// close the pool
	pool.close();

	// restart the postgresql server
	postgresql.stop().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	postgresql.start().await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	// check if the database is created
	postgresql.database_exists(DB_NAME).await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	// check if postgresql is running
	while postgresql.status() != Status::Started {
		eprintln!("current status: {:?}", postgresql.status());
		tokio::time::sleep(Duration::from_secs(1)).await;
	}
	let database_url = postgresql.settings().url(DB_NAME);
	Ok((postgresql, database_url))
}

pub async fn create_secret_store(
	path: std::path::PathBuf,
) -> anyhow::Result<Arc<dyn SecretStorage>> {
	let secret_store = SecretStore::new(path);
	let secret_store = Arc::new(secret_store);
	Ok(secret_store)
}

pub async fn create_metadata_store(
	path: std::path::PathBuf,
) -> anyhow::Result<Arc<dyn MetaStorage>> {
	let metastore = MetaStore::new(path);
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

async fn enable_extension(pool: &ActualDbPool) -> Result<(), DieselError> {
	let mut conn = pool.get().await.map_err(|e| QueryBuilderError(e.into()))?;
	conn.batch_execute("CREATE EXTENSION IF NOT EXISTS vectors").await?;
	Ok(())
}
