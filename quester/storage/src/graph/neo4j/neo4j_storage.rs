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

	async fn insert_vector(&self, _payload: Vec<(String, VectorPayload)>) -> StorageResult<()> {
		// Implement Neo4j vector insertion logic (if needed)
		Ok(())
	}

	async fn insert_graph(
		&self,
		payload: Vec<(String, SemanticKnowledgePayload)>,
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
				("subject_type", data.subject_type),
				("predicate_type", data.predicate_type),
				("object_type", data.object_type),
				("subject", data.subject),
				("predicate", data.predicate),
				("object", data.object),
				("sentence", data.sentence),
			];

			let tx_res = txn.execute(Query::new(cypher_query).params(params)).await;
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
