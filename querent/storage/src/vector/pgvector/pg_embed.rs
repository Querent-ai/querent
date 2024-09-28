use crate::{
	enable_extension, models::*, postgres_index::QuerySuggestion, semantic_knowledge,
	utils::traverse_node, ActualDbPool, DieselError, FabricAccessor, FabricStorage,
	SemanticKnowledge, Storage, StorageError, StorageErrorKind, StorageResult, POOL_TIMEOUT,
};
use diesel_migrations::MigrationHarness;

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{
	r2d2::{ConnectionManager, Pool},
	sql_types::{Array, BigInt, Double, Float4, Nullable, Text},
	ExpressionMethods, PgConnection, QueryDsl,
};
use diesel_async::{
	pg::AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager,
	scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use postgresql_embedded::Result;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tracing::error;

use super::{fetch_documents_for_embedding_pgembed, parse_vector};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/vector/pgvector/migrations/");

pub struct PGEmbed {
	pub pool: ActualDbPool,
}

impl PGEmbed {
	pub async fn new(database_url: String) -> StorageResult<Self> {
		let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.clone());
		let pool = diesel_async::pooled_connection::deadpool::Pool::builder(manager)
			.max_size(10)
			.wait_timeout(POOL_TIMEOUT)
			.create_timeout(POOL_TIMEOUT)
			.recycle_timeout(POOL_TIMEOUT)
			.runtime(deadpool::Runtime::Tokio1)
			.build()
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		// Enable the extension
		enable_extension(&pool).await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let manager_local = ConnectionManager::<PgConnection>::new(database_url.clone());
		let pool_local = Pool::builder()
			.test_on_check_out(true)
			.build(manager_local)
			.expect("Could not build connection pool");
		let mut mig_run = pool_local.clone().get().unwrap();
		mig_run.run_pending_migrations(MIGRATIONS).unwrap();
		Ok(PGEmbed { pool })
	}
}

#[async_trait]
impl FabricStorage for PGEmbed {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let max_retries = 5;
		let mut retries = 0;

		loop {
			match self.pool.get().await {
				Ok(_) => return Ok(()), // Connected successfully
				Err(e) if retries < max_retries => {
					retries += 1;
					eprintln!("Failed to connect with error: {:?}", e);
					tokio::time::sleep(Duration::from_secs(2)).await; // Delay between retries
				},
				Err(e) => {
					log::error!("Failed to connect to database: {:?}", e);
				},
			}
		}
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
						let embeddings_array =
							item.embeddings.iter().map(|v| *v).collect::<Vec<f32>>();
						match diesel::sql_query(
							"INSERT INTO embedded_knowledge (embeddings, score, event_id) VALUES ($1::vector, $2, $3)",
						)
						.bind::<diesel::sql_types::Array<Float4>, _>(embeddings_array)
						.bind::<Float4, _>(item.score)
						.bind::<Text, _>(item.event_id.clone())
						.execute(conn)
						.await
						{
							Ok(_) => {
								tracing::debug!("Successfully inserted vector for event_id: {}", item.event_id);
							}
							Err(e) => {
								tracing::error!(
									"Error inserting vector for event_id: {}: {:?}",
									item.event_id, e
								);
								return Err(e);
							}
						}
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

