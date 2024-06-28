use async_trait::async_trait;
use common::{ DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{
	result::{ConnectionError, ConnectionResult},
	ExpressionMethods, QueryDsl,
};

use crate::DieselError;

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager, ManagerConfig},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};
use futures_util::{future::BoxFuture, FutureExt};
use proto::semantics::PostgresConfig;
use std::{sync::Arc, time::SystemTime};
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
use rustls::{
	client::{ServerCertVerified, ServerCertVerifier},
	ServerName,
};

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = embedded_knowledge)]

pub struct EmbeddedKnowledge {
	pub embeddings: Option<Vector>,
	pub score: f32,
	pub event_id: String,
}

// #[derive(Debug, Clone, Copy, FromSqlRow, AsExpression)]
// #[diesel(sql_type = BigInt)]
// pub struct EventId(pub u64);

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
		}
	}
}

pub struct PGVector {
	pub pool: ActualDbPool,
	pub config: PostgresConfig,
}

struct NoCertVerifier {}

impl ServerCertVerifier for NoCertVerifier {
	fn verify_server_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_server_name: &ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: SystemTime,
	) -> Result<ServerCertVerified, rustls::Error> {
		// Will verify all (even invalid) certs without any checks (sslmode=require)
		Ok(ServerCertVerified::assertion())
	}
}

impl PGVector {
	pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
		let mut d_config = ManagerConfig::default();
		let tls_enabled = config.url.contains("sslmode=require");
		let manager = if tls_enabled {
			// diesel-async does not support any TLS connections out of the box, so we need to manually
			// provide a setup function which handles creating the connection
			d_config.custom_setup = Box::new(establish_connection);
			AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
				config.url.clone(),
				d_config,
			)
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

fn establish_connection(config: &str) -> BoxFuture<ConnectionResult<AsyncPgConnection>> {
	let fut = async {
		let rustls_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(Arc::new(NoCertVerifier {}))
			.with_no_client_auth();

		let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
		let (client, conn) = tokio_postgres::connect(config, tls)
			.await
			.map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
		tokio::spawn(async move {
			if let Err(e) = conn.await {
				error!("Database connection failed: {e}");
			}
		});
		AsyncPgConnection::try_from(client).await
	};
	fut.boxed()
}

#[async_trait]
impl Storage for PGVector {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	// async fn get_discovered_data(
	// 	&self,
	// 	session_id: String,
	// ) -> StorageResult<Vec<DiscoveredKnowledgePayload>> {
	// 	// Ok(vec![])
	// 	let conn = &mut self.pool.get().await.map_err(|e| StorageError {
	// 		kind: StorageErrorKind::Internal,
	// 		source: Arc::new(anyhow::Error::from(e)),
	// 	})?;
	// 	let results = conn.transaction::<_, diesel::result::Error, _>(|conn| {
    //         async move {
    //             let query_result = discovered_knowledge::dsl::discovered_knowledge
    //                 .select((
    //                     discovered_knowledge::dsl::doc_id,
    //                     discovered_knowledge::dsl::doc_source,
    //                     discovered_knowledge::dsl::sentence,
    //                     discovered_knowledge::dsl::subject,
    //                     discovered_knowledge::dsl::object,
    //                     discovered_knowledge::dsl::cosine_distance,
    //                     discovered_knowledge::dsl::session_id,
    //                     discovered_knowledge::dsl::score,
    //                 ))
    //                 .filter(discovered_knowledge::dsl::session_id.eq(&session_id))
    //                 .load::<(
    //                     String,
    //                     String,
    //                     String,
    //                     String,
    //                     String,
    //                     Option<f64>,
    //                     Option<String>,
    //                     Option<f64>,
    //                 )>(conn)
    //                 .await;
                
