use std::{
	collections::{HashMap, HashSet},
	path::PathBuf,
	sync::Arc,
};

use crate::{
	postgres_index::QuerySuggestion, DiscoveredKnowledge, FabricAccessor, FabricStorage,
	FilteredSemanticKnowledge, SemanticKnowledge, Storage, StorageError, StorageErrorKind,
	StorageResult,
};
use anyhow::Error;
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use pgvector::Vector;
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
	pub cosine_distance: Option<f64>,
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
	pub subject_type: String,
	pub object: String,
	pub object_type: String,
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
	pub collection_id: Option<String>,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct FilteredResultsSurreal {
	pub document_id: String,
	pub subject: String,
	pub object: String,
	pub document_source: String,
	pub sentence: String,
	pub score: f32,
	pub embeddings: Option<Vec<f32>>,
	pub event_id: String,
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
			collection_id: Some(payload.collection_id),
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
impl FabricStorage for SurrealDB {
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
		let session_id = session_id.ok_or_else(|| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::msg("Missing session_id")),
		})?;

		let query = query.ok_or_else(|| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::msg("Missing query")),
		})?;

		let response = response.ok_or_else(|| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::msg("Missing response")),
		})?;

		let form = InsightKnowledgeSurrealDb {
			session_id: session_id.clone(),
			query: query.clone(),
			response: response.clone(),
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
		log::debug!("Created: {:?}", created);

		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		for (_document_id, _source, _image_id, item) in payload {
			let form = EmbeddedKnowledgeSurrealDb {
				embeddings: item.embeddings.clone(),
				score: item.score,
				event_id: item.event_id.clone(),
			};

			let created: Vec<Record> =
				self.db.create("embedded_knowledge").content(form).await.map_err(|e| {
					StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(e)),
					}
				})?;
			log::debug!("Created: {:?}", created);
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
			log::debug!("Created: {:?}", created);
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

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}
}