		let transaction_result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
			Box::pin(async move {
				for item in payload {
					if let Some(embedding) = &item.query_embedding {
						let embeddings_array = embedding.iter().copied().collect::<Vec<f32>>();
						match diesel::sql_query(
							"INSERT INTO discovered_knowledge (doc_id, doc_source, sentence, subject, object, cosine_distance, query_embedding, query, session_id, score, collection_id) 
							 VALUES ($1, $2, $3, $4, $5, $6, $7::vector, $8, $9, $10, $11)",
						)
						.bind::<Text, _>(item.doc_id.clone())
						.bind::<Text, _>(item.doc_source.clone())
						.bind::<Text, _>(item.sentence.clone())
						.bind::<Text, _>(item.subject.clone())
						.bind::<Text, _>(item.object.clone())
						.bind::<Nullable<diesel::sql_types::Double>, _>(item.cosine_distance)
						.bind::<diesel::sql_types::Array<Float4>, _>(embeddings_array)
						.bind::<Nullable<Text>, _>(item.query.clone())
						.bind::<Nullable<Text>, _>(item.session_id.clone())
						.bind::<Float4, _>(item.score)
						.bind::<Text, _>(item.collection_id.clone())
						.execute(conn)
						.await
						{
							Ok(_) => {
								tracing::debug!(
									"Successfully inserted discovered knowledge for doc_id: {}",
									item.doc_id
								);
							}
							Err(e) => {
								tracing::error!(
									"Error inserting discovered knowledge for doc_id: {}: {:?}",
									item.doc_id, e
								);
								return Err(e);
							}
						}
					} else {
						tracing::warn!(
							"Skipping insertion for doc_id: {} due to missing query_embedding.",
							item.doc_id
						);
					}
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
				fetch_documents_for_embedding_pgembed(
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
					fetch_documents_for_embedding_pgembed(
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
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
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
					diesel::insert_into(semantic_knowledge::dsl::semantic_knowledge)
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
}

#[async_trait]
impl FabricAccessor for PGEmbed {
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
				let combined_query: String;
				if documents.len() == 1 {
					combined_query =  format!(
						"A unique document stands out for its rich diversity of semantic connections, reflecting a broad spectrum of topics: '{}'.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0),
					);
				} else {
					combined_query =  format!(
						"Certain documents stand out for their rich diversity of semantic connections, reflecting a broad spectrum of topics. For instance, document '{}' reveals a complex network of unique data points, followed by '{}'.",
						documents[0].0.rsplit('/').next().unwrap_or(&documents[0].0),
						documents[1].0.rsplit('/').next().unwrap_or(&documents[1].0),
					);
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

	async fn get_discovered_data(
		&self,
		session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let query = format!(
			"SELECT doc_id, doc_source, sentence, subject, object, cosine_distance, session_id, score, query, 
			array_to_string(query_embedding::real[], ',') as query_embedding, collection_id
			 FROM discovered_knowledge
			 WHERE session_id = '{}'",
			session_id
		);
		let results: Vec<DiscoveredKnowledgeRaw> = match diesel::sql_query(query).load(conn).await {
			Ok(res) => res,
			Err(e) => {
				eprintln!("Error querying discovered knowledge data: {:?}", e);
				return Err(StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				});
			},
		};
		let discovered_knowledge = results
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
				query_embedding: parse_vector(item.query_embedding),
				collection_id: item.collection_id,
			})
			.collect();

		Ok(discovered_knowledge)
	}
}

impl Storage for PGEmbed {}

#[cfg(test)]
use diesel::sql_query;
use diesel::QueryableByName;

#[allow(dead_code)]
#[derive(QueryableByName, Debug)]
struct RowCount {
	#[diesel(sql_type = diesel::sql_types::BigInt)]
	count: i64,
}

#[allow(dead_code)]
#[derive(QueryableByName, Debug)]
struct InsertedDataRaw {
	#[diesel(sql_type = Text)]
	embeddings: String,
	#[diesel(sql_type = Float4)]
	score: f32,
	#[diesel(sql_type = Text)]
	event_id: String,
}

#[allow(dead_code)]
#[derive(QueryableByName, Debug)]
struct InsertedDataDiscovery {
	#[diesel(sql_type = Text)]
	query_embedding: String,
	#[diesel(sql_type = Double)]
	score: f64,
	#[diesel(sql_type = Text)]
	doc_id: String,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::start_postgres_embedded;
	use std::path::PathBuf;

	#[tokio::test]
	async fn test_direct_insert_and_verify() {
		let db_path = PathBuf::from("/tmp/test_direct_insert_and_verify");
		println!("Initializing PGEmbed with database path: {:?}", db_path);
		let res = start_postgres_embedded(db_path.clone()).await;
		assert!(res.is_ok(), "Failed to start embedded postgres: {:?}", res);
		let (_postgres, database_url) = res.unwrap();
		let embed = PGEmbed::new(database_url).await.unwrap();
		println!("PGEmbed initialized successfully.");

		// Get a connection from the pool
		let conn = &mut embed.pool.get().await.unwrap();
		println!("Database connection established.");
		let payload = vec![
			(
				"doc_1".to_string(),
				"source_1".to_string(),
				None,
				VectorPayload {
					event_id: "event_1".to_string(),
					embeddings: vec![1.0_f32, 2.0_f32, 3.0_f32],
					score: 0.85_f32,
				},
			),
			(
				"doc_2".to_string(),
				"source_2".to_string(),
				None,
				VectorPayload {
					event_id: "event_2".to_string(),
					embeddings: vec![4.0_f32, 5.0_f32, 6.0_f32],
					score: 0.92_f32,
				},
			),
		];
		match embed.insert_vector("test_collection".to_string(), &payload).await {
			Ok(_) => println!("Vectors inserted successfully using `insert_vector`."),
			Err(e) => {
				eprintln!("Failed to insert vectors using `insert_vector`: {:?}", e);
				return;
			},
		}
		println!("Fetching row count from embedded_knowledge table.");
		let result: RowCount = match sql_query("SELECT COUNT(*) AS count FROM embedded_knowledge")
			.get_result(conn)
			.await
		{
			Ok(res) => {
				println!("Row count fetched successfully: {:?}", res);
				res
			},
			Err(e) => {
				eprintln!("Failed to get row count: {:?}", e);
				return;
			},
		};
		assert_eq!(result.count, 2, "Expected 2 rows to be inserted.");
		println!("Verified row count is correct: {}", result.count);
		println!("Fetching inserted vectors as strings from embedded_knowledge table.");
		let results: Vec<InsertedDataRaw> = match sql_query(
			"SELECT array_to_string(embeddings::real[], ',') as embeddings, score, event_id 
         FROM embedded_knowledge ORDER BY event_id",
		)
		.load(conn)
		.await
		{
			Ok(res) => {
				println!("Fetched results successfully: {:?}", res);
				res
			},
			Err(e) => {
				eprintln!("Failed to fetch inserted vectors: {:?}", e);
				return;
			},
		};
		let parsed_results: Vec<(Vec<f32>, f32, String)> = results
			.into_iter()
			.map(|row| {
				// Parse the embeddings string into a Vec<f32>
				let embeddings = row
					.embeddings
					.split(',') // Split by comma
					.filter_map(|s| s.trim().parse::<f32>().ok()) // Parse into f32
					.collect::<Vec<f32>>();

				(embeddings, row.score, row.event_id)
			})
			.collect();
		let expected_embeddings_1 = vec![1.0_f32, 2.0_f32, 3.0_f32];
		let expected_embeddings_2 = vec![4.0_f32, 5.0_f32, 6.0_f32];
		let expected_score_1 = 0.85_f32;
		let expected_score_2 = 0.92_f32;
		println!("Verifying first entry.----{:?}", parsed_results[0].0);
		assert_eq!(parsed_results[0].0, expected_embeddings_1, "Embeddings mismatch for event_1");
		assert_eq!(parsed_results[0].1, expected_score_1, "Score mismatch for event_1");
		assert_eq!(parsed_results[0].2, "event_1", "Event ID mismatch for event_1");

		println!("Verifying second entry.");
		assert_eq!(parsed_results[1].0, expected_embeddings_2, "Embeddings mismatch for event_2");
		assert_eq!(parsed_results[1].1, expected_score_2, "Score mismatch for event_2");
		assert_eq!(parsed_results[1].2, "event_2", "Event ID mismatch for event_2");
	}

	#[tokio::test]
	async fn test_similarity_search_l2() {
		// sleep for 2 seconds to allow the previous test to finish
		let db_path = PathBuf::from("/tmp/test_similarity_search_l2");
		println!("Initializing PGEmbed with database path: {:?}", db_path);
		let res = start_postgres_embedded(db_path.clone()).await;
		assert!(res.is_ok(), "Failed to start embedded postgres: {:?}", res);
		let (_postgres, database_url) = res.unwrap();
		let embed = PGEmbed::new(database_url).await.unwrap();
		println!("PGEmbed initialized successfully.");
		let conn = &mut embed.pool.get().await.unwrap();
		println!("Database connection established.");
		let insert_vectors = vec![
			VectorPayload {
				event_id: "event_1".to_string(),
				embeddings: vec![1.0, 2.0, 3.0],
				score: 0.85,
			},
			VectorPayload {
				event_id: "event_2".to_string(),
				embeddings: vec![4.0, 5.0, 6.0],
				score: 0.92,
			},
		];
		let payload = insert_vectors
			.into_iter()
			.map(|vp| ("doc_1".to_string(), "source_1".to_string(), None, vp))
			.collect::<Vec<_>>();
		match embed.insert_vector("test_collection".to_string(), &payload).await {
			Ok(_) => println!("Vectors inserted successfully."),
			Err(e) => {
				eprintln!("Failed to insert vectors: {:?}", e);
				return;
			},
		}
		let insert_semantics = vec![
			SemanticKnowledge {
				document_id: "doc_1".to_string(),
				subject: "subject_1".to_string(),
				object: "object_1".to_string(),
				document_source: "source_1".to_string(),
				sentence: "Sample sentence 1.".to_string(),
				event_id: "event_1".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				object_type: "".to_string(),
				source_id: "".to_string(),
				subject_type: "".to_string(),
			},
			SemanticKnowledge {
				document_id: "doc_2".to_string(),
				subject: "subject_2".to_string(),
				object: "object_2".to_string(),
				document_source: "source_2".to_string(),
				sentence: "Sample sentence 2.".to_string(),
				event_id: "event_2".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				object_type: "".to_string(),
				source_id: "".to_string(),
				subject_type: "".to_string(),
			},
		];
		for semantic in insert_semantics {
			sql_query(
            "INSERT INTO semantic_knowledge (document_id, subject, object, document_source, sentence, event_id) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind::<Text, _>(semantic.document_id)
        .bind::<Text, _>(semantic.subject)
        .bind::<Text, _>(semantic.object)
        .bind::<Text, _>(semantic.document_source)
        .bind::<Text, _>(semantic.sentence)
        .bind::<Text, _>(semantic.event_id)
        .execute(conn)
        .await
        .expect("Failed to insert semantic knowledge.");
		}
		let session_id = "test_session".to_string();
		let query = "test_query".to_string();
		let collection_id = "test_collection".to_string();
		let payload = vec![1.0, 2.0, 3.0];
		let max_results = 10;
		let offset = 0;
		// let top_pairs_embeddings = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
		let top_pairs_embeddings = vec![];
		let search_results = embed
			.similarity_search_l2(
				session_id.clone(),
				query.clone(),
				collection_id.clone(),
				&payload,
				max_results,
				offset,
				&top_pairs_embeddings,
			)
			.await;

		match search_results {
			Ok(results) => {
				println!("Search results: {:?}", results);
				assert!(!results.is_empty(), "Expected results but got none.");
				assert_eq!(results[0].subject, "subject_1");
				assert_eq!(results[0].object, "object_1");
				// assert_eq!(results[0].cosine_distance, Some(0.0));
			},
			Err(e) => {
				eprintln!("Similarity search failed: {:?}", e);
				assert!(false, "Similarity search should not have failed.");
			},
		}
	}

	#[tokio::test]
	async fn test_autogenerate_queries() {
		let db_path = PathBuf::from("/tmp/test_pgembed_autogenerate_queries");
		println!("Initializing PGEmbed with database path: {:?}", db_path);
		let res = start_postgres_embedded(db_path.clone()).await;
		assert!(res.is_ok(), "Failed to start embedded postgres: {:?}", res);
		let (_postgres, database_url) = res.unwrap();
		let embed = PGEmbed::new(database_url).await.unwrap();
		println!("PGEmbed initialized successfully.");
		let conn = &mut embed.pool.get().await.unwrap();

		let insert_semantics = vec![
			SemanticKnowledge {
				document_id: "doc_1".to_string(),
				subject: "subject_1".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_1".to_string(),
				object_type: "type_2".to_string(),
				event_id: "event_1".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				source_id: "".to_string(),
				sentence: "This is 1".to_string(),
				document_source: "".to_string(),
			},
			SemanticKnowledge {
				document_id: "doc_2".to_string(),
				subject: "subject_2".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_2".to_string(),
				object_type: "type_2".to_string(),
				event_id: "event_2".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				source_id: "".to_string(),
				sentence: "This is 2".to_string(),
				document_source: "".to_string(),
			},
			SemanticKnowledge {
				document_id: "doc_3".to_string(),
				subject: "subject_3".to_string(),
				subject_type: "type_3".to_string(),
				object: "object_3".to_string(),
				object_type: "type_4".to_string(),
				event_id: "event_3".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				source_id: "".to_string(),
				sentence: "This is 3".to_string(),
				document_source: "".to_string(),
			},
			SemanticKnowledge {
				document_id: "doc_4".to_string(),
				subject: "subject_4".to_string(),
				subject_type: "type_1".to_string(),
				object: "object_4".to_string(),
				object_type: "type_2".to_string(),
				event_id: "event_4".to_string(),
				collection_id: Some("".to_string()),
				image_id: Some("".to_string()),
				source_id: "".to_string(),
				sentence: "This is 4".to_string(),
				document_source: "".to_string(),
			},
		];
		for semantic in insert_semantics {
			sql_query(
            "INSERT INTO semantic_knowledge (document_id, subject, subject_type, object, object_type, event_id) 
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind::<Text, _>(semantic.document_id)
        .bind::<Text, _>(semantic.subject)
        .bind::<Text, _>(semantic.subject_type)
        .bind::<Text, _>(semantic.object)
        .bind::<Text, _>(semantic.object_type)
        .bind::<Text, _>(semantic.event_id)
        .execute(conn)
        .await
        .expect("Failed to insert semantic knowledge.");
		}
		let max_suggestions = 5;
		let query_suggestions = embed.autogenerate_queries(max_suggestions).await;

		match query_suggestions {
			Ok(suggestions) => {
				println!("Query Suggestions: {:?}", suggestions);
				assert!(!suggestions.is_empty(), "Expected suggestions but got none.");
				assert!(
					suggestions.len() <= max_suggestions as usize,
					"Number of suggestions exceeds max_suggestions."
				);
				if !suggestions.is_empty() {
					assert_eq!(suggestions[0].tags[0], "Filters", "Expected 'Filters' tag.");
				}
			},
			Err(e) => {
				eprintln!("Failed to generate query suggestions: {:?}", e);
				assert!(false, "Query suggestion generation should not have failed.");
			},
		}
	}
	#[tokio::test]
	async fn test_insert_discovered_knowledge() {
		let db_path = PathBuf::from("/tmp/test_insert_discovered_knowledge");
		println!("Initializing PGEmbed with database path: {:?}", db_path);
		let res = start_postgres_embedded(db_path.clone()).await;
		assert!(res.is_ok(), "Failed to start embedded postgres: {:?}", res);
		let (_postgres, database_url) = res.unwrap();
		let embed = PGEmbed::new(database_url).await.unwrap();
		println!("PGEmbed initialized successfully.");
		println!("Database connection established.");
		let payload = vec![
			DocumentPayload {
				doc_id: "test_doc_1".to_string(),
				doc_source: "test_source".to_string(),
				sentence: "This is a test sentence.".to_string(),
				knowledge: "test knowledge".to_string(),
				subject: "test_subject".to_string(),
				object: "test_object".to_string(),
				cosine_distance: Some(0.5),
				query_embedding: Some(vec![0.1, 0.2, 0.3, 0.4]),
				query: Some("test query".to_string()),
				session_id: Some("session_1".to_string()),
				score: 0.8,
				collection_id: "collection_1".to_string(),
			},
			DocumentPayload {
				doc_id: "test_doc_2".to_string(),
				doc_source: "test_source_2".to_string(),
				sentence: "Another test sentence.".to_string(),
				knowledge: "more test knowledge".to_string(),
				subject: "test_subject_2".to_string(),
				object: "test_object_2".to_string(),
				cosine_distance: Some(0.6),
				query_embedding: Some(vec![0.5, 0.6, 0.7, 0.8]),
				query: Some("another test query".to_string()),
				session_id: Some("session_2".to_string()),
				score: 0.9,
				collection_id: "collection_2".to_string(),
			},
		];
		match embed.insert_discovered_knowledge(&payload).await {
			Ok(_) => println!("Data inserted successfully using `insert_discovered_knowledge`."),
			Err(e) => {
				eprintln!("Failed to insert data using `insert_discovered_knowledge`: {:?}", e);
				return;
			},
		}
		let session_id = "session_1".to_string();
		match embed.get_discovered_data(session_id.clone()).await {
			Ok(fetched_data) => {
				println!(
					"Fetched data successfully using `get_discovered_data`: {:?}",
					fetched_data
				);
			},
			Err(e) => {
				eprintln!("Failed to fetch data using `get_discovered_data`: {:?}", e);
				return;
			},
		}
	}
}
