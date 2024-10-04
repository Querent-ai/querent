use crate::{
	models::*, postgres_index::QuerySuggestion, semantic_knowledge, utils::traverse_node,
	ActualDbPool, DieselError, FabricAccessor, FabricStorage, Storage, StorageError,
	StorageErrorKind, StorageResult, POOL_TIMEOUT,
};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use deadpool::Runtime;
use diesel::{
	sql_types::{Array, BigInt, Text},
	ExpressionMethods, QueryDsl,
};
use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
	scoped_futures::ScopedFutureExt,
	AsyncConnection, RunQueryDsl,
};
use pgvector::Vector;
use proto::semantics::PostgresConfig;
use std::{collections::HashSet, sync::Arc};
use tracing::error;

use super::{fetch_documents_for_embedding, FilteredSemanticKnowledge};

pub struct PGVector {
	pub pool: ActualDbPool,
	pub config: PostgresConfig,
}

impl PGVector {
	pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
		let tls_enabled = config.url.contains("sslmode=require");
		let manager = if tls_enabled {
			// // diesel-async does not support any TLS connections out of the box, so we need to manually
			// // provide a setup function which handles creating the connection
			// let mut d_config = ManagerConfig::default();
			// d_config.custom_setup = Box::new(establish_connection);
			// AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
			// 	config.url.clone(),
			// 	d_config,
			// )
			// TODO: Support for TLS
			log::warn!("TLS in pg connection directly is not supported yet");
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.url.clone())
		} else {
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.url.clone())
		};
		let pool = Pool::builder(manager)
			.max_size(10)
			.wait_timeout(POOL_TIMEOUT)
			.create_timeout(POOL_TIMEOUT)
			.recycle_timeout(POOL_TIMEOUT)
			.runtime(Runtime::Tokio1)
			.build()
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		Ok(PGVector { pool, config })
	}
}

#[async_trait]
impl FabricStorage for PGVector {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let transaction_result = conn
			.transaction::<_, DieselError, _>(|conn| {
				Box::pin(async move {
					for (_document_id, _source, _image_id, item) in payload {
						let form = EmbeddedKnowledge {
							embeddings: Some(Vector::from(item.embeddings.clone())),
							score: item.score,
							event_id: item.event_id.clone(),
						};
						diesel::insert_into(embedded_knowledge::dsl::embedded_knowledge)
							.values(form)
							.execute(conn)
							.await?;
					}
					Ok(())
				})
			})
			.await;