#[async_trait]
impl FabricAccessor for SurrealDB {
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
			traverse_node(&self.db, head.clone(), &mut combined_results, &mut visited_pairs, 0)
				.await?;
			traverse_node(&self.db, tail.clone(), &mut combined_results, &mut visited_pairs, 0)
				.await?;
		}

		Ok(combined_results)
	}

	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		discovery_session_id: String,
		pipeline_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		let mut query = String::from(
			"SELECT doc_id, doc_source, sentence, subject, object, cosine_distance, session_id, score, query, 
			query_embedding, collection_id
			FROM discovered_knowledge WHERE 1 = 1"
		);
		if !discovery_session_id.is_empty() {
			query.push_str(&format!(" AND session_id = '{}'", discovery_session_id));
		}
		if !pipeline_id.is_empty() {
			query.push_str(&format!(" AND collection_id = '{}'", pipeline_id));
		}
		let mut response: Response = self.db.query(query).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let results = response.take::<Vec<DiscoveredKnowledge>>(0).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		Ok(results
			.into_iter()
			.map(|item| DiscoveredKnowledge {
				doc_id: item.doc_id,
				doc_source: item.doc_source,
				sentence: item.sentence,
				subject: item.subject,
				object: item.object,
				cosine_distance: item.cosine_distance,
				session_id: item.session_id,
				score: item.score,
				query: item.query,
				query_embedding: item.query_embedding.map(Vector::from),
				collection_id: item.collection_id,
			})
			.collect())
	}

	async fn get_semanticknowledge_data(
		&self,
		collection_id: &str,
	) -> StorageResult<Vec<FilteredSemanticKnowledge>> {
		let query = format!(
			"SELECT document_id, subject, subject_type, object, object_type, document_source, sentence, event_id, source_id, image_id FROM semantic_knowledge WHERE collection_id = '{}'",
			collection_id
		);
		let mut response: Response = self.db.query(query).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let results = response.take::<Vec<SemanticKnowledge>>(0).map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		Ok(results
			.into_iter()
			.map(|item| FilteredSemanticKnowledge {
				subject: item.subject,
				subject_type: item.subject_type,
				object: item.object,
				object_type: item.object_type,
				document_source: item.document_source,
				sentence: item.sentence,
				event_id: item.event_id,
				source_id: item.source_id,
				image_id: item.image_id,
				document_id: item.document_id,
			})
			.collect())
	}

	/// Retrieve filtered results when query is empty and semantic pair filters are provided
	async fn filter_and_query(
		&self,
		session_id: &String,
		top_pairs: &Vec<String>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut query = String::from(
			"SELECT document_id, subject, object, document_source, sentence, event_id, subject_type, object_type, source_id
			 FROM semantic_knowledge"
		);

		let formatted_pairs = top_pairs
			.iter()
			.map(|pair| {
				let parts: Vec<&str> = pair.split(" - ").collect();
				format!(
					"(subject = '{}' AND object = '{}') OR (subject = '{}' AND object = '{}')",
					parts[0], parts[1], parts[1], parts[0]
				)
			})
			.collect::<Vec<String>>()
			.join(" OR ");

		if !formatted_pairs.is_empty() {
			query.push_str(&format!(" WHERE ({})", formatted_pairs));
		}

		query.push_str(" ORDER BY event_id ASC");
		query.push_str(&format!(" LIMIT {} START {}", max_results, offset));

		let mut response = self.db.query(query).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let semantic_results: Vec<SemanticKnowledge> =
			response.take(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		let event_ids: Vec<String> = semantic_results
			.iter()
			.filter_map(|record| Some(record.event_id.clone()))
			.collect();

		let embedding_conditions = event_ids
			.iter()
			.map(|id| format!("event_id == '{}'", id))
			.collect::<Vec<String>>()
			.join(" OR ");

		let embedding_query = format!(
			"SELECT event_id, score, embeddings
			 FROM embedded_knowledge
			 WHERE {}",
			embedding_conditions
		);

		let mut embedding_response =
			self.db.query(embedding_query).await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let embedding_results: Vec<QueryResultEmbedded> =
			embedding_response.take(0).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		let embeddings_map: HashMap<String, QueryResultEmbedded> = embedding_results
			.into_iter()
			.map(|result| (result.event_id.clone(), result))
			.collect();
		let document_payloads = semantic_results
			.into_iter()
			.map(|result| {
				let embedding_result = embeddings_map.get(&result.event_id);

				DocumentPayload {
					doc_id: result.document_id,
					doc_source: result.document_source,
					sentence: result.sentence,
					knowledge: format!("{} - {}", result.subject, result.object),
					subject: result.subject,
					object: result.object,
					cosine_distance: None,
					query_embedding: embedding_result
						.map(|e| e.embeddings.clone().unwrap_or_else(Vec::new)),
					query: Some("".to_string()),
					session_id: Some(session_id.clone()),
					score: embedding_result.map(|e| e.score).unwrap_or(0.0),
					collection_id: String::new(),
				}
			})
			.collect();

		Ok(document_payloads)
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
				let combined_query = if documents.len() < 2 {
					format!(
						"Certain documents stand out for their rich diversity of semantic connections, reflecting a broad spectrum of topics. Document '{}' reveals a complex fabric of unique data points.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0)
					)
				} else {
					format!(
						"Certain documents stand out for their rich diversity of semantic connections, reflecting a broad spectrum of topics. For instance, document '{}' reveals a complex fabric of unique data points, followed by '{}'.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0),
						documents[1].0.rsplit('/').next().unwrap_or(&documents[1].0)
					)
				};

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

		if let Some(first_score) = score_result.get(0) {
			let score = first_score.score;
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
				Box::pin(traverse_node(
					db,
					result.subject,
					combined_results,
					visited_pairs,
					depth + 1,
				))
				.await?;
			}
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

		if let Some(first_score) = score_result.get(0) {
			let score = first_score.score;
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
				Box::pin(traverse_node(
					db,
					result.subject,
					combined_results,
					visited_pairs,
					depth + 1,
				))
				.await?;
			}
		}
	}

	Ok(())
}

