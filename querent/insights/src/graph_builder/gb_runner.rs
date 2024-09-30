use crate::{InsightConfig, InsightError,
	InsightErrorKind, InsightInput, InsightOutput, InsightResult, InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use common::EventType;
use fastembed::TextEmbedding;
use futures::{pin_mut, Stream, StreamExt};
use serde_json::Value;
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	sync::{Arc, RwLock},
};
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

		let embeddings = embedding_model.embed(vec![query.to_string()], None)?;
		let query_embedding = &embeddings[0];
		let mut documents = Vec::new();
		let mut unique_sentences: HashSet<String> = HashSet::new();

		for (event_type, storages) in self.config.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storages.iter() {
					if query.is_empty() {
						// TODO : send all data from the fabric to Neo4j and skip adding in insights table
					}

					let mut fetched_results = Vec::new();
					let mut _total_fetched = 0;
					while documents.len() < 10 {
						let documents_len = documents.len();

						let search_results = storage
							.similarity_search_l2(
								self.config.discovery_session_id.to_string(),
								query.to_string(),
								self.config.semantic_pipeline_id.to_string(),
								query_embedding,
								10,
								0,
								&vec![],
							)
							.await;

						match search_results {
							Ok(results) => {
								if results.is_empty() {
									break;
								}
								_total_fetched += results.len() as i64;
								fetched_results.extend(results.clone());
								let mut combined_results: HashMap<String, (HashSet<String>, f32)> =
									HashMap::new();
								let mut ordered_sentences: Vec<String> = Vec::new();

								for document in &results {
									let tag = format!(
										"{}-{}",
										document.subject.replace('_', " "),
										document.object.replace('_', " "),
									);
									if let Some((existing_tags, total_strength)) =
										combined_results.get_mut(&document.sentence)
									{
										existing_tags.insert(tag);
										*total_strength += document.score;
									} else {
										let mut tags_set = HashSet::new();
										tags_set.insert(tag);
										combined_results.insert(
											document.sentence.clone(),
											(tags_set, document.score),
										);
										ordered_sentences.push(document.sentence.clone());
									}
								}

								for sentence in ordered_sentences {
									if documents.len() >= 10 {
										let index = match results
											.iter()
											.position(|doc| doc.sentence == sentence)
										{
											Some(idx) => idx,
											None => {
												tracing::error!(
													"Unable to find sentence in results: {}",
													sentence
												);
												continue;
											},
										};
										_total_fetched -= results.len() as i64 - index as i64;
										break;
									}

									if unique_sentences.insert(sentence.clone()) {
										if let Some((tags_set, total_strength)) =
											combined_results.get(&sentence)
										{
											let formatted_tags = tags_set
												.clone()
												.into_iter()
												.collect::<Vec<_>>()
												.join(", ");
											if let Some(source) =
												results.iter().find(|doc| doc.sentence == sentence)
											{
												let formatted_document =
													proto::discovery::Insight {
														document: source.doc_id.clone(),
														source: source.doc_source.clone(),
														relationship_strength: total_strength
															.to_string(),
														sentence,
														tags: formatted_tags,
														top_pairs: vec![],
													};

												documents.push(formatted_document);
											} else {
												tracing::error!("Unable to find source document for sentence in Retriever: {}", sentence);
											}
										} else {
											tracing::error!(
												"Unable to process insights in Retriever: {}",
												sentence
											);
										}
									} else {
										break;
									}
								}
							},
							Err(e) => {
								log::error!("Failed to search for similar documents: {}", e);
								break;
							},
						}

						if documents_len == documents.len() {
							break;
						}
					}

					storage
						.insert_insight_knowledge(
							Some(query.to_string()),
							Some(session_id.to_string()),
							Some("abc".to_string()),
						)
						.await
						.map_err(|e| {
							InsightError::new(
								InsightErrorKind::Internal,
								anyhow::anyhow!("Failed to insert insight knowledge: {:?}", e)
									.into(),
							)
						})?;
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
