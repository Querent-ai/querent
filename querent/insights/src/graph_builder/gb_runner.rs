// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use crate::{
	insert_discovered_knowledge_async, InsightConfig, InsightError, InsightErrorKind, InsightInput,
	InsightOutput, InsightResult, InsightRunner,
};
use async_trait::async_trait;
use common::{EventType, SemanticKnowledgePayload};
use fastembed::TextEmbedding;
use futures::Stream;
use lazy_static::lazy_static;
use proto::{Neo4jConfig, StorageType};
use serde_json::Value;
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	sync::Arc,
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
	pub neo4j_database: String,
	pub neo4j_storage: Option<Arc<Neo4jStorage>>,
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
			username: self.neo4j_username.to_string(),
			password: self.neo4j_password.to_string(),
			db_name: self.neo4j_database.to_string(),
			fetch_size: 100,
			max_connection_pool_size: 5,
		};
		let neo4j_storage = match &self.neo4j_storage {
			Some(storage) => Arc::clone(storage),
			None => {
				let new_storage =
					Arc::new(Neo4jStorage::new(new_config.clone()).await.map_err(|_err| {
						InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Failed to initialize Neo4j Storage").into(),
						)
					})?);

				new_storage.check_connectivity().await.map_err(|_err| {
					InsightError::new(
						InsightErrorKind::Internal,
						anyhow::anyhow!("Failed to connect to Neo4j after initialization").into(),
					)
				})?;
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
		let mut insights_text = "".to_string();
		let embedding_model = self.embedding_model.as_ref().ok_or_else(|| {
			InsightError::new(
				InsightErrorKind::Internal,
				anyhow::anyhow!("Embedding model is not initialized").into(),
			)
		})?;
		let mut discovery_session_id = String::new();
		let mut unique_sentences: HashSet<String> = HashSet::new();
		let mut neo4j_payload: Vec<(String, String, Option<String>, SemanticKnowledgePayload)> =
			Vec::new();
		for (event_type, storages) in self.config.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storages.iter() {
					if !query.is_empty() || !self.config.discovery_session_id.is_empty() {
						if !query.is_empty() {
							let embeddings =
								embedding_model.embed(vec![query.to_string()], None)?;
							let query_embedding = &embeddings[0];
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
									let mut combined_results: HashMap<
										String,
										(HashSet<String>, f32),
									> = HashMap::new();
									if let Some(first_result) = results.first() {
										discovery_session_id = first_result
											.session_id
											.clone()
											.unwrap_or("".to_string());
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
												document.sentence.to_string(),
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
									self.config.discovery_session_id.to_string()
								},
								self.config.semantic_pipeline_id.to_string(),
							)
							.await
						{
							Ok(discovered_data) =>
								for knowledge in discovered_data {
									let semantic_payload = SemanticKnowledgePayload {
										subject: knowledge.subject,
										object: knowledge.object,
										predicate: knowledge
											.sentence
											.split_whitespace()
											.take(5)
											.collect::<Vec<&str>>()
											.join(" "),
										sentence: knowledge.sentence,
										subject_type: "Entity".to_string(),
										object_type: "Entity".to_string(),
										predicate_type: "fabric connection".to_string(),
										blob: Some("".to_string()),
										image_id: Some("".to_string()),
										event_id: "".to_string(),
										source_id: "".to_string(),
									};

									neo4j_payload.push((
										knowledge.doc_id,
										knowledge.doc_source,
										Some("".to_string()),
										semantic_payload,
									));
								},
							Err(err) => {
								log::error!("Failed to fetch discovered data: {:?}", err);
							},
						}
					} else if !self.config.semantic_pipeline_id.to_string().is_empty() {
						match storage
							.get_semanticknowledge_data(&self.config.semantic_pipeline_id)
							.await
						{
							Ok(discovered_data) =>
								for knowledge in discovered_data {
									let semantic_payload = SemanticKnowledgePayload {
										subject: knowledge.subject,
										object: knowledge.object,
										predicate: knowledge
											.sentence
											.split_whitespace()
											.take(5)
											.collect::<Vec<&str>>()
											.join(" "),
										sentence: knowledge.sentence,
										subject_type: knowledge.subject_type,
										object_type: knowledge.object_type,
										predicate_type: "relationship".to_string(),
										blob: Some("".to_string()),
										image_id: Some(
											knowledge
												.image_id
												.clone()
												.unwrap_or_else(|| "".to_string()),
										),
										event_id: knowledge.event_id,
										source_id: knowledge.source_id.to_string(),
									};

									neo4j_payload.push((
										knowledge.document_id.clone(),
										knowledge.document_source.to_string(),
										knowledge.image_id.clone(),
										semantic_payload,
									));
								},
							Err(err) => {
								log::error!("Failed to fetch discovered data: {:?}", err);
							},
						}
					}
					if !neo4j_payload.is_empty() {
						match neo4j_storage
							.insert_graph(session_id.to_string(), &neo4j_payload)
							.await
						{
							Ok(_) => {
								log::info!("Successfully inserted discovered knowledge into Neo4j");
								let mut unique_nodes: HashSet<String> = HashSet::new();
								let mut unique_relationships: HashSet<(
									String,
									String,
									String,
									String,
									String,
									String,
									Option<String>,
									String,
								)> = HashSet::new();
								let mut node_connections: HashMap<String, usize> = HashMap::new();
								let mut connected_nodes: HashSet<(String, String)> = HashSet::new();
								for (doc_id, doc_source, _image_id, semantic_payload) in
									&neo4j_payload
								{
									let relationship = (
										semantic_payload.subject.clone(),
										semantic_payload.object.clone(),
										semantic_payload.predicate.clone(),
										semantic_payload.sentence.clone(),
										doc_id.clone(),
										doc_source.clone(),
										_image_id.clone(),
										semantic_payload.predicate_type.clone(),
									);
									if unique_relationships.insert(relationship.clone()) {
										*node_connections
											.entry(semantic_payload.subject.clone())
											.or_insert(0) += 1;
										*node_connections
											.entry(semantic_payload.object.clone())
											.or_insert(0) += 1;

										unique_nodes.insert(semantic_payload.subject.clone());
										unique_nodes.insert(semantic_payload.object.clone());

										connected_nodes.insert((
											semantic_payload.subject.clone(),
											semantic_payload.object.clone(),
										));
									}
								}
								let total_nodes = unique_nodes.len();
								let relationship_count = unique_relationships.len();
								let total_degrees: usize = node_connections.values().sum();
								let average_node_degree = if total_nodes > 0 {
									total_degrees as f64 / total_nodes as f64
								} else {
									0.0
								};
								let mut top_nodes: Vec<_> = node_connections.iter().collect();
								top_nodes.sort_by(|a, b| b.1.cmp(a.1));

								let top_3_nodes: Vec<_> = top_nodes.iter().take(3).collect();
								let total_possible_relationships =
									total_nodes * (total_nodes - 1) / 2;
								let graph_density = if total_possible_relationships > 0 {
									relationship_count as f64 / total_possible_relationships as f64
								} else {
									0.0
								};
								let mut communities: HashMap<String, HashSet<String>> =
									HashMap::new();
								for (subject, object) in connected_nodes {
									communities
										.entry(subject.clone())
										.or_insert(HashSet::new())
										.insert(object.clone());
									communities
										.entry(object.clone())
										.or_insert(HashSet::new())
										.insert(subject.clone());
								}

								let total_communities = communities.len();
								let largest_community_size = communities
									.values()
									.map(|community| community.len())
									.max()
									.unwrap_or(0);
								insights_text = format!(
									"Graph Builder Summary:\n\
									- Total Nodes: {}\n\
									- Average Node Degree: {:.2}\n\
									- Graph Density: {:.4}\n\
									- Number of Communities: {}\n\
									- Largest Community Size: {}\n\
									- Top 3 Central Nodes: {:?}\n",
									total_nodes,
									average_node_degree,
									graph_density,
									total_communities,
									largest_community_size,
									top_3_nodes
								);
							},
							Err(err) => {
								log::error!(
									"Failed to insert discovered knowledge into Neo4j: {:?}",
									err
								);
							},
						}
					}
				}
				return Ok(InsightOutput { data: Value::String(insights_text) });
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
