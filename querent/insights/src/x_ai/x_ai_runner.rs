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
	cosine_similarity, extract_sentences, prompts::get_suggestions_prompt, InsightConfig,
	InsightError, InsightErrorKind, InsightInput, InsightOutput, InsightResult, InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use common::{DocumentPayload, EventType};
use fastembed::TextEmbedding;
use futures::{pin_mut, Stream, StreamExt};
use llms::{Message, LLM};
use serde_json::Value;
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	sync::{Arc, RwLock},
};
use storage::DiscoveredKnowledge;

use super::prompts::get_final_prompt;

pub struct XAIRunner {
	pub config: InsightConfig,
	pub llm: Arc<dyn LLM>,
	pub embedding_model: Option<TextEmbedding>,
	pub previous_query_results: RwLock<String>,
	pub previous_filtered_results: RwLock<Vec<(String, String)>>,
	pub previous_session_id: RwLock<String>,
	pub prompt: String,
}

#[async_trait]
impl InsightRunner for XAIRunner {
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

		let prompt: &str = if self.prompt.is_empty() {
			tracing::info!("User did not provide a prompt. Going to use default prompt.");
			""
		} else {
			&self.prompt
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
						let suggestions = storage.autogenerate_queries(3).await.map_err(|e| {
							InsightError::new(
								InsightErrorKind::Internal,
								anyhow::anyhow!("Failed to autogenerate queries: {:?}", e).into(),
							)
						})?;
						let suggestion_texts: Vec<&str> =
							suggestions.iter().map(|s| s.query.as_str()).collect();
						let suggestions_prompt = get_suggestions_prompt(&suggestion_texts);
						let human_message = vec![Message::new_human_message(&suggestions_prompt)];
						let suggestions_response =
							self.llm.generate(&human_message).await.map_err(|e| {
								InsightError::new(
									InsightErrorKind::Internal,
									anyhow::anyhow!("Failed to generate suggestions: {:?}", e)
										.into(),
								)
							})?;
						return Ok(InsightOutput {
							data: Value::String(
								suggestions_response.generation.replace("\\n", " "),
							),
						});
					}
					let mut fetched_results = Vec::new();
					let mut _total_fetched = 0;
					let search_results = if !self.config.discovery_session_id.is_empty() {
						storage
							.get_discovered_data(
								self.config.discovery_session_id.clone(),
								self.config.semantic_pipeline_id.clone(),
							)
							.await
							.map(|results: Vec<DiscoveredKnowledge>| {
								results
									.into_iter()
									.filter_map(|discovered| {
										let sentence_embedding = embedding_model
											.embed(vec![discovered.sentence.clone()], None)
											.ok()?
											.get(0)?
											.clone();
										let similarity =
											cosine_similarity(&sentence_embedding, query_embedding);
										if similarity >= 0.5 {
											Some(discovered)
										} else {
											None
										}
									})
									.collect::<Vec<DiscoveredKnowledge>>()
							})
					} else {
						storage
							.similarity_search_l2(
								self.config.discovery_session_id.to_string(),
								query.to_string(),
								self.config.semantic_pipeline_id.to_string(),
								query_embedding,
								100,
								0,
								&vec![],
							)
							.await
							.map(|results: Vec<DocumentPayload>| {
								results
									.into_iter()
									.map(DiscoveredKnowledge::from_document_payload)
									.collect::<Vec<DiscoveredKnowledge>>()
							})
					};
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
									*total_strength += document.score.unwrap_or(0.0) as f32;
								} else {
									let mut tags_set = HashSet::new();
									tags_set.insert(tag);
									combined_results.insert(
										document.sentence.clone(),
										(tags_set, document.score.unwrap_or(0.0) as f32),
									);
									ordered_sentences.push(document.sentence.clone());
								}
							}

							for sentence in ordered_sentences {
								if documents.len() >= 10 {
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
											let formatted_document = proto::discovery::Insight {
												document: source.doc_id.clone(),
												source: source.doc_source.clone(),
												relationship_strength: total_strength.to_string(),
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

					let unfiltered_results = extract_sentences(&fetched_results).join("\n");
					let _unfiltered_prompt = format!(
						"Query: {}\n\n\
                         -Data-\n: {}\n\
                         #######\n\
                         Output:",
						query, unfiltered_results
					);

					let numbered_sentences: Vec<String> = unfiltered_results
						.lines()
						.enumerate()
						.map(|(i, s)| format!("{}. {}", i + 1, s.trim()))
						.collect();

					let context = numbered_sentences
						.iter()
						.map(|s| s.as_str())
						.collect::<Vec<&str>>()
						.join("\n");
					let final_prompt = if prompt.is_empty() {
						get_final_prompt(query, &context)
					} else {
						format!(
							"{}\n\n#######\n\
                             -Data-\n\
                             Query: {}\n\
                             Summaries: {}\n\
                             #######\n\
                             Output:",
							prompt.to_string(),
							query,
							context
						)
					};

					let human_message = vec![Message::new_human_message(&final_prompt)];
					let summary = self.llm.generate(&human_message).await.map_err(|e| {
						InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Failed to generate summary: {:?}", e).into(),
						)
					})?;
					let generation_text = summary.generation.replace("\\n", "\n");

					storage
						.insert_insight_knowledge(
							Some(query.to_string()),
							Some(session_id.to_string()),
							Some(generation_text.to_string()),
						)
						.await
						.map_err(|e| {
							InsightError::new(
								InsightErrorKind::Internal,
								anyhow::anyhow!("Failed to insert insight knowledge: {:?}", e)
									.into(),
							)
						})?;

					let combined_summary = format!("{}", generation_text);

					return Ok(InsightOutput { data: Value::String(combined_summary) });
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
		input: Pin<Box<dyn Stream<Item = InsightInput> + Send + 'life0>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<InsightOutput>> + Send + 'life0>>> {
		let embedding_model = self.embedding_model.as_ref().map(Arc::new);
		let stream = Box::pin(stream! {
			pin_mut!(input);
			while let Some(input) = input.next().await {

				let session_id = match input.data.get("session_id").and_then(Value::as_str) {
					Some(sid) => sid.to_string(),
					None => {
						yield Err(InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Session ID is missing").into(),
						));
						continue;
					}
				};

				let query = input.data.get("query").and_then(Value::as_str);
				let query = match query {
					Some(q) => q,
					None => {
						tracing::info!("Query is missing. Generating auto-suggestions.");
						"Auto_Suggest"
					},
				};

				let prompt: &str = if self.prompt.is_empty() {
					tracing::info!("User did not provide a prompt. Going to use default prompt.");
					""
				} else {
					&self.prompt
				};

				let embedding_model = match embedding_model.as_ref() {
					Some(model) => model,
					None => {
						yield Err(InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Embedding model is not initialized").into(),
						));
						continue;
					}
				};

				let embeddings = match embedding_model.embed(vec![query], None) {
					Ok(emb) => emb,
					Err(e) => {
						yield Err(InsightError::new(
							InsightErrorKind::Internal,
							anyhow::anyhow!("Failed to embed query: {:?}", e).into(),
						));
						continue;
					}
				};

				let query_embedding = &embeddings[0];
				let mut documents = Vec::new();
				let mut unique_sentences: HashSet<String> = HashSet::new();
				for (event_type, storages) in self.config.event_storages.iter() {
					if *event_type == EventType::Vector {
						for storage in storages.iter() {
							if query.is_empty() {
								let suggestions = match storage.autogenerate_queries(3).await {
									Ok(suggestions) => suggestions,
									Err(e) => {
										yield Err(InsightError::new(
											InsightErrorKind::Internal,
											anyhow::anyhow!("Failed to autogenerate queries: {:?}", e).into(),
										));
										continue;
									}
								};

								let suggestion_texts: Vec<&str> =
									suggestions.iter().map(|s| s.query.as_str()).collect();
								let suggestions_prompt = get_suggestions_prompt(&suggestion_texts);
								let human_message = vec![Message::new_human_message(&suggestions_prompt)];
								let suggestions_response = match self.llm.generate(&human_message).await {
									Ok(response) => response,
									Err(e) => {
										yield Err(InsightError::new(
											InsightErrorKind::Internal,
											anyhow::anyhow!("Failed to generate suggestions: {:?}", e).into(),
										));
										continue;
									}
								};

								yield Ok(InsightOutput {
									data: Value::String(suggestions_response.generation.replace("\\n", " ")),
								});
								continue;
							}
							let mut fetched_results = Vec::new();
							let mut _total_fetched = 0;
							let search_results = if !self.config.discovery_session_id.is_empty() {
								storage
							.get_discovered_data(
								self.config.discovery_session_id.clone(),
								self.config.semantic_pipeline_id.clone(),
							)
							.await
							.map(|results: Vec<DiscoveredKnowledge>| {
								results
									.into_iter()
									.filter_map(|discovered| {
										let sentence_embedding = embedding_model
											.embed(vec![discovered.sentence.clone()], None)
											.ok()?
											.get(0)?
											.clone();
										let similarity =
											cosine_similarity(&sentence_embedding, query_embedding);
										if similarity >= 0.5 {
											Some(discovered)
										} else {
											None
										}
									})
									.collect::<Vec<DiscoveredKnowledge>>()
							})
							} else {
								storage
									.similarity_search_l2(
										self.config.discovery_session_id.to_string(),
										query.to_string(),
										self.config.semantic_pipeline_id.to_string(),
										query_embedding,
										100,
										0,
										&vec![],
									)
									.await
									.map(|results: Vec<DocumentPayload>| {
										results
											.into_iter()
											.map(DiscoveredKnowledge::from_document_payload)
											.collect::<Vec<DiscoveredKnowledge>>()
									})
							};
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
											*total_strength += document.score.unwrap_or(0.0) as f32;
										} else {
											let mut tags_set = HashSet::new();
											tags_set.insert(tag);
											combined_results.insert(
												document.sentence.clone(),
												(tags_set, document.score.unwrap_or(0.0) as f32),
											);
											ordered_sentences.push(document.sentence.clone());
										}
									}

								for sentence in ordered_sentences {
									if documents.len() >= 10 {
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

							let unfiltered_results = extract_sentences(&fetched_results).join("\n");
							let _unfiltered_prompt = format!(
								"Query: {}\n\n\
								 -Data-\n: {}\n\
								 #######\n\
								 Output:",
								query, unfiltered_results
							);

							let numbered_sentences: Vec<String> = unfiltered_results
								.lines()
								.enumerate()
								.map(|(i, s)| format!("{}. {}", i + 1, s.trim()))
								.collect();

							let context = numbered_sentences
								.iter()
								.map(|s| s.as_str())
								.collect::<Vec<&str>>()
								.join("\n");
							let final_prompt = if prompt.is_empty() {
								get_final_prompt(query, &context)
							} else {
								format!(
									"{}\n\n#######\n\
									 -Data-\n\
									 Query: {}\n\
									 Summaries: {}\n\
									 #######\n\
									 Output:",
									prompt.to_string(),
									query,
									context
								)
							};

							let human_message = vec![Message::new_human_message(&final_prompt)];
							let summary = match self.llm.generate(&human_message).await {
								Ok(summary) => summary,
								Err(e) => {
									yield Err(InsightError::new(
										InsightErrorKind::Internal,
										anyhow::anyhow!("Failed to generate summary: {:?}", e).into(),
									));
									continue;
								}
							};

							let generation_text = summary.generation.replace("\\n", "\n");

							if let Err(e) = storage.insert_insight_knowledge(
								Some(query.to_string()),
								Some(session_id.to_string()),
								Some(generation_text.to_string()),
							).await {
								yield Err(InsightError::new(
									InsightErrorKind::Internal,
									anyhow::anyhow!("Failed to insert insight knowledge: {:?}", e).into(),
								));
								continue;
							}

							let combined_summary = format!(
								"{}",
								generation_text
							);

							yield Ok(InsightOutput { data: Value::String(combined_summary) });
						}
					}
				}

				yield Err(InsightError::new(
					InsightErrorKind::NotFound,
					anyhow::anyhow!("No relevant insights found").into(),
				));
			}
		});

		Ok(stream)
	}
}
