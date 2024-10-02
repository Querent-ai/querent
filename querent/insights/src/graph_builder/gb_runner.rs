use crate::{
	insert_discovered_knowledge_async, InsightConfig, InsightError, InsightErrorKind, InsightInput,
	InsightOutput, InsightResult, InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventType, SemanticKnowledgePayload};
use fastembed::TextEmbedding;
use futures::{pin_mut, Stream, StreamExt};
use lazy_static::lazy_static;
use proto::{Neo4jConfig, StorageType};
use serde_json::Value;
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	sync::{Arc, RwLock},
};
use storage::{FabricStorage, Neo4jStorage};
use tokio::sync::Mutex;
lazy_static! {
	static ref NEO4J_STORAGE: Mutex<Option<(Arc<Neo4jStorage>, Neo4jConfig)>> = Mutex::new(None);
}

pub struct GraphBuilderRunner {
	pub config: InsightConfig,
	pub embedding_model: Option<TextEmbedding>,
	pub neo4j_instance_url: String,
	pub neo4j_username: String,
	pub neo4j_password: String,
	pub neo4j_database: Option<String>,
}

#[async_trait]
impl InsightRunner for GraphBuilderRunner {
	async fn run(&self, input: InsightInput) -> InsightResult<InsightOutput> {
		let session_id = input.data.get("session_id").and_then(Value::as_str).ok_or_else(|| {
			InsightError::new(
				InsightErrorKind::Internal,
				anyhow::anyhow!("Session ID is missing").into(),
			)
		})?;
		let new_config = Neo4jConfig {
			name: "neo4j".to_string(),
			storage_type: Some(StorageType::Graph),
			url: self.neo4j_instance_url.to_string(),
			username: self.neo4j_username.clone(),
			password: self.neo4j_password.clone(),
			db_name: self.neo4j_database.clone().unwrap_or("".to_string()),
			fetch_size: 100,
			max_connection_pool_size: 5,
		};

		let mut storage_lock = NEO4J_STORAGE.lock().await;
		let neo4j_storage = match &*storage_lock {
			Some((existing_storage, existing_config)) if existing_config == &new_config =>
				Arc::clone(existing_storage),
			_ => {
				let new_storage =
					Arc::new(Neo4jStorage::new(new_config.clone()).await.map_err(|_err| {
						InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Failed to initialize Neo4j Storage").into(),
						)
					})?);
				*storage_lock = Some((Arc::clone(&new_storage), new_config));
				new_storage
			},
		};
		let query = input.data.get("query").and_then(Value::as_str);
		let query = match query {
			Some(q) => q,
			None => {
				tracing::info!("Query is missing. Generating auto-suggestions.");
				"Auto_Suggest"
			},
		};

		let embedding_model = self.embedding_model.as_ref().ok_or_else(|| {
			InsightError::new(
				InsightErrorKind::Internal,
				anyhow::anyhow!("Embedding model is not initialized").into(),
			)
		})?;
		let mut discovery_session_id = String::new();
		let embeddings = embedding_model.embed(vec![query.to_string()], None)?;
		let query_embedding = &embeddings[0];
		let mut unique_sentences: HashSet<String> = HashSet::new();
		for (event_type, storages) in self.config.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storages.iter() {
					if !query.is_empty() {
						let mut fetched_results = Vec::new();
						let mut _total_fetched = 0;

						let search_results = storage
							.similarity_search_l2(
								self.config.discovery_session_id.to_string(),
								query.to_string(),
								self.config.semantic_pipeline_id.to_string(),
								query_embedding,
								100,
								0,
								&vec![],
							)
							.await;

						match search_results {
							Ok(results) => {
								if results.is_empty() {
									return Ok(InsightOutput {
										data: Value::String("No results found".to_string()),
									});
								}
								_total_fetched += results.len() as i64;
								let mut combined_results: HashMap<String, (HashSet<String>, f32)> =
									HashMap::new();
								if let Some(first_result) = results.first() {
									discovery_session_id = first_result.session_id.clone().unwrap();
								}
								for document in &results {
									let tag = format!(
										"{}-{}",
										document.subject.replace('_', " "),
										document.object.replace('_', " "),
									);
									if unique_sentences.insert(document.sentence.clone()) {
										let mut tags_set = HashSet::new();
										tags_set.insert(tag);
										combined_results.insert(
											document.sentence.clone(),
											(tags_set, document.score),
										);

										fetched_results.push(document.clone());
									} else {
										tracing::info!(
											"Duplicate found, skipping sentence: {}",
											document.sentence
										);
									}
								}
							},
							Err(e) => {
								log::error!("Failed to search for similar documents: {}", e);
								break;
							},
						}
						tokio::spawn(insert_discovered_knowledge_async(
							storage.clone(),
							fetched_results,
						));
					}
					match storage
						.get_discovered_data(
							if !discovery_session_id.is_empty() {
								discovery_session_id.clone()
							} else {
								self.config.discovery_session_id.clone()
							},
							self.config.semantic_pipeline_id.clone(),
						)
						.await
					{
						Ok(discovered_data) => {
							let mut neo4j_payload: Vec<(
								String,
								String,
								Option<String>,
								SemanticKnowledgePayload,
							)> = Vec::new();
							for knowledge in discovered_data {
								let semantic_payload = SemanticKnowledgePayload {
									subject: knowledge.subject.clone(),
									object: knowledge.object.clone(),
									predicate: knowledge
										.sentence
										.split_whitespace()
										.take(5)
										.collect::<Vec<&str>>()
										.join(" "),
									sentence: knowledge.sentence.clone(),
									subject_type: "Entity".to_string(),
									object_type: "Entity".to_string(),
									predicate_type: "relationship".to_string(),
									blob: Some("".to_string()),
									image_id: Some("".to_string()),
									event_id: "".to_string(),
									source_id: "".to_string(),
								};

								neo4j_payload.push((
									knowledge.doc_id.clone(),
									knowledge.doc_source.clone(),
									None,
									semantic_payload,
								));
							}

							if !neo4j_payload.is_empty() {
								match neo4j_storage
									.insert_graph(session_id.to_string(), &neo4j_payload)
									.await
								{
									Ok(_) => {
										log::info!(
											"Successfully inserted discovered knowledge into Neo4j"
										);
									},
									Err(err) => {
										log::error!("Failed to insert discovered knowledge into Neo4j: {:?}", err);
									},
								}
							}
						},
						Err(err) => {
							log::error!("Failed to fetch discovered data: {:?}", err);
						},
					}
					return Ok(InsightOutput { data: Value::String("".to_string()) });
				}
			}
		}

		Err(InsightError::new(
			InsightErrorKind::NotFound,
			anyhow::anyhow!("No relevant insights found").into(),
		))
	}

	async fn run_stream<'life0>(
		&'life0 self,
		_input: Pin<Box<dyn Stream<Item = InsightInput> + Send + 'life0>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<InsightOutput>> + Send + 'life0>>> {
		Err(InsightError::new(
			InsightErrorKind::NotFound,
			anyhow::anyhow!("Not Implemented!").into(),
		))
	}
}
