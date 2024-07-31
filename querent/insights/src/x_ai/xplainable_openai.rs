use crate::{
	prompts::get_suggestions_prompt, rerank_documents, ConfigCallbackResponse, CustomInsightOption,
	Insight, InsightConfig, InsightCustomOptionValue, InsightError, InsightErrorKind, InsightInfo,
	InsightInput, InsightOutput, InsightResult, InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use common::EventType;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use futures::{pin_mut, Stream, StreamExt};
use llms::{Message, OpenAI, OpenAIConfig, LLM};
use serde_json::Value;
use storage::{extract_unique_pairs, find_intersection, get_top_k_pairs};
use tracing::error;
// use tokio::sync::RwLock;
use crate::insight_utils::unique_sentences;
use std::{
	collections::HashMap,
	pin::Pin,
	sync::{Arc, RwLock},
};

use super::prompts::{get_analysis_prompt, get_final_prompt};
/// XAI Insight struct.
pub struct XAI {
	info: InsightInfo,
}

impl XAI {
	pub fn new() -> Self {
		let mut additional_options = HashMap::new();
		additional_options.insert(
			"openai_api_key".to_string(),
			CustomInsightOption {
				id: "openai_api_key".to_string(),
				label: "OpenAI API Key".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("OpenAI API Key".to_string()),
			},
		);
		Self {
			info: InsightInfo {
				id: "querent.insights.x_ai.openai".to_string(),
				name: "Querent xAI with GPT".to_string(),
				description: "xAI utilizes generative models to perform a directed traversal in Querent's attention data fabric.".to_string(),
				version: "1.0.0".to_string(),
				author: "Querent AI".to_string(),
				license: "Apache-2.0".to_string(),
				icon: &[], // Add your icon bytes here.
				additional_options,
				conversational: true,
				premium: false,
			},
		}
	}
}

pub struct XAIRunner {
	pub config: InsightConfig,
	pub llm: Arc<dyn LLM>,
	pub embedding_model: Option<TextEmbedding>,
	pub previous_query_results: RwLock<String>,
	pub previous_filtered_results: RwLock<Vec<(String, String)>>,
	pub previous_session_id: RwLock<String>,
}

#[async_trait]
impl Insight for XAI {
	async fn info(&self) -> InsightInfo {
		self.info.clone()
	}

	fn supports_streaming(&self) -> bool {
		true
	}

	fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
		ConfigCallbackResponse::Empty
	}

	fn get_runner(&self, config: &InsightConfig) -> InsightResult<Arc<dyn InsightRunner>> {
		let openai_api_key = config.get_custom_option("openai_api_key");
		if openai_api_key.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("OpenAI API Key is required").into(),
			));
		}
		let openai_api_key = openai_api_key.unwrap().value.clone();
		let openai_api_key = match openai_api_key {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("OpenAI API Key is required").into(),
				));
			},
		};
		let default_openai_config: OpenAIConfig =
			OpenAIConfig::default().with_api_key(openai_api_key);
		let openai_llm = OpenAI::new(default_openai_config);

		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		})
		.map_err(|e| InsightError::new(InsightErrorKind::Internal, e.into()))?;

		Ok(Arc::new(XAIRunner {
			config: config.clone(),
			llm: Arc::new(openai_llm),
			embedding_model: Some(embedding_model),
			previous_query_results: RwLock::new(String::new()),
			previous_filtered_results: RwLock::new(Vec::new()),
			previous_session_id: RwLock::new(String::new()),
		}))
	}
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

		let prompt = input.data.get("prompt").and_then(Value::as_str);
		let prompt = match prompt {
			Some(q) => q,
			None => {
				tracing::info!("Prompt is missing. Going to use default prompt.");
				"Default Prompt"
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
		let mut numbered_sentences: Vec<String> = Vec::new();

		let mut all_discovered_data: Vec<(
			String,
			String,
			String,
			String,
			String,
			String,
			String,
			f32,
		)> = Vec::new();
		for (event_type, storages) in self.config.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storages.iter() {
					if query.is_empty() {
						let suggestions = storage.autogenerate_queries(10).await.map_err(|e| {
							InsightError::new(
								InsightErrorKind::Internal,
								anyhow::anyhow!("Failed to autogenerate queries: {:?}", e).into(),
							)
						})?;
						let suggestion_texts: Vec<String> =
							suggestions.iter().map(|s| s.query.clone()).collect();
						let suggestions_prompt = get_suggestions_prompt(&suggestion_texts);
						let human_message = vec![Message::new_human_message(&suggestions_prompt)];
						println!("Human Message -----------------{:?}", human_message);
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
					let search_results = storage
						.similarity_search_l2(
							self.config.discovery_session_id.clone(),
							query.to_string(),
							self.config.semantic_pipeline_id.clone(),
							query_embedding,
							10,
							0,
						)
						.await;

					match search_results {
						Ok(results) => {
							let filtered_results = get_top_k_pairs(results.clone(), 2);
							let traverser_results_1 =
								storage.traverse_metadata_table(filtered_results.clone()).await;

							if self
								.previous_query_results
								.read()
								.map_or(true, |results| results.is_empty()) ||
								self.previous_session_id
									.read()
									.map_or(true, |id| *id != session_id)
							{
								match &traverser_results_1 {
									Ok(ref traverser_results) => {
										if let Ok(mut prev_query_results) =
											self.previous_query_results.write()
										{
											*prev_query_results =
												serde_json::to_string(traverser_results)
													.unwrap_or_default();
										} else {
											error!("Failed to acquire write lock on previous_query_results");
										}

										if let Ok(mut prev_filtered_results) =
											self.previous_filtered_results.write()
										{
											*prev_filtered_results = filtered_results.clone();
										} else {
											error!("Failed to acquire write lock on previous_filtered_results");
										}

										if let Ok(mut prev_session_id) =
											self.previous_session_id.write()
										{
											*prev_session_id = session_id.to_string();
										} else {
											error!("Failed to acquire write lock on previous_session_id");
										}

										all_discovered_data.extend(traverser_results.clone());
										let (unique_sentences, _count) =
											unique_sentences(&all_discovered_data);
										if let Some(reranked_results) =
											rerank_documents(query, unique_sentences.clone())
										{
											let top_10_reranked = reranked_results
												.into_iter()
												.take(10)
												.collect::<Vec<_>>();
											numbered_sentences = top_10_reranked
												.iter()
												.enumerate()
												.map(|(i, (s, _))| format!("{}. {}", i + 1, s))
												.collect();
										} else {
											numbered_sentences = unique_sentences
												.iter()
												.enumerate()
												.map(|(i, s)| format!("{}. {}", i + 1, s))
												.collect();
										}
									},
									Err(e) => {
										error!("Failed to serialize traverser results: {:?}", e);
									},
								}
							} else {
								let previous_results: Vec<(
									String,
									String,
									String,
									String,
									String,
									String,
									String,
									f32,
								)> = match self.previous_query_results.read() {
									Ok(query_results) =>
										serde_json::from_str(&query_results).unwrap_or_default(),
									Err(e) => {
										error!("Failed to acquire read lock on previous_query_results: {:?}", e);
										vec![]
									},
								};
								let current_results = match &traverser_results_1 {
									Ok(ref traverser_results) => traverser_results.clone(),
									Err(e) => {
										error!("Failed to search for similar documents in traverser: {:?}", e);
										vec![]
									},
								};

								let formatted_output_1 = extract_unique_pairs(
									current_results.clone(),
									filtered_results.clone(),
								);
								let formatted_output_2 = match self.previous_filtered_results.read()
								{
									Ok(filtered_results) => extract_unique_pairs(
										previous_results.clone(),
										filtered_results.clone(),
									),
									Err(e) => {
										error!("Failed to acquire read lock on previous_filtered_results: {:?}", e);
										extract_unique_pairs(previous_results.clone(), vec![])
									},
								};

								let results_intersection = find_intersection(
									formatted_output_1.clone(),
									formatted_output_2.clone(),
								);

								let final_traverser_results = if results_intersection.is_empty() {
									if let Ok(mut prev_filtered_results) =
										self.previous_filtered_results.write()
									{
										*prev_filtered_results = formatted_output_1.clone();
									} else {
										error!("Failed to acquire write lock on previous_filtered_results when queries do not match");
									}
									storage
										.traverse_metadata_table(formatted_output_1.clone())
										.await
								} else {
									if let Ok(mut prev_filtered_results) =
										self.previous_filtered_results.write()
									{
										*prev_filtered_results = results_intersection.clone();
									} else {
										error!("Failed to acquire write lock on previous_filtered_resultswhen queries match");
									}
									storage
										.traverse_metadata_table(results_intersection.clone())
										.await
								};

								match final_traverser_results.clone() {
									Ok(ref results) => {
										if let Ok(mut prev_query_results) =
											self.previous_query_results.write()
										{
											*prev_query_results =
												serde_json::to_string(results).unwrap_or_default();
										} else {
											error!("Failed to acquire write lock on previous_query_results post data fabric traversal");
										}
										all_discovered_data.extend(results.clone());
										let (unique_sentences, _count) =
											unique_sentences(&all_discovered_data);
										numbered_sentences = unique_sentences
											.iter()
											.enumerate()
											.map(|(i, s)| format!("{}. {}", i + 1, s))
											.collect();
									},
									Err(e) => {
										error!("Failed to search for similar documents in traverser: {:?}", e);
									},
								}
							}

							let context = numbered_sentences.join("\n");
							let final_prompt = if prompt.is_empty() {
								get_final_prompt(query, &context)
							} else {
								prompt.to_string()
							};
							let human_message = vec![Message::new_human_message(&final_prompt)];
							let summary = self.llm.generate(&human_message).await.map_err(|e| {
								InsightError::new(
									InsightErrorKind::Internal,
									anyhow::anyhow!("Failed to generate summary: {:?}", e).into(),
								)
							})?;
							let answer = summary.generation.replace("\\n", "\n");
							let prompt = get_analysis_prompt(query, &answer);
							let human_message = vec![Message::new_human_message(&prompt)];
							let summary_2 =
								self.llm.generate(&human_message).await.map_err(|e| {
									InsightError::new(
										InsightErrorKind::Internal,
										anyhow::anyhow!("Failed to generate summary: {:?}", e)
											.into(),
									)
								})?;
							let generation_text = summary_2.generation.replace("\\n", "\n");
							storage
								.insert_insight_knowledge(
									Some(query.to_string()),
									Some(session_id.to_string()),
									Some(generation_text.clone()),
								)
								.await
								.map_err(|e| {
									InsightError::new(
										InsightErrorKind::Internal,
										anyhow::anyhow!(
											"Failed to insert insight knowledge: {:?}",
											e
										)
										.into(),
									)
								})?;

							return Ok(InsightOutput { data: Value::String(generation_text) });
						},
						Err(e) => error!("Error retrieving discovered data: {:?}", e),
					}
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
		let config = self.config.clone();
		let llm = self.llm.clone();

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
					let query = match input.data.get("query").and_then(Value::as_str) {
						Some(q) => q.to_string(),
						None => {
							tracing::info!("Query is missing. Generating auto-suggestions.");
							"Auto_Suggest".to_string()
						}
					};

					let prompt = match input.data.get("prompt").and_then(Value::as_str) {
						Some(p) => p.to_string(),
						None => {
							tracing::info!("Prompt is missing. Going to use default prompt.");
							"Default Prompt".to_string()
						}
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
					let embeddings = match embedding_model.embed(vec![query.clone()], None) {
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
					let mut numbered_sentences: Vec<String>= Vec::new();

					let mut all_discovered_data: Vec<(String, String, String, String, String, String, String, f32)> = Vec::new();
					for (event_type, storages) in config.event_storages.iter() {
						if *event_type == EventType::Vector {
							for storage in storages.iter() {
								if query == "Auto_Suggest" {
									let suggestions = match storage.autogenerate_queries(10).await {
										Ok(sugg) => sugg,
										Err(e) => {
											yield Err(InsightError::new(
												InsightErrorKind::Internal,
												anyhow::anyhow!("Failed to autogenerate queries: {:?}", e).into(),
											));
											continue;
										}
									};

									let suggestion_texts: Vec<String> = suggestions.iter().map(|s| s.query.clone()).collect();
									let suggestions_prompt = get_suggestions_prompt(&suggestion_texts);

									let human_message = vec![Message::new_human_message(&suggestions_prompt)];

									let suggestions_response = match llm.generate(&human_message).await {
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
								let search_results = storage.similarity_search_l2(
									config.discovery_session_id.clone(),
									query.clone(),
									config.semantic_pipeline_id.clone(),
									query_embedding,
									10,
									0,
								).await;

								match search_results {
									Ok(results) => {
										let filtered_results = get_top_k_pairs(results.clone(), 2);
										let traverser_results_1 = storage.traverse_metadata_table(filtered_results.clone()).await;

										if self.previous_query_results.read().unwrap().is_empty()
											|| *self.previous_session_id.read().unwrap() != session_id
										{
											match &traverser_results_1 {
												Ok(ref traverser_results) => {
													*self.previous_query_results.write().unwrap() = serde_json::to_string(traverser_results).unwrap_or_default();
													*self.previous_filtered_results.write().unwrap() = filtered_results.clone();
													*self.previous_session_id.write().unwrap() = session_id.to_string();
													let (unique_sentences, _count) = unique_sentences(&all_discovered_data);
													if let Some(reranked_results) = rerank_documents(&query, unique_sentences.clone()) {


														let top_10_reranked = reranked_results.into_iter().take(10).collect::<Vec<_>>();
														numbered_sentences = top_10_reranked
															.iter()
															.enumerate()
															.map(|(i, (s, _))| format!("{}. {}", i + 1, s))
															.collect();
													} else {
														numbered_sentences = unique_sentences
															.iter()
															.enumerate()
															.map(|(i, s)| format!("{}. {}", i + 1, s))
															.collect();
													}
													all_discovered_data.extend(traverser_results.clone());
												}
												Err(e) => {
													error!("Failed to serialize traverser results: {:?}", e);
												}
											}
										} else {
											let previous_results: Vec<(
												String,
												String,
												String,
												String,
												String,
												String,
												String,
												f32,
											)> = serde_json::from_str(&self.previous_query_results.read().unwrap()).unwrap_or_default();
											let current_results = match &traverser_results_1 {
												Ok(ref traverser_results) => traverser_results.clone(),
												Err(e) => {
													error!("Failed to search for similar documents in traverser: {:?}", e);
													vec![]
												}
											};

											let formatted_output_1 = extract_unique_pairs(current_results.clone(), filtered_results.clone());
											let formatted_output_2 = extract_unique_pairs(previous_results.clone(), self.previous_filtered_results.read().unwrap().clone());

											let results_intersection = find_intersection(formatted_output_1.clone(), formatted_output_2.clone());

											let final_traverser_results = if results_intersection.is_empty() {
												*self.previous_filtered_results.write().unwrap() = formatted_output_1.clone();
												storage.traverse_metadata_table(formatted_output_1.clone()).await
											} else {
												*self.previous_filtered_results.write().unwrap() = results_intersection.clone();
												storage.traverse_metadata_table(results_intersection.clone()).await
											};

											match final_traverser_results.clone() {
												Ok(ref results) => {
													*self.previous_query_results.write().unwrap() = serde_json::to_string(results).unwrap_or_default();
													all_discovered_data.extend(results.clone());
													let (unique_sentences, _count) = unique_sentences(&all_discovered_data);
													numbered_sentences = unique_sentences
															.iter()
															.enumerate()
															.map(|(i, s)| format!("{}. {}", i + 1, s))
															.collect();

												}
												Err(e) => {
													error!("Failed to search for similar documents in traverser: {:?}", e);
												}
											}
										}
										let context = numbered_sentences.join("\n");
										let final_prompt = if prompt.is_empty() {
											get_final_prompt(&query, &context)
										} else {
											prompt.to_string()
										};

									let human_message = vec![Message::new_human_message(&final_prompt)];

									let summary = match llm.generate(&human_message).await {
										Ok(sum) => sum,
										Err(e) => {
											yield Err(InsightError::new(
												InsightErrorKind::Internal,
												anyhow::anyhow!("Failed to generate summary: {:?}", e).into(),
											));
											continue;
										}
									};

									let answer = summary.generation.replace("\\n", "\n");
									let prompt = get_analysis_prompt(&query, &answer);
									let human_message = vec![Message::new_human_message(&prompt)];
									let summary_2 = self.llm.generate(&human_message).await.map_err(|e| {
										InsightError::new(
											InsightErrorKind::Internal,
											anyhow::anyhow!("Failed to generate summary: {:?}", e).into(),
										)
									})?;
									let generation_text = summary_2.generation.replace("\\n", "\n");
									match storage.insert_insight_knowledge(
										Some(query.to_string()),
										Some(session_id.to_string()),
										Some(generation_text.clone())
									).await {
										Ok(_) => {},
										Err(e) => {
											yield Err(InsightError::new(
												InsightErrorKind::Internal,
												anyhow::anyhow!("Failed to insert insight knowledge: {:?}", e).into(),
											));
											continue;
										}
									};

									yield Ok(InsightOutput { data: Value::String(generation_text) });
								}
								Err(e) => error!("Error retrieving discovered data: {:?}", e),
							}
						}
					}
				}
			}
		});

		Ok(stream)
	}
}