impl Storage for SurrealDB {}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::tempdir;
	use tokio;
	use uuid::Uuid;

	#[tokio::test]
	async fn test_surrealdb_new() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await;
		assert!(surreal_db.is_ok());
		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_check_connectivity() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let result = surreal_db.check_connectivity().await;
		assert!(result.is_ok());
		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_insert_insight_knowledge() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();

		let session_id = Some("session_1".to_string());
		let query = Some("What is Rust?".to_string());
		let response = Some("Rust is a systems programming language...".to_string());

		let result = surreal_db
			.insert_insight_knowledge(query.clone(), session_id.clone(), response.clone())
			.await;
		assert!(result.is_ok());
		let select_query =
			format!("SELECT * FROM insight_knowledge WHERE session_id = '{}'", session_id.unwrap());
		let mut response = surreal_db.db.query(select_query).await.unwrap();
		let records: Vec<InsightKnowledgeSurrealDb> = response.take(0).unwrap();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].query, query.unwrap());

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_insert_vector() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();

		let collection_id = "collection_1".to_string();
		let payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			VectorPayload {
				embeddings: vec![0.1, 0.2, 0.3],
				score: 0.9,
				event_id: "event_1".to_string(),
			},
		)];

		let result = surreal_db.insert_vector(collection_id.clone(), &payload).await;
		assert!(result.is_ok());
		let select_query = format!(
			"SELECT * FROM embedded_knowledge WHERE event_id = '{}'",
			payload[0].3.event_id
		);
		let mut response = surreal_db.db.query(select_query).await.unwrap();
		let records: Vec<EmbeddedKnowledgeSurrealDb> = response.take(0).unwrap();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].embeddings, payload[0].3.embeddings);
		assert_eq!(records[0].score, payload[0].3.score);
		assert_eq!(records[0].event_id, payload[0].3.event_id);

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_insert_discovered_knowledge() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();

		let payload = vec![DocumentPayload {
			doc_id: "doc_1".to_string(),
			doc_source: "source_1".to_string(),
			sentence: "This is a test sentence.".to_string(),
			knowledge: "This is knowledge".to_string(),
			subject: "subject_1".to_string(),
			object: "object_1".to_string(),
			cosine_distance: Some(0.5),
			query_embedding: Some(vec![0.1, 0.2, 0.3]),
			query: Some("Test query".to_string()),
			session_id: Some("session_1".to_string()),
			score: 0.9,
			collection_id: "collection_1".to_string(),
		}];

		let result = surreal_db.insert_discovered_knowledge(&payload).await;
		assert!(result.is_ok());
		let select_query =
			format!("SELECT * FROM discovered_knowledge WHERE doc_id = '{}'", payload[0].doc_id);
		let mut response = surreal_db.db.query(select_query).await.unwrap();
		let records: Vec<DiscoveredKnowledgeSurrealDb> = response.take(0).unwrap();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].doc_id, payload[0].doc_id);
		assert_eq!(records[0].sentence, payload[0].sentence);

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_index_knowledge() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();

		let collection_id = "collection_1".to_string();
		let payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			SemanticKnowledgePayload {
				subject: "subject_1".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_1".to_string(),
				object_type: "type_2".to_string(),
				sentence: "This is a test sentence.".to_string(),
				event_id: "event_1".to_string(),
				source_id: "source_id_1".to_string(),
				predicate: "predicate_1".to_string(),
				predicate_type: "ptype_1".to_string(),
				image_id: Some("".to_string()),
				blob: Some("event_1".to_string()),
			},
		)];

		let result = surreal_db.index_knowledge(collection_id.clone(), &payload).await;
		assert!(result.is_ok());
		let select_query =
			format!("SELECT * FROM semantic_knowledge WHERE document_id = '{}'", payload[0].0);
		let mut response = surreal_db.db.query(select_query).await.unwrap();
		let records: Vec<SemanticKnowledge> = response.take(0).unwrap();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].subject, payload[0].3.subject);
		assert_eq!(records[0].object, payload[0].3.object);

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_insert_graph() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();

		let collection_id = "collection_1".to_string();
		let payload = vec![];

		let result = surreal_db.insert_graph(collection_id, &payload).await;
		assert!(result.is_ok());

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_traverse_metadata_table() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let semantic_payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			SemanticKnowledgePayload {
				subject: "subject_a".to_string(),
				subject_type: "type_a".to_string(),
				object: "subject_b".to_string(),
				object_type: "type_b".to_string(),
				sentence: "A relates to B.".to_string(),
				event_id: "event_1".to_string(),
				source_id: "source_id_1".to_string(),
				predicate: "predicate_1".to_string(),
				predicate_type: "ptype_1".to_string(),
				image_id: Some("".to_string()),
				blob: Some("event_1".to_string()),
			},
		)];

		surreal_db
			.index_knowledge("collection_1".to_string(), &semantic_payload)
			.await
			.unwrap();

		let vector_payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			VectorPayload {
				embeddings: vec![0.1, 0.2, 0.3],
				score: 0.9,
				event_id: "event_1".to_string(),
			},
		)];

		surreal_db
			.insert_vector("collection_1".to_string(), &vector_payload)
			.await
			.unwrap();

		let filtered_pairs = vec![("subject_a".to_string(), "subject_b".to_string())];

		let result = surreal_db.traverse_metadata_table(&filtered_pairs).await;
		assert!(result.is_ok());
		let traversal_results = result.unwrap();
		assert!(!traversal_results.is_empty());

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_get_discovered_data() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let payload = vec![DocumentPayload {
			doc_id: "doc_1".to_string(),
			doc_source: "source_1".to_string(),
			sentence: "This is a test sentence.".to_string(),
			subject: "subject_1".to_string(),
			object: "object_1".to_string(),
			cosine_distance: Some(0.5),
			query_embedding: Some(vec![0.1, 0.2, 0.3]),
			query: Some("Test query".to_string()),
			session_id: Some("session_1".to_string()),
			score: 0.9,
			collection_id: "collection_1".to_string(),
			knowledge: "This is knowledge".to_string(),
		}];

		surreal_db.insert_discovered_knowledge(&payload).await.unwrap();

		let result = surreal_db
			.get_discovered_data("session_1".to_string(), "collection_1".to_string())
			.await;
		assert!(result.is_ok());
		let data = result.unwrap();
		assert_eq!(data.len(), 1);
		assert_eq!(data[0].doc_id, "doc_1");

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_get_semanticknowledge_data() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let collection_id = "collection_1".to_string();
		let payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			SemanticKnowledgePayload {
				subject: "subject_1".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_1".to_string(),
				object_type: "type_2".to_string(),
				sentence: "This is a test sentence.".to_string(),
				event_id: "event_1".to_string(),
				source_id: "source_id_1".to_string(),
				predicate: "predicate_1".to_string(),
				predicate_type: "ptype_1".to_string(),
				image_id: Some("".to_string()),
				blob: Some("event_1".to_string()),
			},
		)];

		surreal_db.index_knowledge(collection_id.clone(), &payload).await.unwrap();

		let result = surreal_db.get_semanticknowledge_data(&collection_id).await;
		assert!(result.is_ok());
		let data = result.unwrap();
		assert_eq!(data.len(), 1);
		assert_eq!(data[0].subject, "subject_1");

		drop(surreal_db);
		temp_dir.close().unwrap();
	}

	#[tokio::test]
	async fn test_filter_and_query() -> Result<(), Box<dyn std::error::Error>> {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let mut semantic_payload = Vec::new();
		for i in 1..=20 {
			semantic_payload.push((
				format!("doc_{}", i),
				format!("source_{}", i),
				None,
				SemanticKnowledgePayload {
					subject: format!("subject_{}", i),
					subject_type: format!("type_{}", i),
					object: format!("object_{}", i),
					object_type: format!("type_{}", i),
					sentence: format!("This is a test sentence number {}.", i),
					event_id: format!("event_{}", i),
					source_id: format!("source_id_{}", i),
					predicate: format!("predicate_{}", i),
					predicate_type: format!("ptype_{}", i),
					image_id: Some(format!("image_{}", i)),
					blob: Some(format!("blob_{}", i)),
				},
			));
		}
		let mut vector_payload = Vec::new();
		for i in 1..=20 {
			vector_payload.push((
				format!("doc_{}", i),
				format!("source_{}", i),
				None,
				VectorPayload {
					embeddings: vec![0.1 * i as f32, 0.2 * i as f32, 0.3 * i as f32],
					score: 1.0 / i as f32,
					event_id: format!("event_{}", i),
				},
			));
		}
		surreal_db
			.index_knowledge("collection_1".to_string(), &semantic_payload)
			.await
			.unwrap();
		surreal_db
			.insert_vector("collection_1".to_string(), &vector_payload)
			.await
			.unwrap();

		let session_id = "test_session".to_string();
		let top_pairs =
			vec!["subject_1 - object_1".to_string(), "subject_5 - object_5".to_string()];
		let max_results = 10;
		let offset = 0;
		let results = surreal_db
			.filter_and_query(&session_id, &top_pairs, max_results, offset)
			.await
			.unwrap();

		assert_eq!(results.len(), 2, "Expected {} results, got {}", max_results, results.len());

		drop(surreal_db);
		temp_dir.close().unwrap();

		Ok(())
	}

	#[tokio::test]
	async fn test_similarity_search_l2() {
		let temp_dir = tempdir().unwrap();
		let db_path = temp_dir.path().join(format!("test-{}.db", Uuid::new_v4()));
		let surreal_db = SurrealDB::new(db_path.clone()).await.unwrap();
		let collection_id = "collection_1".to_string();
		let session_id = "session_1".to_string();
		let query = "Sample query".to_string();
		let embedding_1 = vec![0.1, 0.2, 0.3];
		let query_embedding = vec![0.1, 0.2, 0.3];

		let payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			SemanticKnowledgePayload {
				subject: "subject_1".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_1".to_string(),
				object_type: "type_2".to_string(),
				sentence: "This is a test sentence.".to_string(),
				event_id: "event_1".to_string(),
				source_id: "source_id_1".to_string(),
				predicate: "predicate_1".to_string(),
				predicate_type: "ptype_1".to_string(),
				image_id: Some("".to_string()),
				blob: Some("event_1".to_string()),
			},
		)];

		let result = surreal_db.index_knowledge(collection_id.clone(), &payload).await;
		assert!(result.is_ok());

		let vector_payload = vec![(
			"doc_1".to_string(),
			"source_1".to_string(),
			None,
			VectorPayload {
				embeddings: embedding_1.clone(),
				score: 0.9,
				event_id: "event_1".to_string(),
			},
		)];
		surreal_db.insert_vector(collection_id.clone(), &vector_payload).await.unwrap();
		let result = surreal_db
			.similarity_search_l2(
				session_id.clone(),
				query.clone(),
				collection_id.clone(),
				&query_embedding,
				10,
				0,
				&vec![],
			)
			.await;

		assert!(result.is_ok());
		let documents = result.unwrap();
		assert!(documents.len() >= 1);
		assert_eq!(documents[0].doc_id, "doc_1");

		drop(surreal_db);
		temp_dir.close().unwrap();
	}
}
