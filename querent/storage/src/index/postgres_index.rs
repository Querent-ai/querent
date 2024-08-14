use crate::DiscoveredKnowledge;
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};

use proto::{
	discovery::DiscoverySessionRequest,
	insights::InsightAnalystRequest,
	semantics::{PostgresConfig, SemanticPipelineRequest},
};

use std::sync::Arc;

use crate::{ActualDbPool, Storage, StorageError, StorageErrorKind, StorageResult, POOL_TIMEOUT};
use deadpool::Runtime;
use diesel::{table, Insertable, Queryable, Selectable};
use diesel_async::AsyncConnection;
use serde::Serialize;
#[derive(Serialize, Queryable, Insertable, Selectable, Debug, Clone)]
#[diesel(table_name = semantic_knowledge)]
pub struct SemanticKnowledge {
	pub subject: String,
	pub subject_type: String,
	pub object: String,
	pub object_type: String,
	pub sentence: String,
	pub document_id: String,
	pub document_source: String,
	pub collection_id: Option<String>,
	pub image_id: Option<String>,
	pub event_id: String,
	pub source_id: String,
}

// #[derive(Debug, Clone, Copy, FromSqlRow, AsExpression, Serialize)]
// #[diesel(sql_type = BigInt)]
// pub struct EventId(pub u64);

pub struct PostgresStorage {
	pub pool: ActualDbPool,
	pub config: PostgresConfig,
}
#[derive(Debug)]
pub struct QuerySuggestion {
	pub query: String,
	pub frequency: i64,
	pub document_source: String,
	pub sentence: String,
	pub tags: Vec<String>,
	pub top_pairs: Vec<String>,
}

impl PostgresStorage {
	pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
		let tls_enabled = config.url.contains("sslmode=require");
		let manager = if tls_enabled {
			// // diesel-async does not support any TLS connections out of the box, so we need to manually
			// // provide a setup function which handles creating the connection
			// let mut d_config = ManagerConfig::default();
			// d_config.custom_setup = Box::new(establish_connection);
			// AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
			// 	config.url.clone(),
			// 	d_config,
			// )
			// TODO: Support TLS
			log::warn!("TLS is not supported yet for Postgres. Please disable it in the connection string.");
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.url.clone())
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

#[async_trait]
impl Storage for PostgresStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, _api_key: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Insert InsightKnowledge into storage
	async fn insert_insight_knowledge(
		&self,
		_query: Option<String>,
		_session_id: Option<String>,
		_response: Option<String>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		Ok(vec![])
	}

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
		Ok(())
	}

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn similarity_search_l2(
		&self,
		_session_id: String,
		_query: String,
		_collection_id: String,
		_payload: &Vec<f32>,
		_max_results: i32,
		_offset: i64,
		_top_pairs_embeddings: Vec<Vec<f32>>,
	) -> StorageResult<Vec<DocumentPayload>> {
		Ok(vec![])
	}

	async fn traverse_metadata_table(
		&self,
		_filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(String, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
	}

	async fn index_knowledge(
		&self,
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for (document_id, document_source, image_id, item) in payload {
					let form = SemanticKnowledge {
						subject: item.subject.clone(),
						subject_type: item.subject_type.clone(),
						object: item.object.clone(),
						object_type: item.object_type.clone(),
						sentence: item.sentence.clone(),
						document_id: document_id.clone(),
						document_source: document_source.clone(),
						collection_id: Some(collection_id.clone()),
						image_id: image_id.clone(),
						event_id: item.event_id.clone(),
						source_id: item.source_id.clone(),
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

	/// Asynchronously fetches suggestions from semantic table.
	async fn autogenerate_queries(
		&self,
		_max_suggestions: i32,
	) -> StorageResult<Vec<QuerySuggestion>> {
		// Return an empty vector
		Ok(Vec::new())
	}

	/// Store key value pair
	async fn store_secret(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_secret(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	//Delete the key value pair
	async fn delete_secret(&self, _key: &String) -> StorageResult<()> {
		Ok(())
	}

	//Get all collectors key value pairs
	async fn get_all_secrets(&self) -> StorageResult<Vec<(String, String)>> {
		Ok(Vec::new())
	}

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<(String, SemanticPipelineRequest)>> {
		Ok(Vec::new())
	}

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(
		&self,
		_pipeline_id: &String,
		_pipeline: SemanticPipelineRequest,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Get semantic pipeline by id
	async fn get_pipeline(
		&self,
		_pipeline_id: &String,
	) -> StorageResult<Option<SemanticPipelineRequest>> {
		Ok(None)
	}

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, _pipeline_id: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(
		&self,
	) -> StorageResult<Vec<(String, DiscoverySessionRequest)>> {
		Ok(Vec::new())
	}

	/// Set Discovery session ran by this node
	async fn set_discovery_session(
		&self,
		_session_id: &String,
		_session: DiscoverySessionRequest,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Get Discovery session by id
	async fn get_discovery_session(
		&self,
		_session_id: &String,
	) -> StorageResult<Option<DiscoverySessionRequest>> {
		Ok(None)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(
		&self,
	) -> StorageResult<Vec<(String, InsightAnalystRequest)>> {
		Ok(Vec::new())
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(
		&self,
		_session_id: &String,
		_session: InsightAnalystRequest,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Get Insight session by id
	async fn get_insight_session(
		&self,
		_session_id: &String,
	) -> StorageResult<Option<InsightAnalystRequest>> {
		Ok(None)
	}

	/// Retrieve Filetered Results when query is empty and semantic pair filters are provided
	async fn filter_and_query(
		&self,
		_session_id: &String,
		_top_pairs: &Vec<String>,
		_max_results: i32,
		_offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		Ok(vec![])
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
		document_source -> Varchar,
		collection_id -> Nullable<Varchar>,
		image_id -> Nullable<VarChar>,
		event_id -> Varchar,
		source_id -> Varchar
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
