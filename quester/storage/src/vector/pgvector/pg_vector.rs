use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{
	result::{ConnectionError, ConnectionResult},
	ExpressionMethods, QueryDsl,
};

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager, ManagerConfig},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};
use futures_util::{future::BoxFuture, FutureExt};
use proto::semantics::PostgresConfig;
use std::{sync::Arc, time::SystemTime};
use tracing::error;

use crate::{ActualDbPool, Storage, StorageError, StorageErrorKind, StorageResult, POOL_TIMEOUT};
use pgvector::Vector;

use deadpool::Runtime;
use diesel::{table, Insertable, Queryable, QueryableByName, Selectable};
use diesel_async::AsyncConnection;
use pgvector::VectorExpressionMethods;
use rustls::{
	client::{ServerCertVerified, ServerCertVerifier},
	ServerName,
};

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = embedded_knowledge)]

pub struct EmbeddedKnowledge {
	pub document_id: String,
	pub document_source: String,
	pub knowledge: String,
	pub embeddings: Option<Vector>,
	pub predicate: String,
	pub sentence: Option<String>,
	pub collection_id: Option<String>,
	pub image_id: Option<String>,
}

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = discovered_knowledge)]
pub struct DiscoveredKnowledge {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub knowledge: String,
	pub subject: String,
	pub object: String,
	pub predicate: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vector>,
	pub session_id: Option<String>,
}

impl DiscoveredKnowledge {
	pub fn from_document_payload(payload: DocumentPayload) -> Self {
		DiscoveredKnowledge {
			doc_id: payload.doc_id,
			doc_source: payload.doc_source,
			sentence: payload.sentence,
			knowledge: payload.knowledge,
			subject: payload.subject,
			object: payload.object,
			predicate: payload.predicate,
			cosine_distance: payload.cosine_distance,
			query_embedding: Some(Vector::from(payload.query_embedding.unwrap_or_default())),
			session_id: payload.session_id,
		}
	}
}

pub struct PGVector {
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

impl PGVector {
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

		Ok(PGVector { pool, config })
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
impl Storage for PGVector {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for (document_id, source, image_id, item) in payload {
					let form = EmbeddedKnowledge {
						document_id: document_id.clone(),
						document_source: source.clone(),
						embeddings: Some(Vector::from(item.embeddings.clone())),
						predicate: item.namespace.clone(),
						knowledge: item.id.clone(),
						sentence: item.sentence.clone(),
						collection_id: Some(_collection_id.clone()),
						image_id: image_id.clone(),
					};
					diesel::insert_into(embedded_knowledge::dsl::embedded_knowledge)
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

	async fn insert_discovered_knowledge(
		&self,
		payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for item in payload {
					let form = DiscoveredKnowledge::from_document_payload(item.clone());
					diesel::insert_into(discovered_knowledge::dsl::discovered_knowledge)
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
	async fn similarity_search_l2(
		&self,
		session_id: String,
		_collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let vector = Vector::from(payload.clone());
		let query_result = embedded_knowledge::dsl::embedded_knowledge
			.select((
				embedded_knowledge::dsl::document_id,
				embedded_knowledge::dsl::document_source,
				embedded_knowledge::dsl::knowledge,
				embedded_knowledge::dsl::embeddings,
				embedded_knowledge::dsl::predicate,
				embedded_knowledge::dsl::sentence,
				embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()),
			))
			.filter(embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()).le(0.6))
			.order_by(embedded_knowledge::dsl::embeddings.cosine_distance(vector))
			.limit(max_results as i64)
			.offset(offset)
			.load::<(String, String, String, Option<Vector>, String, Option<String>, Option<f64>)>(
				&mut *conn,
			)
			.await;

		match query_result {
			Ok(result) => {
				let mut results = Vec::new();
				for (
					doc_id,
					doc_source,
					knowledge,
					_embeddings,
					predicate,
					sentence,
					cosine_distance,
				) in result
				{
					let mut doc_payload = DocumentPayload::default();
					doc_payload.doc_id = doc_id;
					doc_payload.knowledge = knowledge;
					let knowledge_parts: Vec<&str> = doc_payload.knowledge.split('-').collect();
					if knowledge_parts.len() == 3 {
						doc_payload.subject = knowledge_parts[0].to_string();
						doc_payload.predicate = knowledge_parts[1].to_string();
						doc_payload.object = knowledge_parts[2].to_string();
					} else {
						log::debug!(
							"Knowledge triple not in correct format: {:?}",
							doc_payload.knowledge
						);
					};
					doc_payload.doc_source = doc_source;
					doc_payload.cosine_distance = cosine_distance;
					doc_payload.predicate = predicate;
					if let Some(sentence_value) = sentence {
						doc_payload.sentence = sentence_value;
					}
					doc_payload.session_id = Some(session_id.clone());
					doc_payload.query_embedding = Some(payload.clone());
					results.push(doc_payload);
				}
				Ok(results)
			},
			Err(err) => {
				log::error!("Query failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Store key value pair
	async fn store_kv(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_kv(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	embedded_knowledge (id) {
		id -> Int4,
		document_id -> Varchar,
		document_source -> Varchar,
		knowledge -> Text,
		embeddings -> Nullable<Vector>,
		predicate -> Text,
		sentence -> Nullable<Text>,
		collection_id -> Nullable<Text>,
		image_id -> Nullable<Text>,
	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	discovered_knowledge (id) {
		id -> Int4,
		doc_id -> Varchar,
		doc_source -> Varchar,
		sentence -> Text,
		knowledge -> Text,
		subject -> Text,
		object -> Text,
		predicate -> Text,
		cosine_distance -> Nullable<Float8>,
		query -> Nullable<Text>,
		query_embedding -> Nullable<Vector>,
		session_id -> Nullable<Text>,
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use proto::semantics::StorageType;
	const TEST_DB_URL: &str = "postgres://querent:querent@localhost/querent_test?sslmode=prefer";

	#[tokio::test]
	async fn test_postgres_storage() {
		// Create a postgres config
		let config = PostgresConfig {
			url: TEST_DB_URL.to_string(),
			name: "test".to_string(),
			storage_type: Some(StorageType::Index),
		};

		// Create a PostgresStorage instance with the test database URL
		let storage_result = PGVector::new(config).await;

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
