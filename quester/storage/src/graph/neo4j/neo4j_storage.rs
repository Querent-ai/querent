use crate::{storage::Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use common::{SemanticKnowledgePayload, VectorPayload};
use neo4rs::*;
use std::sync::Arc;

pub struct Neo4jStorage {
	pub graph: Arc<Graph>,
	pub config: Config,
}

impl Neo4jStorage {
	pub async fn new(
		uri: String,
		user: String,
		pass: String,
		db: String,
		max_connections: usize,
		fetch_size: usize,
	) -> StorageResult<Self> {
		let config = ConfigBuilder::default()
			.uri(uri.clone())
			.user(user.clone())
			.password(pass.clone())
			.db(db.clone())
			.fetch_size(fetch_size)
			.max_connections(max_connections)
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

	async fn insert_vector(&self, _payload: &Vec<(String, VectorPayload)>) -> StorageResult<()> {
		// Implement Neo4j vector insertion logic (if needed)
		Ok(())
	}

	/// Index knowledge for search
	async fn index_knowledge(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn insert_graph(
		&self,
		payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		let mut txn = self.graph.start_txn().await.map_err(|err| {
			log::error!("Neo4j transaction creation failed: {:?}", err);
			StorageError {
				kind: StorageErrorKind::Database,
				source: Arc::new(anyhow::Error::from(err)),
			}
		})?;
		for (_id, data) in payload {
			let cypher_query = data.to_cypher_query();
			let params = vec![
				("entity_type1", data.subject_type.clone()),
				("predicate_type", data.predicate_type.clone()),
				("entity1", data.subject.clone()),
				("predicate", data.predicate.clone()),
				("entity_type2", data.object_type.clone()),
				("entity2", data.object.clone()),
				("sentence", data.sentence.clone()),
				("document_id", _id.clone()),
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_insert_graph() {
		// Provide your Neo4j connection details here
		let uri = "neo4j://localhost:7687";
		let user = "neo4j";
		let pass = "password_neo";
		let db = "neo4j";
		let max_connections = 5;
		let fetch_size = 100;

		// Create a Neo4jStorage instance for testing
		let storage = Neo4jStorage::new(
			uri.to_string(),
			user.to_string(),
			pass.to_string(),
			db.to_string(),
			max_connections,
			fetch_size,
		)
		.await;
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
