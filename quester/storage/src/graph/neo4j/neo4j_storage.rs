use crate::{storage::Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use neo4rs::*;
use proto::semantics::Neo4jConfig;
use std::sync::Arc;

pub struct Neo4jStorage {
	pub graph: Arc<Graph>,
	pub config: Config,
}

impl Neo4jStorage {
	pub async fn new(config: Neo4jConfig) -> StorageResult<Self> {
		let config = ConfigBuilder::default()
			.uri(config.url.clone())
			.user(config.username.clone())
			.password(config.password.clone())
			.db(config.db_name.clone())
			.fetch_size(config.fetch_size as usize)
			.max_connections(config.max_connection_pool_size as usize)
			.build()
			.map_err(|err| {
				log::error!("Neo4j client creation failed: {:?}", err);
				StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				}
			})?;

		let graph = Graph::connect(config.clone()).await;
		match graph {
			Ok(graph) => Ok(Neo4jStorage { graph: Arc::new(graph), config }),
			Err(err) => {
				log::error!("Neo4j client creation failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}
}

#[async_trait]
impl Storage for Neo4jStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		// You can perform a simple query to check connectivity
		let cypher_query = "RETURN 1";
		let _ = self.graph.execute(Query::new(cypher_query.to_string())).await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		// Implement Neo4j vector insertion logic (if needed)
		Ok(())
	}

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
		Ok(())
	}

	/// Index knowledge for search
	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn traverse_metadata_table(
		&self,
		_filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
	}

	async fn similarity_search_l2(
		&self,
		_session_id: String,
		_query: String,
		_collection_id: String,
		_payload: &Vec<f32>,
		_max_results: i32,
		_offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		// Implement Neo4j similarity search logic (if needed)
		Ok(vec![])
	}

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(&self, _pipeline: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get semantic pipeline by id
	async fn get_pipeline(&self, _pipeline_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, _pipeline_id: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Discovery session ran by this node
	async fn set_discovery_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Discovery session by id
	async fn get_discovery_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Insight session by id
	async fn get_insight_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	async fn insert_graph(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		let mut txn = self.graph.start_txn().await.map_err(|err| {
			log::error!("Neo4j transaction creation failed: {:?}", err);
			StorageError {
				kind: StorageErrorKind::Database,
				source: Arc::new(anyhow::Error::from(err)),
			}
		})?;
		for (id, source, image_id, data) in payload {
			let cloned_image_id = image_id.clone();
			let image_id_res;
			match cloned_image_id {
				Some(id) => {
					image_id_res = id;
				},
				None => {
					image_id_res = "".to_string();
				},
			}
			let cypher_query = data.to_cypher_query();
			let params: Vec<(&str, String)> = vec![
				("entity_type1", data.subject_type.clone()),
				("predicate_type", data.predicate_type.clone()),
				("entity1", data.subject.clone()),
				("predicate", data.predicate.clone()),
				("entity_type2", data.object_type.clone()),
				("entity2", data.object.clone()),
				("sentence", data.sentence.clone()),
				("document_id", id.clone()),
				("document_source", source.clone()),
				("collection_id", _collection_id.clone()),
				("image_id", image_id_res.clone()),
			];

			let parameterized_query = Query::new(cypher_query).params(params);
			let tx_res = txn.execute(parameterized_query).await;
			match tx_res {
				Ok(_) => {
					log::debug!("Inserted graph data:");
				},
				Err(err) => {
					log::error!("Graph insertion failed: {:?}", err);
					return Err(StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(err)),
					});
				},
			}
		}
		let finalize_res = txn.commit().await;
		match finalize_res {
			Ok(_) => {
				log::debug!("Transaction committed");
			},
			Err(err) => {
				log::error!("Transaction commit failed: {:?}", err);
				return Err(StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				});
			},
		}
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
}

#[cfg(test)]
mod tests {
	use proto::semantics::StorageType;

	use super::*;

	#[tokio::test]
	async fn test_insert_graph() {
		// Provide your Neo4j connection details here
		let uri = "neo4j://localhost:7687";
		let user = "neo4j";
		let pass = "password_neo";
		let db = "neo4j";
		let max_connection_pool_size = 5;
		let fetch_size = 100;

		// Create config for testing
		let config = Neo4jConfig {
			storage_type: Some(StorageType::Graph),
			name: "neo4j".to_string(),
			url: uri.to_string(),
			username: user.to_string(),
			password: pass.to_string(),
			db_name: db.to_string(),
			max_connection_pool_size,
			fetch_size,
		};
		// Create a Neo4jStorage instance for testing
		let storage = Neo4jStorage::new(config).await;
		if let Err(err) = storage {
			log::error!("Neo4jStorage creation failed: {:?}", err);
			return;
		}
		//Uncomment the following lines if neo4j is running in a docker container
		// assert!(storage.is_ok(), "Neo4jStorage creation failed");

		// // Prepare test data
		// let payload = vec![
		// 	(
		// 		"1".to_string(),
		// 		SemanticKnowledgePayload {
		// 			subject_type: "person".to_string(),
		// 			subject: "alice".to_string(),
		// 			predicate_type: "knows".to_string(),
		// 			predicate: "likes".to_string(),
		// 			object_type: "person".to_string(),
		// 			object: "bob".to_string(),
		// 			sentence: "alice likes that bob".to_string(),
		//			collection_id: "test".to_string(),
		// 		},
		// 	),
		// 	// Add more test data as needed
		// ];
		// // Call the insert_graph function with the test data
		// let _result = storage.unwrap().insert_graph(payload).await;

		// //Assert that the result is Ok indicating successful insertion
		// assert!(_result.is_ok(), "Graph insertion failed: {:?}", _result);
	}
}
