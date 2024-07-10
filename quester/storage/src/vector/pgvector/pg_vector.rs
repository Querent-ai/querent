use crate::{utils::traverse_node, DieselError};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};
use proto::{
	discovery::DiscoverySessionRequest,
	insights::InsightAnalystRequest,
	semantics::{PostgresConfig, SemanticPipelineRequest},
};
use std::{collections::HashSet, sync::Arc};
use tracing::error;

use crate::{
	semantic_knowledge, ActualDbPool, Storage, StorageError, StorageErrorKind, StorageResult,
	POOL_TIMEOUT,
};
use pgvector::Vector;

use deadpool::Runtime;
use diesel::{table, Insertable, Queryable, QueryableByName, Selectable};
use diesel_async::AsyncConnection;
use pgvector::VectorExpressionMethods;

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = embedded_knowledge)]

pub struct EmbeddedKnowledge {
	pub embeddings: Option<Vector>,
	pub score: f32,
	pub event_id: String,
}

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = discovered_knowledge)]
pub struct DiscoveredKnowledge {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub subject: String,
	pub object: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vector>,
	pub query: Option<String>,
	pub session_id: Option<String>,
	pub score: Option<f64>,
	pub collection_id: String,
}

impl DiscoveredKnowledge {
	pub fn from_document_payload(payload: DocumentPayload) -> Self {
		DiscoveredKnowledge {
			doc_id: payload.doc_id,
			doc_source: payload.doc_source,
			sentence: payload.sentence,
			subject: payload.subject,
			object: payload.object,
			cosine_distance: payload.cosine_distance,
			query_embedding: Some(Vector::from(payload.query_embedding.unwrap_or_default())),
			query: payload.query,
			session_id: payload.session_id,
			score: Some(payload.score as f64),
			collection_id: payload.collection_id,
		}
	}
}

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
impl Storage for PGVector {
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

	async fn get_discovered_data(
		&self,
		session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		// Ok(vec![])
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let results = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                let query_result = discovered_knowledge::dsl::discovered_knowledge
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
                    .filter(discovered_knowledge::dsl::session_id.eq(&session_id))
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
                        for (doc_id, doc_source, sentence, subject, object, cosine_distance, session_id, score, query, query_embedding, collection_id) in result {
                            let doc_payload = DiscoveredKnowledge {
                                doc_id,
                                doc_source,
                                sentence,
                                subject,
                                object,
                                cosine_distance,
                                session_id: session_id,
                                score,
								query,
								query_embedding,
								collection_id
                            };
                            results.push(doc_payload);
                        }
                        Ok(results)
                    }
                    Err(e) => {
                        eprintln!("Error querying semantic data: {:?}", e);
                        Err(e)
                    }
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