		transaction_result.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		Ok(())
	}

	async fn insert_discovered_knowledge(
		&self,
		payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for item in payload {
					let form = DiscoveredKnowledge::from_document_payload(item.clone());
					diesel::insert_into(discovered_knowledge::dsl::discovered_knowledge)
						.values(form)
						.execute(conn)
						.await?;
				}
				Ok(())
			}
			.scope_boxed()
		})
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		Ok(())
	}

	async fn insert_insight_knowledge(
		&self,
		query: Option<String>,
		session_id: Option<String>,
		response: Option<String>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				let new_knowledge = InsightKnowledge { query, session_id, response };
				diesel::insert_into(insight_knowledge::table)
					.values(&new_knowledge)
					.execute(conn)
					.await?;
				Ok(())
			}
			.scope_boxed()
		})
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

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
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let mut results: Vec<DocumentPayload> = Vec::new();

		if top_pairs_embeddings.is_empty() || top_pairs_embeddings.len() == 1 {
			let embedding = if top_pairs_embeddings.is_empty() {
				payload.clone()
			} else {
				top_pairs_embeddings[0].clone()
			};
			results.extend(
				fetch_documents_for_embedding(
					&mut conn,
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
						&mut conn,
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

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}
}

#[async_trait]
impl FabricAccessor for PGVector {
	async fn filter_and_query(
		&self,
		session_id: &String,
		top_pairs: &Vec<String>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let subjects_objects: Vec<String> =
			top_pairs.iter().flat_map(|pair| pair.split(" - ").map(String::from)).collect();

		let query = format!(
			"WITH ranked_results AS (
					SELECT 
						semantic_knowledge.id, 
						semantic_knowledge.document_id, 
						semantic_knowledge.subject, 
						semantic_knowledge.object, 
						semantic_knowledge.document_source, 
						semantic_knowledge.sentence, 
						semantic_knowledge.event_id,
						embedded_knowledge.score,
						embedded_knowledge.embeddings,
						
						CASE
							WHEN (semantic_knowledge.subject || ' - ' || semantic_knowledge.object) = ANY($1) 
								OR (semantic_knowledge.object || ' - ' || semantic_knowledge.subject) = ANY($1) THEN 2
							WHEN semantic_knowledge.subject = ANY($2) OR semantic_knowledge.object = ANY($2) THEN 1
							ELSE 0
						END AS match_rank
					FROM 
						semantic_knowledge
					JOIN 
						embedded_knowledge ON semantic_knowledge.event_id = embedded_knowledge.event_id
				)
				SELECT id, document_id, subject, object, document_source, sentence, event_id, score, embeddings
				FROM ranked_results
				ORDER BY match_rank DESC, score DESC
				OFFSET $4
				LIMIT $3"
		);
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let results: Vec<FilteredResults> = diesel::sql_query(query)
			.bind::<Array<Text>, _>(top_pairs)
			.bind::<Array<Text>, _>(subjects_objects)
			.bind::<diesel::sql_types::Integer, _>(max_results)
			.bind::<BigInt, _>(offset)
			.load::<FilteredResults>(&mut conn)
			.await
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		let document_payloads = results
			.into_iter()
			.map(|result| DocumentPayload {
				doc_id: result.document_id,
				doc_source: result.document_source,
				sentence: result.sentence,
				knowledge: format!("{} - {}", result.subject, result.object),
				subject: result.subject,
				object: result.object,
				cosine_distance: None,
				query_embedding: Some(result.embeddings.to_vec()),
				query: Some("".to_string()),
				session_id: Some(session_id.clone()),
				score: result.score,
				collection_id: String::new(),
			})
			.collect();

		Ok(document_payloads)
	}

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
			let conn = &mut self.pool.get().await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

			// Traverse depth 1
			traverse_node(
				&self.pool,
				head.to_string(),
				&mut combined_results,
				&mut visited_pairs,
				conn,
				0,
				"inward",
			)
			.await?;

			traverse_node(
				&self.pool,
				tail.to_string(),
				&mut combined_results,
				&mut visited_pairs,
				conn,
				0,
				"outward",
			)
			.await?;
		}

		Ok(combined_results)
	}

	/// Asynchronously fetches suggestions from semantic table.
	async fn autogenerate_queries(
		&self,
		max_suggestions: i32,
	) -> Result<Vec<QuerySuggestion>, StorageError> {
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let top_pairs_query_result = semantic_knowledge::dsl::semantic_knowledge
			.select((
				semantic_knowledge::dsl::subject,
				semantic_knowledge::dsl::subject_type,
				semantic_knowledge::dsl::object,
				semantic_knowledge::dsl::object_type,
				diesel::dsl::sql::<BigInt>("COUNT(*) as pair_frequency"),
			))
			.group_by((
				semantic_knowledge::dsl::subject,
				semantic_knowledge::dsl::subject_type,
				semantic_knowledge::dsl::object,
				semantic_knowledge::dsl::object_type,
			))
			.order_by(diesel::dsl::sql::<BigInt>("pair_frequency").desc())
			.limit(1000)
			.load::<(String, String, String, String, i64)>(&mut conn)
			.await;

		let top_pairs = match top_pairs_query_result {
			Ok(pairs) => pairs,
			Err(e) => {
				error!("Error fetching top pairs: {:?}", e);
				return Err(StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				});
			},
		};

		let mut suggestions = Vec::new();

		if !top_pairs.is_empty() {
			let mut pairs = Vec::new();
			let mut pair_strings = Vec::new();
			let mut seen_pairs = HashSet::new();
			for (subject, subject_type, object, object_type, frequency) in top_pairs.into_iter() {
				let pair = if subject < object {
					(subject.clone(), object.clone())
				} else {
					(object.clone(), subject.clone())
				};
				if seen_pairs.insert(pair.clone()) {
					pairs.push((subject, subject_type, object, object_type, frequency));
					pair_strings.push(format!("{} - {}", pair.0, pair.1));
				}
				if pair_strings.len() >= 10 {
					break;
				}
			}
			suggestions.push(QuerySuggestion {
				query: pair_strings.join(", "),
				frequency: 1,
				document_source: String::new(),
				sentence: String::new(),
				tags: vec!["Filters".to_string()],
				top_pairs: pair_strings,
			});
		}
		if max_suggestions > 1 {
			let bottom_pairs_query_result = semantic_knowledge::dsl::semantic_knowledge
				.select((
					semantic_knowledge::dsl::subject,
					semantic_knowledge::dsl::subject_type,
					semantic_knowledge::dsl::object,
					semantic_knowledge::dsl::object_type,
					diesel::dsl::sql::<BigInt>("COUNT(*) as pair_frequency"),
				))
				.group_by((
					semantic_knowledge::dsl::subject,
					semantic_knowledge::dsl::subject_type,
					semantic_knowledge::dsl::object,
					semantic_knowledge::dsl::object_type,
				))
				.order_by(diesel::dsl::sql::<BigInt>("pair_frequency").asc())
				.limit(5)
				.load::<(String, String, String, String, i64)>(&mut conn)
				.await;

			let bottom_pairs = match bottom_pairs_query_result {
				Ok(pairs) => pairs,
				Err(e) => {
					error!("Error fetching bottom pairs: {:?}", e);
					return Err(StorageError {
						kind: StorageErrorKind::Query,
						source: Arc::new(anyhow::Error::from(e)),
					});
				},
			};

			let most_mix_documents_query_result = semantic_knowledge::dsl::semantic_knowledge
				.select((
					semantic_knowledge::dsl::document_id,
					diesel::dsl::sql::<BigInt>(
						"COUNT(DISTINCT subject) + COUNT(DISTINCT object) as entity_mix",
					),
				))
				.group_by(semantic_knowledge::dsl::document_id)
				.order_by(diesel::dsl::sql::<BigInt>("entity_mix").desc())
				.limit(5)
				.load::<(String, i64)>(&mut conn)
				.await;

			let most_mix_documents = match most_mix_documents_query_result {
				Ok(docs) => docs,
				Err(e) => {
					error!("Error fetching documents with most mix of entities: {:?}", e);
					return Err(StorageError {
						kind: StorageErrorKind::Query,
						source: Arc::new(anyhow::Error::from(e)),
					});
				},
			};

			if !bottom_pairs.is_empty() {
				let mut pairs = Vec::new();
				for (subject, subject_type, object, object_type, frequency) in
					bottom_pairs.into_iter().take(5)
				{
					pairs.push((subject, subject_type, object, object_type, frequency));
				}
				let combined_query = format!(
						"Explore rare and potentially significant connections within your semantic data fabric.  These connections unveil the underlying patterns and dynamics woven into your data landscape. Noteworthy interactions are found between '{}' and '{}', along with the link between '{}' and '{}'.",
						pairs[0].0, pairs[0].2,
						pairs[1].0, pairs[1].2);

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
				for (document_id, entity_mix) in most_mix_documents.into_iter().take(5) {
					documents.push((document_id, entity_mix));
				}
				let combined_query = format!(
						"Certain documents stand out for their rich diversity of semantic connections, reflecting a broad spectrum of topics. For instance, document '{}' reveals a complex network of unique data points, followed by '{}'.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0),
						documents[1].0.rsplit('/').next().unwrap_or(&documents[1].0),
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

	async fn get_discovered_data(
		&self,
		discovery_session_id: String,
		pipeline_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let results = conn
			.transaction::<_, diesel::result::Error, _>(|conn| {
				async move {
					let mut query = discovered_knowledge::dsl::discovered_knowledge
						.select((
							discovered_knowledge::dsl::doc_id,
							discovered_knowledge::dsl::doc_source,
							discovered_knowledge::dsl::sentence,
							discovered_knowledge::dsl::subject,
							discovered_knowledge::dsl::object,
							discovered_knowledge::dsl::cosine_distance,
							discovered_knowledge::dsl::session_id,
							discovered_knowledge::dsl::score,
							discovered_knowledge::dsl::query,
							discovered_knowledge::dsl::query_embedding,
							discovered_knowledge::dsl::collection_id,
						))
						.into_boxed();
					if !discovery_session_id.is_empty() {
						query = query
							.filter(discovered_knowledge::dsl::session_id.eq(&discovery_session_id))
					}
					if !pipeline_id.is_empty() {
						query =
							query.filter(discovered_knowledge::dsl::collection_id.eq(&pipeline_id));
					}

					let query_result = query
						.load::<(
							String,
							String,
							String,
							String,
							String,
							Option<f64>,
							Option<String>,
							Option<f64>,
							Option<String>,
							Option<Vector>,
							String,
						)>(conn)
						.await;

					match query_result {
						Ok(result) => {
							let mut results: Vec<DiscoveredKnowledge> = Vec::new();
							for (
								doc_id,
								doc_source,
								sentence,
								subject,
								object,
								cosine_distance,
								session_id,
								score,
								query,
								query_embedding,
								collection_id,
							) in result
							{
								let doc_payload = DiscoveredKnowledge {
									doc_id,
									doc_source,
									sentence,
									subject,
									object,
									cosine_distance,
									session_id,
									score,
									query,
									query_embedding,
									collection_id,
								};
								results.push(doc_payload);
							}
							Ok(results)
						},
						Err(e) => {
							eprintln!("Error querying semantic data: {:?}", e);
							Err(e)
						},
					}
				}
				.scope_boxed()
			})
			.await
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		Ok(results)
	}

	async fn get_semanticknowledge_data(
		&self,
		collection_id: &str,
	) -> StorageResult<Vec<FilteredSemanticKnowledge>> {
		let query = format!(
			"SELECT subject, subject_type, object, object_type, sentence, image_id, event_id, source_id, document_source, document_id
			FROM semantic_knowledge 
			WHERE collection_id = $1"
		);

		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let results: Vec<FilteredSemanticKnowledge> = diesel::sql_query(query)
			.bind::<Text, _>(collection_id)
			.load::<FilteredSemanticKnowledge>(&mut conn)
			.await
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		Ok(results)
	}
}

impl Storage for PGVector {}
