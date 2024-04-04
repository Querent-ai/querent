use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{
	result::{ConnectionError, ConnectionResult, Error as DieselError, Error::QueryBuilderError},
	table, Insertable, Queryable, Selectable,
};

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{
		deadpool::{Object as PooledConnection, Pool},
		AsyncDieselConnectionManager, ManagerConfig,
	},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};
use futures_util::{future::BoxFuture, FutureExt};
use proto::semantics::PostgresConfig;
use std::{
	ops::{Deref, DerefMut},
	sync::Arc,
	time::{Duration, SystemTime},
};
use tracing::error;

use crate::{Storage, StorageError, StorageErrorKind, StorageResult};
use deadpool::Runtime;
use diesel_async::AsyncConnection;
use rustls::{
	client::{ServerCertVerified, ServerCertVerifier},
	ServerName,
};
use serde::Serialize;

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
const POOL_TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));

#[derive(Serialize, Queryable, Insertable, Selectable, Debug, Clone)]
#[diesel(table_name = semantic_knowledge)]
pub struct SemanticKnowledge {
	pub subject: String,
	pub subject_type: String,
	pub object: String,
	pub object_type: String,
	pub predicate: String,
	pub predicate_type: String,
	pub sentence: String,
	pub document_id: String,
}

pub struct PostgresStorage {
	pub pool: ActualDbPool,
	pub config: PostgresConfig,
}

struct NoCertVerifier {}

impl ServerCertVerifier for NoCertVerifier {
	fn verify_server_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_server_name: &ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: SystemTime,
	) -> Result<ServerCertVerified, rustls::Error> {
		// Will verify all (even invalid) certs without any checks (sslmode=require)
		Ok(ServerCertVerified::assertion())
	}
}

impl PostgresStorage {
	pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
		let mut d_config = ManagerConfig::default();
		let tls_enabled = config.url.contains("sslmode=require");
		let manager = if tls_enabled {
			// diesel-async does not support any TLS connections out of the box, so we need to manually
			// provide a setup function which handles creating the connection
			d_config.custom_setup = Box::new(establish_connection);
			AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
				config.url.clone(),
				d_config,
			)
		} else {
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.url.clone())
		};
		let pool = Pool::builder(manager)
			.max_size(10)
			.wait_timeout(POOL_TIMEOUT)
			.create_timeout(POOL_TIMEOUT)
			.recycle_timeout(POOL_TIMEOUT)
			.runtime(Runtime::Tokio1)
			.build()
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		Ok(PostgresStorage { pool, config })
	}
}

fn establish_connection(config: &str) -> BoxFuture<ConnectionResult<AsyncPgConnection>> {
	let fut = async {
		let rustls_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(Arc::new(NoCertVerifier {}))
			.with_no_client_auth();

		let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
		let (client, conn) = tokio_postgres::connect(config, tls)
			.await
			.map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
		tokio::spawn(async move {
			if let Err(e) = conn.await {
				error!("Database connection failed: {e}");
			}
		});
		AsyncPgConnection::try_from(client).await
	};
	fut.boxed()
}

#[async_trait]
impl Storage for PostgresStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, VectorPayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn insert_graph(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn similarity_search_l2(
		&self,
		_collection_id: String,
		_payload: &Vec<f32>,
		_max_results: i32,
	) -> StorageResult<Vec<DocumentPayload>> {
		Ok(vec![])
	}

	async fn index_knowledge(
		&self,
		payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for (document_id, item) in payload {
					let form = SemanticKnowledge {
						subject: item.subject.clone(),
						subject_type: item.subject_type.clone(),
						object: item.object.clone(),
						object_type: item.object_type.clone(),
						predicate: item.predicate.clone(),
						predicate_type: item.predicate_type.clone(),
						sentence: item.sentence.clone(),
						document_id: document_id.clone(),
					};
					diesel::insert_into(semantic_knowledge::dsl::semantic_knowledge)
						.values(form)
						.execute(conn)
						.await?;
				}
				Ok(())
			}
			.scope_boxed()
		})
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		Ok(())
	}
}

table! {
	semantic_knowledge (id) {
		id -> Int4,
		subject -> Varchar,
		subject_type -> Varchar,
		object -> Varchar,
		object_type -> Varchar,
		predicate -> Varchar,
		predicate_type -> Varchar,
		sentence -> Text,
		document_id -> Varchar,
	}
}

#[cfg(test)]
mod test {
	use proto::semantics::StorageType;

	use super::*;
	const TEST_DB_URL: &str = "postgres://querent:querent@localhost/querent_test?sslmode=prefer";

	// Test function
	#[tokio::test]
	async fn test_postgres_storage() {
		// Create a postgres config
		let config = PostgresConfig {
			url: TEST_DB_URL.to_string(),
			name: "test".to_string(),
			storage_type: Some(StorageType::Index),
		};

		// Create a PostgresStorage instance with the test database URL
		let storage_result = PostgresStorage::new(config).await;

		// Ensure that the storage is created successfully
		assert!(storage_result.is_ok());

		// Get the storage instance from the result
		let storage = storage_result.unwrap();

		// Perform a connectivity check
		let _connectivity_result = storage.check_connectivity().await;
		// Ensure that the connectivity check is successful
		// Works when there is a database running on the test database URL
		//assert!(_connectivity_result.is_ok());

		// You can add more test cases or assertions based on your specific requirements
	}
}
