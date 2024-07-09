

use std::sync::Arc;

use crate::{Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload};
use proto::semantics::SurrealDbConfig;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::opt::Config;
use surrealdb::sql::{Thing, Value};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, RocksDb};
const NAMESPACE: &str = "querent";
const DATABASE: &str = "querent";


pub struct SurrealDB {
    pub db: Surreal<Db>,
}

impl SurrealDB {
    async fn new(config: SurrealDbConfig) -> StorageResult<Self> {
        // port = "127.0.0.1:8000"
        let config = Config::default().strict();
        let db_path = "../../../db".to_string();
        let db = Surreal::new::<RocksDb>((db_path, config)).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

        // Select a specific namespace / database
        db.use_ns(NAMESPACE).use_db(DATABASE).await;

        Ok(SurrealDB{db})
    }
}

#[async_trait]
impl Storage for SurrealDB {
    async fn check_connectivity(&self) -> anyhow::Result<()> {
        let _query_response = self.db.query(
            "SELECT * FROM non_existing_table LIMIT 1;",
        ).await;

		Ok(())
	}
    


    async fn insert_graph(
		&self,
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Semantic Knowledge

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
	async fn get_all_pipelines(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(&self, _pipeline: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get semantic pipeline by id
	async fn get_pipeline(&self, _pipeline_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, _pipeline_id: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Discovery session ran by this node
	async fn set_discovery_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Discovery session by id
	async fn get_discovery_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Insight session by id
	async fn get_insight_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}
}