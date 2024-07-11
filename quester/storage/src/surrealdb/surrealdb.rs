use std::{fs, sync::Arc};

use crate::{SemanticKnowledge, Storage, StorageError, StorageErrorKind, StorageResult};
use anyhow::Error;
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::{
	semantics::{SemanticPipelineRequest, SurrealDbConfig},
	DiscoverySessionRequest, InsightAnalystRequest,
};
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
struct QueryResultEmbedded {
	embeddings: Option<Vec<f32>>,
	score: f32,
	event_id: String,
	cosine_distance: f64,
}

#[derive(Serialize, Debug, Clone, Deserialize)]

struct QueryResultSemantic {
	document_id: String,
	subject: String,
	object: String,
	document_source: String,
	sentence: String,
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

#[derive(Debug, Deserialize, Clone)]
struct Record {
	#[allow(dead_code)]
	pub id: Thing,
}

impl SurrealDB {
	async fn new(_config: SurrealDbConfig) -> StorageResult<Self> {
		// port = "127.0.0.1:8000"
		let config = Config::default().strict();
		let db_path = "../../../../db".to_string();
		let db = Surreal::new::<RocksDb>((db_path, config)).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		// Use the correct namespace and database
		let _ = db.use_ns(NAMESPACE).use_db(DATABASE).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		// Ensure namespace and database exist
		let create_ns_db_query =
			format!("DEFINE NAMESPACE {}; DEFINE DATABASE {};", NAMESPACE, DATABASE);
		db.query(&create_ns_db_query).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let sql_file_path = "./storage/src/surrealdb/tables-definition.sql";
		let sql = fs::read_to_string(sql_file_path).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(Error::from(e)),
		})?;

		db.query(&sql).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(Error::from(e)),
		})?;

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
		_offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let query_string = format!(
			"SELECT embeddings, score, event_id, vector::similarity::cosine(embeddings, $payload) AS cosine_distance 
			FROM embedded_knowledge 
			WHERE vector::similarity::cosine(embeddings, $payload) <= 0.2 
			ORDER BY cosine_distance 
			LIMIT {}",
			max_results
		);

		let mut response: Response =
			self.db.query(query_string).bind(("payload", payload)).await.map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;

		let query_results =
			response.take::<Vec<QueryResultEmbedded>>(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		let mut results: Vec<DocumentPayload> = Vec::new();

		for query_result in query_results {
			// Step 2: Query the semantic knowledge table
			let query_string_semantic = format!(
				"SELECT document_id, subject, object, document_source, sentence 
				FROM semantic_knowledge 
				WHERE event_id = '{}' ",
				query_result.event_id.clone()
			);

			let mut response_semantic: Response =
				self.db.query(query_string_semantic).await.map_err(|e| StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				})?;

			let query_results_semantic = response_semantic
				.take::<Vec<QueryResultSemantic>>(0)
				.map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;

			for query_result_semantic in query_results_semantic {
				let mut doc_payload = DocumentPayload::default();
				doc_payload.doc_id = query_result_semantic.document_id.clone();
				doc_payload.subject = query_result_semantic.subject.clone();
				doc_payload.object = query_result_semantic.object.clone();
				doc_payload.doc_source = query_result_semantic.document_source.clone();
				doc_payload.cosine_distance = Some(query_result.cosine_distance.clone());
				doc_payload.score = query_result.score.clone();
				doc_payload.sentence = query_result_semantic.sentence.clone();
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
}

#[cfg(test)]
mod tests {

	use crate::{surrealdb::surrealdb::SurrealDB, Storage};
	use common::{SemanticKnowledgePayload, VectorPayload};
	use proto::semantics::SurrealDbConfig;

	#[tokio::test]
	async fn test_surrealdb_integration() -> Result<(), Box<dyn std::error::Error>> {
		// Step 1: Create a new instance of SurrealDB
		let config = SurrealDbConfig {};
		let surreal_db = SurrealDB::new(config).await?;

		// Step 2: Insert data into semantic_knowledge table
		let semantic_payload = vec![(
			"doc1".to_string(),
			"source1".to_string(),
			Some("".to_string()),
			SemanticKnowledgePayload {
				sentence: "sentence1".to_string(),
				subject: "subject1".to_string(),
				object: "object1".to_string(),
				subject_type: "type 1".to_string(),
				object_type: "type 2".to_string(),
				predicate: "abcd".to_string(),
				predicate_type: "type 3".to_string(),
				blob: Some("blob".to_string()),
				image_id: Some("".to_string()),
				event_id: "event1".to_string(),
				source_id: "source_id".to_string(),
			},
		)];
		surreal_db.index_knowledge("collection1".to_string(), &semantic_payload).await?;

		// Step 3: Insert data into embedded_knowledge table
		let vector_payload = vec![(
			"doc1".to_string(),
			"source1".to_string(),
			None,
			VectorPayload {
				embeddings: vec![0.1, 0.2, 0.3],
				score: 1.0,
				event_id: "event1".to_string(),
			},
		)];
		surreal_db.insert_vector("collection1".to_string(), &vector_payload).await?;

		// Step 4: Perform a similarity search
		let query_embeddings = vec![0.1, 0.2, 0.3];
		let results = surreal_db
			.similarity_search_l2(
				"session1".to_string(),
				"query1".to_string(),
				"collection1".to_string(),
				&query_embeddings,
				10,
				0,
			)
			.await?;

		println!("Results: {:?}", results);

		// Assertions
		assert!(!results.is_empty());
		assert_eq!(results[0].doc_id, "doc1");
		assert_eq!(results[0].subject, "subject1");

		Ok(())
	}
}