	async fn similarity_search_l2(
		&self,
		session_id: String,
		query: String,
		collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let vector = Vector::from(payload.clone());
		let query_result = embedded_knowledge::dsl::embedded_knowledge
			.select((
				embedded_knowledge::dsl::embeddings,
				embedded_knowledge::dsl::score,
				embedded_knowledge::dsl::event_id,
				embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()),
			))
			.filter(embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()).le(0.2))
			.order_by(embedded_knowledge::dsl::embeddings.cosine_distance(vector))
			.limit(max_results as i64)
			.offset(offset)
			.load::<(Option<Vector>, f32, String, Option<f64>)>(&mut conn)
			.await;
		match query_result {
			Ok(result) => {
				let mut results: Vec<DocumentPayload> = Vec::new();
				for (_embeddings, score, event_id, other_cosine_distance) in result {
					// Query to semantic knowledge table using uuid
					// let other_cosine_distance_1: f64 = other_cosine_distance.unwrap_or(0.0); // Use a default value if None
					let query_result_semantic = semantic_knowledge::dsl::semantic_knowledge
						.select((
							semantic_knowledge::dsl::document_id,
							semantic_knowledge::dsl::subject,
							semantic_knowledge::dsl::object,
							semantic_knowledge::dsl::document_source,
							semantic_knowledge::dsl::sentence,
						))
						.filter(semantic_knowledge::dsl::event_id.eq(event_id))
						.offset(offset)
						.load::<(String, String, String, String, String)>(&mut conn)
						.await;

					match query_result_semantic {
						Ok(result_semantic) => {
							for (doc_id, subject, object, document_store, sentence) in
								result_semantic
							{
								let mut doc_payload = DocumentPayload::default();
								doc_payload.doc_id = doc_id.clone();
								doc_payload.subject = subject.clone();
								doc_payload.object = object.clone();
								doc_payload.doc_source = document_store.clone();
								doc_payload.cosine_distance = other_cosine_distance;
								doc_payload.score = score;
								doc_payload.sentence = sentence.clone();
								doc_payload.session_id = Some(session_id.clone());
								doc_payload.query_embedding = Some(payload.clone());
								doc_payload.query = Some(query.clone());
								doc_payload.collection_id = collection_id.clone();
								results.push(doc_payload);
							}
						},
						Err(e) => {
							error!("Error querying semantic data: {:?}", e);
							return Err(StorageError {
								kind: StorageErrorKind::Query,
								source: Arc::new(anyhow::Error::from(e)),
							});
						},
					}
				}
				Ok(results)
			},
			Err(err) => {
				log::error!("Query failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}

	async fn traverse_metadata_table(
		&self,
		filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>> {
		let mut combined_results: Vec<(i32, String, String, String, String, String, String, f32)> =
			Vec::new();
		let mut visited_pairs: HashSet<(String, String)> = HashSet::new();

		for (head, tail) in filtered_pairs {
			let conn = &mut self.pool.get().await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

			// Traverse depth 1
			traverse_node(
				&self.pool,
				head.clone(),
				&mut combined_results,
				&mut visited_pairs,
				conn,
				1,
			)
			.await?;
			traverse_node(
				&self.pool,
				tail.clone(),
				&mut combined_results,
				&mut visited_pairs,
				conn,
				1,
			)
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
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
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
	async fn get_all_pipelines(&self) -> StorageResult<Vec<SemanticPipelineRequest>> {
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
	async fn get_all_discovery_sessions(&self) -> StorageResult<Vec<DiscoverySessionRequest>> {
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
	async fn get_all_insight_sessions(&self) -> StorageResult<Vec<InsightAnalystRequest>> {
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

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	embedded_knowledge (id) {
		id -> Int4,
		embeddings -> Nullable<Vector>,
		score -> Float4,
		event_id -> VarChar,
	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	discovered_knowledge (id) {
		id -> Int4,
		doc_id -> Varchar,
		doc_source -> Varchar,
		sentence -> Text,
		subject -> Text,
		object -> Text,
		cosine_distance -> Nullable<Float8>,
		query_embedding -> Nullable<Vector>,
		query -> Nullable<Text>,
		session_id -> Nullable<Text>,
		score -> Nullable<Float8>,
		collection_id -> Text,
	}
}

// #[cfg(test)]
// mod test {
// 	use super::*;
// 	use crate::utils::{
// 		extract_unique_pairs, find_intersection, get_top_k_pairs, process_traverser_results
// 	};
// 	use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
// 	use proto::semantics::StorageType;
// 	const TEST_DB_URL: &str = "postgres://querent:querent@localhost/querent_test?sslmode=prefer";

// 	async fn setup_test_environment() -> PGVector {
// 		// Create a PGVector instance with the test database URL
// 		let config = PostgresConfig {
// 			url: TEST_DB_URL.to_string(),
// 			name: "test".to_string(),
// 			storage_type: Some(StorageType::Index),
// 		};

// 		PGVector::new(config).await.expect("Failed to create Postgres Stroage instance")
// 	}

// 	// Test function
// 	#[tokio::test]

// 	async fn test_traverser_search() {
// 		let mut current_query_embedding: Vec<f32> = Vec::new();
// 		let embedding_model = TextEmbedding::try_new(InitOptions {
// 			model_name: EmbeddingModel::AllMiniLML6V2,
// 			show_download_progress: true,
// 			..Default::default()
// 		});
// 		let embedder = match embedding_model {
// 			Ok(embedder) => embedder,
// 			Err(e) => {
// 				eprintln!("Error initializing embedding model: {}", e);
// 				return; // Exit the function if the model initialization fails
// 			},
// 		};

// 		let query = "What is the fluid type in eagle ford shale?".to_string();
// 		match embedder.embed(vec![query.clone()], None) {
// 			Ok(embeddings) => {
// 				current_query_embedding = embeddings[0].clone();
// 				// Use current_query_embedding as needed
// 			},
// 			Err(e) => {
// 				eprintln!("Error embedding query: {}", e);
// 				return; // Exit the function if the embedding fails
// 			},
// 		}

// 		let storage = setup_test_environment().await;
// 		let results = storage
// 			.similarity_search_l2(
// 				"mock".to_string(),
// 				"mock".to_string(),
// 				"mock".to_string(),
// 				&current_query_embedding,
// 				2,
// 				0,
// 			)
// 			.await;

// 		println!("Results are -------{:?}", results);

// 		let filtered_results = get_top_k_pairs(results.unwrap(), 2);
// 		println!("Filtered Results --------{:?}", filtered_results);
// 		let traverser_results_1 = storage.traverse_metadata_table(filtered_results.clone()).await;
// 		let formatted_output_1 = match traverser_results_1 {
// 			Ok(results) => extract_unique_pairs(results, filtered_results),
// 			Err(e) => {
// 				eprintln!("Error during traversal: {:?}", e);
// 				filtered_results // or handle the error as needed
// 			},
// 		};

// 		println!("Traverser for 1st query final pairs {:?}", formatted_output_1);
// 		// match traverser_results_1 {
// 		// 	Ok(results) => {
// 		// 		println!("---------------------Traverser_1 Results --------{:?}", results);
// 		// 		// let formatted_output = process_traverser_results(results);
// 		// 		println!("--------------------------------------------");
// 		// 			for line in formatted_output {
// 		// 				println!("{}", line);
// 		// }
// 		// }
// 		// Err(e) => {
// 		// 	eprintln!("Error fetching semantic data: {:?}", e);
// 		// }
// 		// }

// 		// Now user sees the output and gives 2nd query

// 		// Example of embedding a second query

// 		let second_query = "What is cyclic injection ?".to_string();
// 		let second_query_embedding = match embedder.embed(vec![second_query.clone()], None) {
// 			Ok(embeddings) => embeddings[0].clone(),
// 			Err(e) => {
// 				eprintln!("Error embedding second query: {}", e);
// 				return; // Exit the function if the embedding fails
// 			},
// 		};

// 		// Perform operations with the second query embedding
// 		let second_results = storage
// 			.similarity_search_l2(
// 				"mock".to_string(),
// 				"mock".to_string(),
// 				"mock".to_string(),
// 				&second_query_embedding,
// 				1,
// 				0,
// 			)
// 			.await;

// 		let second_filtered_results = get_top_k_pairs(second_results.unwrap(), 2);
// 		println!("Second Filtered Results --------{:?}", second_filtered_results);
// 		let traverser_results_2 =
// 			storage.traverse_metadata_table(second_filtered_results.clone()).await;
// 		let formatted_output_2 = match traverser_results_2 {
// 			Ok(results) => extract_unique_pairs(results, second_filtered_results),
// 			Err(e) => {
// 				eprintln!("Error during traversal: {:?}", e);
// 				second_filtered_results // or handle the error as needed
// 			},
// 		};

// 		println!("Traverser for 2nd query final pairs {:?}", formatted_output_2);

// 		let results_intersection = find_intersection(formatted_output_1, formatted_output_2);

// 		println!("Results Intersection ---{:?}", results_intersection);
// 	}

// 	#[tokio::test]
// 	async fn test_postgres_storage() {
// 		// Create a postgres config
// 		let config = PostgresConfig {
// 			url: TEST_DB_URL.to_string(),
// 			name: "test".to_string(),
// 			storage_type: Some(StorageType::Index),
// 		};

// 		// Create a PostgresStorage instance with the test database URL
// 		let storage_result = PGVector::new(config).await;

// 		// Ensure that the storage is created successfully
// 		assert!(storage_result.is_ok());

// 		// Get the storage instance from the result
// 		let storage = storage_result.unwrap();

// 		// Perform a connectivity check
// 		let _connectivity_result = storage.check_connectivity().await;
// 		// Ensure that the connectivity check is successful
// 		// Works when there is a database running on the test database URL
// 		//assert!(_connectivity_result.is_ok());

// 		// You can add more test cases or assertions based on your specific requirements
// 	}
// }
