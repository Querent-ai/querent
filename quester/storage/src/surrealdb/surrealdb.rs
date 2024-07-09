

use std::sync::Arc;

use crate::{SemanticKnowledge, Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::semantics::SurrealDbConfig;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
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

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
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
	) -> StorageResult<Vec<DocumentPayload>> {
		Ok(vec![])
	}

	async fn traverse_metadata_table(
		&self,
		_filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
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
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
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
			let created: Vec<Record> = self.db
				.create("semantic_knowledge")
				.content(form).await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			dbg!(created);
		}

		Ok(())
		
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