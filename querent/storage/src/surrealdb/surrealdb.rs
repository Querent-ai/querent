use std::{
	collections::{HashMap, HashSet},
	path::PathBuf,
	sync::Arc,
};

use crate::{
	postgres_index::QuerySuggestion, DiscoveredKnowledge, SemanticKnowledge, Storage, StorageError,
	StorageErrorKind, StorageResult, RIAN_API_KEY,
};
use anyhow::Error;
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::{semantics::SemanticPipelineRequest, DiscoverySessionRequest, InsightAnalystRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::{
	engine::local::{Db, RocksDb},
	opt::Config,
	sql::Thing,
	Response, Surreal,
};

use super::utils::fetch_documents_for_embedding;
const NAMESPACE: &str = "querent";
const DATABASE: &str = "querent";

#[derive(Serialize, Debug, Clone, Deserialize)]
struct ScoreResponse {
	score: f32,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct EmbeddedKnowledgeSurrealDb {
	pub embeddings: Vec<f32>,
	pub score: f32,
	pub event_id: String,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct QueryResultEmbedded {
	pub embeddings: Option<Vec<f32>>,
	pub score: f32,
	pub event_id: String,
	pub cosine_distance: f64,
}

#[derive(Deserialize, Debug)]
pub struct AutoSuggestPairs {
	subject: String,
	subject_type: String,
	object: String,
	object_type: String,
	pair_frequency: i64,
}

#[derive(Serialize, Debug, Clone, Deserialize)]

pub struct InsightKnowledgeSurrealDb {
	pub session_id: String,
	pub query: String,
	pub response: String,
}
#[derive(Serialize, Debug, Clone, Deserialize)]

pub struct QueryResultSemantic {
	pub document_id: String,
	pub subject: String,
	pub object: String,
	pub document_source: String,
	pub sentence: String,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
struct QueryResultTraverser {
	id: Thing,
	document_id: String,
	subject: String,
	object: String,
	document_source: String,
	sentence: String,
	event_id: String,
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

#[derive(Debug, Deserialize, Clone, Serialize)]
struct Record {
	#[allow(dead_code)]
	pub id: Thing,
}

impl SurrealDB {
	pub async fn new(path: PathBuf) -> StorageResult<Self> {
		let config = Config::default().strict();
		let db = Surreal::new::<RocksDb>((path, config)).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let _ = db.use_ns(NAMESPACE).use_db(DATABASE).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let create_ns_db_query =
			format!("DEFINE NAMESPACE {}; DEFINE DATABASE {};", NAMESPACE, DATABASE);
		db.query(&create_ns_db_query).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let bytes = include_bytes!("./tables-definition.sql");
		let sql = std::str::from_utf8(bytes).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(Error::from(e)),
		})?;

		db.query(sql).await.map_err(|e| StorageError {
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

	async fn insert_insight_knowledge(
		&self,
		query: Option<String>,
		session_id: Option<String>,
		response: Option<String>,
	) -> StorageResult<()> {
		let form = InsightKnowledgeSurrealDb {
			session_id: session_id.unwrap().clone(),
			query: query.unwrap().clone(),
			response: response.unwrap().clone(),
		};

		let created: Vec<Record> = self
			.db
			.create("insight_knowledge")
			.content(form)
			.await
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		dbg!(created);

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
		collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		offset: i64,
		top_pairs_embeddings: &Vec<Vec<f32>>,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut results: Vec<DocumentPayload> = Vec::new();

		if top_pairs_embeddings.is_empty() || top_pairs_embeddings.len() == 1 {
			let embedding = if top_pairs_embeddings.is_empty() {
				payload.clone()
			} else {
				top_pairs_embeddings[0].clone()
			};
			results.extend(
				fetch_documents_for_embedding(
					&self.db,
					&embedding,
					offset,
					max_results as i64,
					&session_id,
					&query,
					&collection_id,
					&payload,
				)
				.await?,
			);
		} else {
			let num_embeddings = top_pairs_embeddings.len();
			let full_cycles = offset / num_embeddings as i64;
			let remaining = offset % num_embeddings as i64;

			for (i, embedding) in top_pairs_embeddings.iter().enumerate() {
				let adjusted_offset =
					if i < remaining as usize { full_cycles + 1 } else { full_cycles };
				results.extend(
					fetch_documents_for_embedding(
						&self.db,
						embedding,
						adjusted_offset,
						1,
						&session_id,
						&query,
						&collection_id,
						&payload,
					)
					.await?,
				);
			}
		}

		Ok(results)
	}
	// async fn similarity_search_l2(
	// 	&self,
	// 	session_id: String,
	// 	query: String,
	// 	_collection_id: String,
	// 	payload: &Vec<f32>,
	// 	max_results: i32,
	// 	offset: i64,
	// 	_top_pairs_embeddings: Vec<Vec<f32>>,
	// ) -> StorageResult<Vec<DocumentPayload>> {
	// 	let mut results: Vec<DocumentPayload> = Vec::new();
	// 	let query_string = format!(
	// 		"SELECT embeddings, score, event_id, vector::similarity::cosine(embeddings, $payload) AS cosine_distance
	// 		FROM embedded_knowledge
	// 		WHERE vector::similarity::cosine(embeddings, $payload) > 0.5
	// 		ORDER BY cosine_distance DESC
	// 		LIMIT {} START {}",
	// 		max_results, offset
	// 	);
	// 	let mut response: Response =
	// 		self.db.query(query_string).bind(("payload", payload)).await.map_err(|e| {
	// 			StorageError {
	// 				kind: StorageErrorKind::Query,
	// 				source: Arc::new(anyhow::Error::from(e)),
	// 			}
	// 		})?;

	// 	let query_results =
	// 		response.take::<Vec<QueryResultEmbedded>>(0).map_err(|e| StorageError {
	// 			kind: StorageErrorKind::Internal,
	// 			source: Arc::new(anyhow::Error::from(e)),
	// 		})?;

	// 	for query_result in query_results {
	// 		let query_string_semantic = format!(
	// 			"SELECT document_id, subject, object, document_source, sentence
	// 			FROM semantic_knowledge
	// 			WHERE event_id = '{}' ",
	// 			query_result.event_id.clone()
	// 		);

	// 		let mut response_semantic: Response =
	// 			self.db.query(query_string_semantic).await.map_err(|e| StorageError {
	// 				kind: StorageErrorKind::Query,
	// 				source: Arc::new(anyhow::Error::from(e)),
	// 			})?;

	// 		let query_results_semantic = response_semantic
	// 			.take::<Vec<QueryResultSemantic>>(0)
	// 			.map_err(|e| StorageError {
	// 				kind: StorageErrorKind::Internal,
	// 				source: Arc::new(anyhow::Error::from(e)),
	// 			})?;

	// 		for query_result_semantic in query_results_semantic {
	// 			let mut doc_payload = DocumentPayload::default();
	// 			doc_payload.doc_id = query_result_semantic.document_id.clone();
	// 			doc_payload.subject = query_result_semantic.subject.clone();
	// 			doc_payload.object = query_result_semantic.object.clone();
	// 			doc_payload.doc_source = query_result_semantic.document_source.clone();
	// 			doc_payload.cosine_distance = Some(1.0 - query_result.cosine_distance.clone());
	// 			doc_payload.score = query_result.score.clone();
	// 			doc_payload.sentence = query_result_semantic.sentence.clone();
	// 			doc_payload.session_id = Some(session_id.clone());
	// 			doc_payload.query_embedding = Some(payload.clone());
	// 			doc_payload.query = Some(query.clone());
	// 			results.push(doc_payload);
	// 		}
	// 	}

	// 	Ok(results)
	// }

	async fn traverse_metadata_table(
		&self,
		filtered_pairs: &[(String, String)],
	) -> StorageResult<Vec<(String, String, String, String, String, String, String, f32)>> {
		let mut combined_results: Vec<(
			String,
			String,
			String,
			String,
			String,
			String,
			String,
			f32,
		)> = Vec::new();
		let mut visited_pairs: HashSet<(String, String)> = HashSet::new();
		for (head, tail) in filtered_pairs {
			// Traverse depth 1
			traverse_node(&self.db, head.clone(), &mut combined_results, &mut visited_pairs, 0)
				.await?;
			traverse_node(&self.db, tail.clone(), &mut combined_results, &mut visited_pairs, 0)
				.await?;
		}

		Ok(combined_results)
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
			let _created: Vec<Record> =
				self.db.create("semantic_knowledge").content(form).await.map_err(|e| {
					StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(e)),
					}
				})?;
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

	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		Ok(vec![])
	}

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, api_key: &String) -> StorageResult<()> {
		// Inner Key: RIAN_API_KEY
		self.store_secret(&RIAN_API_KEY.to_string(), api_key).await?;
		Ok(())
	}

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>> {
		// Inner Key: RIAN_API_KEY
		self.get_secret(&RIAN_API_KEY.to_string()).await
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

	/// Asynchronously fetches suggestions from the semantic table in SurrealDB.
	async fn autogenerate_queries(
		&self,
		max_suggestions: i32,
	) -> Result<Vec<QuerySuggestion>, StorageError> {
		let top_pairs_query = "
		SELECT subject, subject_type, object, object_type, count() as pair_frequency
		FROM semantic_knowledge
		GROUP BY subject, subject_type, object, object_type
		ORDER BY pair_frequency DESC
		LIMIT 1000";

		let mut top_pairs_result = self.db.query(top_pairs_query).await.map_err(|e| {
			StorageError { kind: StorageErrorKind::Query, source: Arc::new(anyhow::Error::from(e)) }
		})?;

		let top_pairs: Vec<AutoSuggestPairs> =
			top_pairs_result.take::<Vec<AutoSuggestPairs>>(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let mut suggestions = Vec::new();

		if !top_pairs.is_empty() {
			let mut pairs = Vec::new();
			let mut pair_strings = Vec::new();
			let mut seen_pairs = HashSet::new();
			for AutoSuggestPairs { subject, subject_type, object, object_type, pair_frequency } in
				top_pairs.into_iter()
			{
				let pair = if subject < object {
					(subject.clone(), object.clone())
				} else {
					(object.clone(), subject.clone())
				};
				if seen_pairs.insert(pair.clone()) {
					pairs.push((subject, subject_type, object, object_type, pair_frequency));
					pair_strings.push(format!("{} - {}", pair.0, pair.1));
				}
				if pair_strings.len() >= 10 {
					break;
				}
			}
			suggestions.push(QuerySuggestion {
				query: "Filters".to_string(),
				frequency: 1,
				document_source: String::new(),
				sentence: String::new(),
				tags: vec!["Filters".to_string()],
				top_pairs: pair_strings,
			});
		}

		if max_suggestions > 1 {
			let bottom_pairs_query = "
				SELECT subject, subject_type, object, object_type, COUNT() as pair_frequency
				FROM semantic_knowledge
				GROUP BY subject, subject_type, object, object_type
				ORDER BY pair_frequency ASC
				LIMIT 5";
			let mut bottom_pairs_result =
				self.db.query(bottom_pairs_query).await.map_err(|e| StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				})?;

			let bottom_pairs: Vec<AutoSuggestPairs> =
				bottom_pairs_result.take::<Vec<AutoSuggestPairs>>(0).map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			let most_mix_documents_query = "
				SELECT document_id, COUNT() AS entity_mix
				FROM
				(
					SELECT document_id, subject, object, COUNT() AS pair_count
					FROM semantic_knowledge
					GROUP BY document_id, subject, object
				)
				GROUP BY document_id
				ORDER BY entity_mix DESC
				LIMIT 5";
			let mut most_mix_documents_result =
				self.db.query(most_mix_documents_query).await.map_err(|e| StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			let most_mix_documents: Vec<(String, i64)> = most_mix_documents_result
				.take::<Vec<HashMap<String, Value>>>(0)
				.map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?
				.into_iter()
				.map(|item| {
					let document_id = item
						.get("document_id")
						.and_then(|v| v.as_str())
						.unwrap_or_default()
						.to_string();
					let entity_mix =
						item.get("entity_mix").and_then(|v| v.as_i64()).unwrap_or_default();
					(document_id, entity_mix)
				})
				.collect();

			if !bottom_pairs.is_empty() {
				let mut pairs = Vec::new();
				for AutoSuggestPairs {
					subject,
					subject_type,
					object,
					object_type,
					pair_frequency,
				} in bottom_pairs.into_iter()
				{
					pairs.push((subject, subject_type, object, object_type, pair_frequency));
				}
				let combined_query = format!(
						"Explore rare and potentially significant connections within your semantic data fabric. These connections unveil the underlying patterns and dynamics woven into your data landscape. Noteworthy interactions are found between '{}' and '{}', along with the link between '{}' and '{}'.",
						pairs[0].0, pairs[0].2,
						pairs[1].0, pairs[1].2
					);

				suggestions.push(QuerySuggestion {
					query: combined_query,
					frequency: 1,
					document_source: String::new(),
					sentence: String::new(),
					tags: vec!["Rare Semantic Data Fabric Interactions".to_string()],
					top_pairs: vec!["Rare Semantic Data Fabric Interactions".to_string()],
				});
			}

			if !most_mix_documents.is_empty() {
				let mut documents = Vec::new();
				for (document_id, entity_mix) in most_mix_documents.into_iter() {
					documents.push((document_id, entity_mix));
				}
				let combined_query = format!(
						"Certain documents stand out for their rich diversity of semantic connections, reflecting a broad spectrum of topics. For instance, document '{}' reveals a complex network of unique data points, followed by '{}'.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0),
						documents[1].0.rsplit('/').next().unwrap_or(&documents[1].0)
					);

				suggestions.push(QuerySuggestion {
					query: combined_query,
					frequency: 1,
					document_source: String::new(),
					sentence: String::new(),
					tags: vec!["Diverse Semantic Data Fabric Interactions".to_string()],
					top_pairs: vec!["Diverse Semantic Data Fabric Interactions".to_string()],
				});
			}
		}

		Ok(suggestions.into_iter().take(max_suggestions as usize).collect())
	}
}

pub async fn traverse_node<'a>(
	db: &'a Surreal<Db>,
	node: String,
	combined_results: &'a mut Vec<(String, String, String, String, String, String, String, f32)>,
	visited_pairs: &'a mut HashSet<(String, String)>,
	depth: usize,
) -> StorageResult<()> {
	if depth >= 1 {
		return Ok(());
	}
	// Fetch inward edges
	let inward_query = format!(
			"SELECT id, document_id, subject, object, document_source, sentence, event_id FROM semantic_knowledge WHERE object = '{}'", 
			node
		);
	let mut response: Response = db.query(inward_query).await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	let inward_results = response.take::<Vec<QueryResultTraverser>>(0).map_err(|e| {
		StorageError { kind: StorageErrorKind::Internal, source: Arc::new(anyhow::Error::from(e)) }
	})?;

	for result in inward_results {
		let score_query = format!(
			"SELECT score FROM embedded_knowledge WHERE event_id = '{}'",
			result.event_id.clone()
		);

		let mut score_response: Response =
			db.query(score_query).await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		let score_result =
			score_response.take::<Vec<ScoreResponse>>(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let score = score_result[0].score;
		if visited_pairs.insert((result.subject.clone(), result.object.clone())) {
			combined_results.push((
				result.id.id.to_string(),
				result.document_id,
				result.subject.clone(),
				result.object.clone(),
				result.document_source,
				result.sentence,
				result.event_id,
				score,
			));
			Box::pin(traverse_node(db, result.subject, combined_results, visited_pairs, depth + 1))
				.await?;
		}
	}
	// Fetch outward edges
	let outward_query = format!(
			"SELECT id, document_id, subject, object, document_source, sentence, event_id FROM semantic_knowledge WHERE subject = '{}'", 
			node
		);

	let mut response: Response = db.query(outward_query).await.map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;

	let outward_results = response.take::<Vec<QueryResultTraverser>>(0).map_err(|e| {
		StorageError { kind: StorageErrorKind::Internal, source: Arc::new(anyhow::Error::from(e)) }
	})?;

	for result in outward_results {
		let score_query =
			format!("SELECT score FROM embedded_knowledge WHERE event_id = '{}'", result.event_id);

		let mut score_response: Response =
			db.query(score_query).await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let score_result =
			score_response.take::<Vec<ScoreResponse>>(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let score = score_result[0].score;

		if visited_pairs.insert((result.subject.clone(), result.object.clone())) {
			combined_results.push((
				result.id.id.to_string(),
				result.document_id,
				result.subject.clone(),
				result.object.clone(),
				result.document_source,
				result.sentence,
				result.event_id,
				score,
			));
			Box::pin(traverse_node(db, result.subject, combined_results, visited_pairs, depth + 1))
				.await?;
		}
	}

	Ok(())
}

// #[cfg(test)]
// mod tests {

// 	use std::path::Path;

// 	use crate::{surrealdb::surrealdb::SurrealDB, Storage};
// 	use common::{SemanticKnowledgePayload, VectorPayload};
// 	use csv::Reader;
// 	use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
// 	use serde::Deserialize;

// 	#[derive(Debug, Deserialize)]
// 	struct CsvRecord {
// 		pub document_id: String,
// 		pub document_source: String,
// 		pub image_id: Option<String>,
// 		pub subject: String,
// 		pub subject_type: String,
// 		pub object: String,
// 		pub object_type: String,
// 		pub sentence: String,
// 		pub event_id: String,
// 		pub source_id: String,
// 	}

// 	#[derive(Debug, Deserialize)]
// 	struct VectorRecord {
// 		pub id: i32,
// 		pub embeddings: String,
// 		pub score: f32,
// 		pub event_id: String,
// 	}

// 	#[tokio::test]
// 	async fn test_insert_csv_data_into_surrealdb() -> Result<(), Box<dyn std::error::Error>> {
// 		let path = Path::new("../../../../db").to_path_buf();
// 		let surrealdb = SurrealDB::new(path).await?;

// 		// Read the CSV file
// 		let file_path = "/home/ansh/Downloads/semantic_knowledge (3).csv";
// 		let mut rdr = Reader::from_path(file_path)?;

// 		let mut payload = Vec::new();
// 		for result in rdr.deserialize() {
// 			let record: CsvRecord = result?;
// 			let semantic_payload = SemanticKnowledgePayload {
// 				subject: record.subject,
// 				subject_type: record.subject_type,
// 				object: record.object,
// 				object_type: record.object_type,
// 				sentence: record.sentence,
// 				event_id: record.event_id,
// 				source_id: record.source_id,
// 				predicate: "abbc".to_string(),
// 				predicate_type: "abbc".to_string(),
// 				image_id: None,
// 				blob: None,
// 			};
// 			payload.push((
// 				"doc1".to_string(),
// 				"source1".to_string(),
// 				Some("".to_string()),
// 				semantic_payload,
// 			));
// 		}

// 		// Insert into SurrealDB
// 		surrealdb.index_knowledge("abcd".to_string(), &payload).await?;

// 		Ok(())
// 	}

// 	#[tokio::test]
// 	async fn test_vector_csv_data_into_surrealdb() -> Result<(), Box<dyn std::error::Error>> {
// 		let path = Path::new("../../../../db").to_path_buf();
// 		let surrealdb = SurrealDB::new(path).await?;

// 		// Read the CSV file
// 		let file_path = "/home/ansh/Downloads/embedded_knowledge.csv";
// 		let mut rdr = Reader::from_path(file_path)?;

// 		let mut payload = Vec::new();
// 		for result in rdr.deserialize() {
// 			let record: VectorRecord = result?;

// 			// Parse embeddings
// 			let embeddings: Vec<f32> = record
// 				.embeddings
// 				.trim_matches(|p| p == '[' || p == ']')
// 				.split(',')
// 				.filter_map(|s| s.trim().parse().ok())
// 				.collect();
// 			let vector_payload = VectorPayload {
// 				embeddings,
// 				score: record.score.clone(),
// 				event_id: record.event_id.clone(),
// 			};
// 			payload.push((
// 				"doc1".to_string(),
// 				"source1".to_string(),
// 				Some("".to_string()),
// 				vector_payload,
// 			));
// 		}

// 		// Insert into SurrealDB
// 		surrealdb.insert_vector("abcd".to_string(), &payload).await?;

// 		Ok(())
// 	}

// #[tokio::test]
// async fn test_surrealdb_integration() -> Result<(), Box<dyn std::error::Error>> {
// 	let path = Path::new("../../../../db").to_path_buf();
// 	let surreal_db = SurrealDB::new(path).await?;

// 	let mut current_query_embedding: Vec<f32> = Vec::new();

// 	let embedding_model = TextEmbedding::try_new(InitOptions {
// 		model_name: EmbeddingModel::AllMiniLML6V2,
// 		show_download_progress: true,
// 		..Default::default()
// 	});
// 	let embedder = embedding_model?;

// 	let query = "What is the temperature for high-Mg calcite in biogenic carbonates ?".to_string();
// 	let embeddings = embedder.embed(vec![query.clone()], None)?;
// 	current_query_embedding = embeddings[0].clone();

// 	// Step 4: Perform a similarity search
// 	let results = surreal_db
// 		.similarity_search_l2(
// 			"session1".to_string(),
// 			query,
// 			"collection1".to_string(),
// 			&current_query_embedding,
// 			10,
// 			0,
// 		)
// 		.await?;

// 	println!("Results: {:?}", results);

// 	// Assertions
// 	assert!(!results.is_empty());

// 	Ok(())
// }

// 	#[tokio::test]
// 	async fn test_vector_dimensions() -> Result<(), Box<dyn std::error::Error>> {
// 		let path = Path::new("../../../../db").to_path_buf();
// 		let surreal_db = SurrealDB::new(path).await?;

// 		let embedding_model = TextEmbedding::try_new(InitOptions {
// 			model_name: EmbeddingModel::AllMiniLML6V2,
// 			show_download_progress: true,
// 			..Default::default()
// 		});
// 		let embedder = embedding_model?;

// 		let query = "What is the fluid type in eagle ford shale?".to_string();
// 		let embeddings = embedder.embed(vec![query.clone()], None)?;
// 		let current_query_embedding = embeddings[0].clone();

// 		// Perform a similarity search
// 		let results = surreal_db
// 			.similarity_search_l2(
// 				"session1".to_string(),
// 				"query1".to_string(),
// 				"collection1".to_string(),
// 				&current_query_embedding,
// 				10,
// 				0,
// 			)
// 			.await?;

// 		println!("Results: {:?}", results);

// 		// Assertions
// 		assert!(!results.is_empty());
// 		assert_eq!(results[0].doc_id, "doc1");
// 		assert_eq!(results[0].subject, "subject1");

// 		Ok(())
// 	}
//  }