    //             match query_result {
    //                 Ok(result) => {
    //                     let mut results: Vec<DiscoveredKnowledgePayload> = Vec::new();
    //                     for (doc_id, doc_source, sentence, subject, object, cosine_distance, session_id, score) in result {
    //                         let doc_payload = DiscoveredKnowledgePayload {
    //                             doc_id,
    //                             doc_source,
    //                             sentence,
    //                             subject,
    //                             object,
    //                             cosine_distance,
    //                             session_id: session_id,
    //                             score,
    //                         };
    //                         results.push(doc_payload);
    //                     }
    //                     Ok(results)
    //                 }
    //                 Err(e) => {
    //                     eprintln!("Error querying semantic data: {:?}", e);
    //                     Err(e)
    //                 }
    //             }
    //         }
    //         .scope_boxed()
    //     })
    //     .await
    //     .map_err(|e| StorageError {
    //         kind: StorageErrorKind::Internal,
    //         source: Arc::new(anyhow::Error::from(e)),
    //     })?;
        
    //     Ok(results)
    // }

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

	async fn similarity_search_l2(
		&self,
		session_id: String,
		query: String,
		_collection_id: String,
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
			.filter(embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()).le(0.6))
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
								results.push(doc_payload);
							}
						},
						Err(e) => {
							eprintln!("Error querying semantic data: {:?}", e);
							// Handle the error appropriately, for example by returning an error result
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
		let mut combined_results: Vec<(i32, String, String, String, String, String, String, f32)> = Vec::new();
		
		for (head, tail) in filtered_pairs {
			let conn = &mut self.pool.get().await.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			
			// Fetch inward edges where object is the head entity
			let inward_query_result = semantic_knowledge::dsl::semantic_knowledge
				.select((
					semantic_knowledge::dsl::id,
					semantic_knowledge::dsl::document_id,
					semantic_knowledge::dsl::subject,
					semantic_knowledge::dsl::object,
					semantic_knowledge::dsl::document_source,
					semantic_knowledge::dsl::sentence,
					semantic_knowledge::dsl::event_id,
				))
				.filter(semantic_knowledge::dsl::object.eq(&head))
				.load::<(i32, String, String, String, String, String, String)>(conn)
				.await;
				
			match inward_query_result {
				Ok(results) => {
					for result in results {
						let (id, doc_id, subject, object, doc_source, sentence, event_id) = result;
						// Query the embedded_knowledge table to get the score for the event_id
						let score_query_result = embedded_knowledge::dsl::embedded_knowledge
							.select(embedded_knowledge::dsl::score)
							.filter(embedded_knowledge::dsl::event_id.eq(&event_id))
							.first::<f32>(conn)
							.await;
							
						match score_query_result {
							Ok(score) => {
								combined_results.push((id, doc_id, subject, object, doc_source, sentence, event_id, score));
							}
							Err(e) => {
								eprintln!("Error querying score for event_id {}: {:?}", event_id, e);
								// return Err(StorageError {
								// 	kind: StorageErrorKind::Query,
								// 	source: Arc::new(anyhow::Error::from(e)),
								// });
							}
						}
					}
				}
				Err(e) => {
					eprintln!("Error querying inward edges for head entity {}: {:?}", head, e);
					return Err(StorageError {
						kind: StorageErrorKind::Query,
						source: Arc::new(anyhow::Error::from(e)),
					});
				}
			}
	
			// Fetch outward edges where subject is the tail entity
			let outward_query_result = semantic_knowledge::dsl::semantic_knowledge
				.select((
					semantic_knowledge::dsl::id,
					semantic_knowledge::dsl::document_id,
					semantic_knowledge::dsl::subject,
					semantic_knowledge::dsl::object,
					semantic_knowledge::dsl::document_source,
					semantic_knowledge::dsl::sentence,
					semantic_knowledge::dsl::event_id,
				))
				.filter(semantic_knowledge::dsl::subject.eq(&tail))
				.load::<(i32, String, String, String, String, String, String)>(conn)
				.await;
				
			match outward_query_result {
				Ok(results) => {
					for result in results {
						let (id, doc_id, subject, object, doc_source, sentence, event_id) = result;
						// Query the embedded_knowledge table to get the score for the event_id
						let score_query_result = embedded_knowledge::dsl::embedded_knowledge
							.select(embedded_knowledge::dsl::score)
							.filter(embedded_knowledge::dsl::event_id.eq(&event_id))
							.first::<f32>(conn)
							.await;
							
						match score_query_result {
							Ok(score) => {
								combined_results.push((id, doc_id, subject, object, doc_source, sentence, event_id, score));
							}
							Err(e) => {
								eprintln!("Error querying score for event_id {}: {:?}", event_id, e);
								// return Err(StorageError {
								// 	kind: StorageErrorKind::Query,
								// 	source: Arc::new(anyhow::Error::from(e)),
								// });
							}
						}
					}
				}
				Err(e) => {
					eprintln!("Error querying outward edges for tail entity {}: {:?}", tail, e);
					return Err(StorageError {
						kind: StorageErrorKind::Query,
						source: Arc::new(anyhow::Error::from(e)),
					});
				}
			}
		}
	
		Ok(combined_results)
	}
	
	// async fn get_semantic_metadata_by_ids(
	// 	&self,
	// 	session_id: String,
	// 	ids: Vec<i32>,
	// ) -> StorageResult<Vec<DiscoveredKnowledgePayload>>{
	// 	let conn = &mut self.pool.get().await.map_err(|e| StorageError {
    //         kind: StorageErrorKind::Internal,
    //         source: Arc::new(anyhow::Error::from(e)),
    //     })?;


	// 	let query_result = semantic_knowledge::dsl::semantic_knowledge
	// 			.select((
	// 				semantic_knowledge::dsl::id,
	// 				semantic_knowledge::dsl::document_id,
	// 				semantic_knowledge::dsl::subject,
	// 				semantic_knowledge::dsl::subject_type,
	// 				semantic_knowledge::dsl::object,
	// 				semantic_knowledge::dsl::object_type,
	// 				semantic_knowledge::dsl::document_source,
	// 				semantic_knowledge::dsl::sentence,
	// 				semantic_knowledge::dsl::event_id,
	// 			))
	// 			.filter(semantic_knowledge::dsl::id.eq_any(&ids))
	// 			.load::<(i32, String, String, String, String, String, String, String, String)>(conn)
	// 			.await;
	// 		match query_result {
	// 			Ok(results) => {
	// 				let mut discovered_knowledge = Vec::new();
	// 				for (id, doc_id, subject, subject_type, object, object_type, doc_source, sentence, event_id) in results {
	// 					// Query the embedded_knowledge table to get the score for the event_id
	// 					let score_query_result = embedded_knowledge::dsl::embedded_knowledge
	// 						.select(embedded_knowledge::dsl::score)
	// 						.filter(embedded_knowledge::dsl::event_id.eq(&event_id))
	// 						.first::<Option<f32>>(conn)
	// 						.await;
	
	// 					match score_query_result {
	// 						Ok(Some(score)) => {
	// 							discovered_knowledge.push(DiscoveredKnowledgePayload {
	// 								doc_id,
	// 								subject,
	// 								object,
	// 								doc_source,
	// 								sentence,
	// 								cosine_distance: Some(0.0),
	// 								session_id: Some(session_id),
	// 								score: Some(score as f64),
	// 							});
	// 						}
	// 						Err(e) => {
	// 							eprintln!("Error querying score for event_id {}: {:?}", event_id, e);
	// 							return Err(StorageError {
	// 								kind: StorageErrorKind::Query,
	// 								source: Arc::new(anyhow::Error::from(e)),
	// 							});
	// 						}
	// 					}
	// 				}
	// 				Ok(discovered_knowledge)
	// 			}
	// 			Err(e) => {
	// 				eprintln!("Error querying semantic knowledge by ids: {:?}", e);
	// 				Err(StorageError {
	// 					kind: StorageErrorKind::Query,
	// 					source: Arc::new(anyhow::Error::from(e)),
	// 				})
	// 			}
	// 		}
	// 	}

	// async fn get_semantic_metadata_by_ids(
	// 	&self,
	// 	_session_id: String,
	// 	_ids: Vec<i32>,
	// ) -> StorageResult<Vec<DiscoveredKnowledgePayload>>{
	// 	Ok(vec![])
	// }

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
	async fn store_kv(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_kv(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
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
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::utils::{get_top_k_pairs, process_traverser_results};
	use proto::semantics::StorageType;
	use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
	const TEST_DB_URL: &str = "postgres://querent:querent@localhost/querent_test?sslmode=prefer";

	async fn setup_test_environment() -> PGVector {
		// Create a PGVector instance with the test database URL
		let config = PostgresConfig {
			url: TEST_DB_URL.to_string(),
			name: "test".to_string(),
			storage_type: Some(StorageType::Index),
		};

		PGVector::new(config).await.expect("Failed to create Postgres Stroage instance")
	}

	// Test function
	#[tokio::test]

	async fn test_traverser_search() {
		let mut current_query_embedding: Vec<f32> = Vec::new();
		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		});
		match embedding_model {
			Ok(embedder) => {
				let query = "What is the fluid type in eagle ford shale?".to_string();
				match embedder.embed(vec![query.clone()], None) {
					Ok(embeddings) => {
						current_query_embedding = embeddings[0].clone();
						// Use current_query_embedding as needed
					}
					Err(e) => {
						eprintln!("Error embedding query: {}", e);
						// Handle the error appropriately
					}
				}
			}
			Err(e) => {
				eprintln!("Error initializing embedding model: {}", e);
				// Handle the error appropriately
			}
		}
		// self.embedding_model = Some(embedding_model);
		let storage = setup_test_environment().await;
		let results = storage
		.similarity_search_l2("mock".to_string(), "mock".to_string(), "mock".to_string(), &current_query_embedding, 10, 0)
		.await;
		
		// println!("Results are -------{:?}", results);
	// 	let results = storage
	// 		.get_discovered_data("12ac36ef40294d87bbbfbe719caa5f70".to_string())
	// 		.await;
	// 	// println!("Results: {:?}", results);
	// 	assert!(results.is_ok());
	let filtered_results = get_top_k_pairs(results.unwrap(), 10);
	println!("Filtered Results --------{:?}", filtered_results);
	let traverser_results_1 = storage.traverse_metadata_table(filtered_results).await;
	match traverser_results_1 {
		Ok(results) => {
			// println!("---------------------Traverser_1 Results --------{:?}", results);
			let formatted_output = process_traverser_results(results);
			println!("--------------------------------------------");
			for line in formatted_output {
				println!("{}", line);
}
		}
		Err(e) => {
			eprintln!("Error fetching semantic data: {:?}", e);
		}
	}

		
		// let documents = results.unwrap();
		// assert!(!documents.is_empty(), "The search should return at least one result.");
	}


	#[tokio::test]
	async fn test_postgres_storage() {
		// Create a postgres config
		let config = PostgresConfig {
			url: TEST_DB_URL.to_string(),
			name: "test".to_string(),
			storage_type: Some(StorageType::Index),
		};

		// Create a PostgresStorage instance with the test database URL
		let storage_result = PGVector::new(config).await;

		// Ensure that the storage is created successfully
		assert!(storage_result.is_ok());

		// Get the storage instance from the result
		let storage = storage_result.unwrap();

		// Perform a connectivity check
		let _connectivity_result = storage.check_connectivity().await;
		// Ensure that the connectivity check is successful
		// Works when there is a database running on the test database URL
		//assert!(_connectivity_result.is_ok());

		// You can add more test cases or assertions based on your specific requirements
	}
}