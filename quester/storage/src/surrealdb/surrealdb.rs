use std::sync::Arc;

use crate::{SemanticKnowledge, Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::semantics::SurrealDbConfig;
use serde::{Deserialize, Serialize};
use surrealdb::{
	engine::local::{Db, RocksDb},
	opt::Config,
	sql::Thing,
	Response, Surreal,
};
const NAMESPACE: &str = "querent";
const DATABASE: &str = "querent";

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct EmbeddedKnowledgeSurrealDb {
	pub embeddings: Vec<f32>,
	pub score: f32,
	pub event_id: String,
}

#[derive(Serialize, Debug, Clone, Deserialize)]

pub struct DiscoveredKnowledgeSurrealDb {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub subject: String,
	pub object: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vec<f32>>,
	pub query: Option<String>,
	pub session_id: Option<String>,
	pub score: Option<f64>,
}

impl DiscoveredKnowledgeSurrealDb {
	pub fn from_document_payload(payload: DocumentPayload) -> Self {
		Self {
			doc_id: payload.doc_id,
			doc_source: payload.doc_source,
			sentence: payload.sentence,
			subject: payload.subject,
			object: payload.object,
			cosine_distance: payload.cosine_distance,
			query_embedding: payload.query_embedding.map(|v| v.into_iter().collect()),
			query: payload.query,
			session_id: payload.session_id,
			score: Some(payload.score as f64),
		}
	}
}

pub struct SurrealDB {
	pub db: Surreal<Db>,
}

#[derive(Debug, Deserialize)]
struct Record {
	#[allow(dead_code)]
	id: Thing,
}

impl SurrealDB {
	async fn new(_config: SurrealDbConfig) -> StorageResult<Self> {
		// port = "127.0.0.1:8000"
		let config = Config::default().strict();
		let db_path = "../../../db".to_string();
		let db = Surreal::new::<RocksDb>((db_path, config)).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		// Select a specific namespace / database
		let _ = db.use_ns(NAMESPACE).use_db(DATABASE).await;

		Ok(SurrealDB { db })
	}
}

#[async_trait]
impl Storage for SurrealDB {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _query_response = self.db.query("SELECT * FROM non_existing_table LIMIT 1;").await;

		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		for (_document_id, _source, _image_id, item) in payload {
			let form = EmbeddedKnowledgeSurrealDb {
				embeddings: item.embeddings.clone(), // Assuming embeddings is a Vec<f32>
				score: item.score,
				event_id: item.event_id.clone(),
			};

			// Insert the form into the 'embedded_knowledge' table in SurrealDB
			let created: Vec<Record> =
				self.db.create("embedded_knowledge").content(form).await.map_err(|e| {
					StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(e)),
					}
				})?;
			dbg!(created);
		}
		Ok(())
	}

	async fn insert_discovered_knowledge(
		&self,
		payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		for item in payload {
			let form = DiscoveredKnowledgeSurrealDb::from_document_payload(item.clone());

			let created: Vec<Record> =
				self.db.create("discovered_knowledge").content(form).await.map_err(|e| {
					StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(e)),
					}
				})?;
			dbg!(created);
		}

		Ok(())
	}

	async fn similarity_search_l2(
		&self,
		session_id: String,
		query: String,
		_collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let query_string = format!(
			"SELECT embeddings, score, event_id, vector::similarity::cosine(embeddings, $payload) AS distance 
			FROM embedded_knowledge 
			WHERE vector::similarity::cosine(embeddings, $payload) <= 0.2 
			ORDER BY distance 
			LIMIT {} OFFSET {}",
			max_results, offset
		);

		let mut response: Response =
			self.db.query(query_string).bind(("payload", payload)).await.map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;

		let query_result =
			response.take::<Vec<(Option<Vec<f32>>, f32, String, f64)>>(0).map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;

		let mut results: Vec<DocumentPayload> = Vec::new();

		for (_embeddings, score, event_id, distance) in query_result {
			// Step 2: Query the semantic knowledge table
			let query_string_semantic = format!(
				"SELECT document_id, subject, object, document_source, sentence 
				FROM semantic_knowledge 
				WHERE event_id = '{}' 
				OFFSET {}",
				event_id, offset
			);

			let mut response_semantic: Response =
				self.db.query(query_string_semantic).await.map_err(|e| StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				})?;

			let query_result_semantic = response_semantic
				.take::<Vec<(String, String, String, String, String)>>(0)
				.map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;

			for (doc_id, subject, object, document_store, sentence) in query_result_semantic {
				let mut doc_payload = DocumentPayload::default();
				doc_payload.doc_id = doc_id.clone();
				doc_payload.subject = subject.clone();
				doc_payload.object = object.clone();
				doc_payload.doc_source = document_store.clone();
				doc_payload.cosine_distance = Some(distance);
				doc_payload.score = score;
				doc_payload.sentence = sentence.clone();
				doc_payload.session_id = Some(session_id.clone());
				doc_payload.query_embedding = Some(payload.clone());
				doc_payload.query = Some(query.clone());
				results.push(doc_payload);
			}
		}

		Ok(results)
	}

	async fn traverse_metadata_table(
		&self,
		_filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
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
			let created: Vec<Record> =
				self.db.create("semantic_knowledge").content(form).await.map_err(|e| {
					StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(e)),
					}
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
