use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use crate::{
	postgres_index::QuerySuggestion, DiscoveredKnowledge, Storage, StorageError, StorageErrorKind,
	StorageResult,
};
use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::{semantics::SemanticPipelineRequest, DiscoverySessionRequest, InsightAnalystRequest};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;

const TABLE_PIPELINES: TableDefinition<&str, &[u8]> = TableDefinition::new("querent_pipelines");
const TABLE_DISCOVERY_SESSIONS: TableDefinition<&str, &[u8]> =
	TableDefinition::new("querent_discovery_sessions");
const TABLE_INSIGHT_SESSIONS: TableDefinition<&str, &[u8]> =
	TableDefinition::new("querent_insight_sessions");

pub struct MetaStore {
	db: Arc<Database>,
}

impl Debug for MetaStore {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		writeln!(f, "ReDb")?;
		Ok(())
	}
}

impl MetaStore {
	pub fn new(dir_path: PathBuf) -> Self {
		if !dir_path.exists() {
			std::fs::create_dir_all(&dir_path)
				.expect("Failed to create directory to init key value store");
		}
		let db_path = dir_path.join("querent_metadata.redb");
		let db = Database::create(db_path).expect("Failed to init key value store");

		let write_txn = db.begin_write().unwrap();
		write_txn.open_table(TABLE_PIPELINES).unwrap();
		write_txn.open_table(TABLE_DISCOVERY_SESSIONS).unwrap();
		write_txn.open_table(TABLE_INSIGHT_SESSIONS).unwrap();
		write_txn.commit().unwrap();

		Self { db: Arc::new(db) }
	}
}

#[async_trait]
impl Storage for MetaStore {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		Ok(()) // Placeholder, add vector insertion logic here
	}

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}


	/// Retrieve Filetered Results when query is empty and semantic pair filters are provided
	async fn filter_and_query(
		&self,
		_session_id: &String,
		_top_pairs: &Vec<String>,
		_max_results: i32,
		_offset: i64,
	) -> StorageResult<Vec<DocumentPayload>>{
		Ok(vec![])
	}

	
	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		Ok(vec![])
	}

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
	) -> StorageResult<Vec<(String, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
	}

	/// Insert InsightKnowledge into storage
	async fn insert_insight_knowledge(
		&self,
		_query: Option<String>,
		_session_id: Option<String>,
		_response: Option<String>,
	) -> StorageResult<()> {
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

	async fn similarity_search_l2(
		&self,
		_session_id: String,
		_query: String,
		_collection_id: String,
		_payload: &Vec<f32>,
		_max_results: i32,
		_offset: i64,
		_top_pairs_embeddings: Vec<Vec<f32>>,
	) -> StorageResult<Vec<DocumentPayload>> {
		// Implement Neo4j similarity search logic (if needed)
		Ok(vec![])
	}

	/// Store key value pair
	async fn store_secret(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get value for key
	async fn get_secret(&self, _key: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	async fn get_all_secrets(&self) -> StorageResult<Vec<(String, String)>> {
		Ok(Vec::new())
	}

	async fn delete_secret(&self, _key: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<(String, SemanticPipelineRequest)>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let mut pipelines = Vec::new();
		{
			let table = read_txn.open_table(TABLE_PIPELINES).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			let iter = table.iter().map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;
			for result in iter {
				let (key_access_guard, value_access_guard) =
					result.map_err(|err| StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(err)),
					})?;
				let key = key_access_guard.value();
				let value = value_access_guard.value();
				let pipeline: SemanticPipelineRequest =
					bincode::deserialize(value).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?;
				pipelines.push((key.to_string(), pipeline));
			}
		}
		Ok(pipelines)
	}

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(
		&self,
		pipeline_id: &String,
		pipeline: SemanticPipelineRequest,
	) -> StorageResult<()> {
		let encoded_data = bincode::serialize(&pipeline).map_err(|e| StorageError {
			kind: StorageErrorKind::Serialization,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let write_txn = self.db.begin_write().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		{
			let mut table = write_txn.open_table(TABLE_PIPELINES).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			table.insert(pipeline_id.as_str(), encoded_data.as_slice()).map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;
		}
		write_txn.commit().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})
	}

	/// Get semantic pipeline by id
	async fn get_pipeline(
		&self,
		pipeline_id: &String,
	) -> StorageResult<Option<SemanticPipelineRequest>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let pipeline = {
			let table = read_txn.open_table(TABLE_PIPELINES).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			let value = table.get(pipeline_id.as_str()).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			match value {
				Some(value) =>
					Some(bincode::deserialize(value.value()).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?),
				None => None,
			}
		};
		Ok(pipeline)
	}

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, pipeline_id: &String) -> StorageResult<()> {
		let write_txn = self.db.begin_write().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		{
			let mut table = write_txn.open_table(TABLE_PIPELINES).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			table.remove(pipeline_id.as_str()).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
		}
		write_txn.commit().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})
	}

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(
		&self,
	) -> StorageResult<Vec<(String, DiscoverySessionRequest)>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let mut sessions = Vec::new();
		{
			let table =
				read_txn.open_table(TABLE_DISCOVERY_SESSIONS).map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			let iter = table.iter().map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;
			for result in iter {
				let (key_access_guard, value_access_guard) =
					result.map_err(|err| StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(err)),
					})?;
				let key = key_access_guard.value();
				let value = value_access_guard.value();
				let session: DiscoverySessionRequest =
					bincode::deserialize(value).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?;
				sessions.push((key.to_string(), session));
			}
		}
		Ok(sessions)
	}

	/// Set Discovery session ran by this node
	async fn set_discovery_session(
		&self,
		session_id: &String,
		session: DiscoverySessionRequest,
	) -> StorageResult<()> {
		let encoded_data = bincode::serialize(&session).map_err(|e| StorageError {
			kind: StorageErrorKind::Serialization,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let write_txn = self.db.begin_write().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		{
			let mut table =
				write_txn.open_table(TABLE_DISCOVERY_SESSIONS).map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			table.insert(session_id.as_str(), encoded_data.as_slice()).map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;
		}
		write_txn.commit().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})
	}

	/// Get Discovery session by id
	async fn get_discovery_session(
		&self,
		session_id: &String,
	) -> StorageResult<Option<DiscoverySessionRequest>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let session = {
			let table =
				read_txn.open_table(TABLE_DISCOVERY_SESSIONS).map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			let value = table.get(session_id.as_str()).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			match value {
				Some(value) =>
					Some(bincode::deserialize(value.value()).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?),
				None => None,
			}
		};
		Ok(session)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(
		&self,
	) -> StorageResult<Vec<(String, InsightAnalystRequest)>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let mut sessions = Vec::new();
		{
			let table = read_txn.open_table(TABLE_INSIGHT_SESSIONS).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			let iter = table.iter().map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;
			for result in iter {
				let (key_access_guard, value_access_guard) =
					result.map_err(|err| StorageError {
						kind: StorageErrorKind::Internal,
						source: Arc::new(anyhow::Error::from(err)),
					})?;
				let key = key_access_guard.value();
				let value = value_access_guard.value();
				let session: InsightAnalystRequest =
					bincode::deserialize(value).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?;
				sessions.push((key.to_string(), session));
			}
		}
		Ok(sessions)
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(
		&self,
		session_id: &String,
		session: InsightAnalystRequest,
	) -> StorageResult<()> {
		let encoded_data = bincode::serialize(&session).map_err(|e| StorageError {
			kind: StorageErrorKind::Serialization,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let write_txn = self.db.begin_write().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		{
			let mut table =
				write_txn.open_table(TABLE_INSIGHT_SESSIONS).map_err(|e| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				})?;
			table.insert(session_id.as_str(), encoded_data.as_slice()).map_err(|e| {
				StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(e)),
				}
			})?;
		}
		write_txn.commit().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})
	}

	/// Get Insight session by id
	async fn get_insight_session(
		&self,
		session_id: &String,
	) -> StorageResult<Option<InsightAnalystRequest>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let session = {
			let table = read_txn.open_table(TABLE_INSIGHT_SESSIONS).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			let value = table.get(session_id.as_str()).map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;
			match value {
				Some(value) =>
					Some(bincode::deserialize(value.value()).map_err(|e| StorageError {
						kind: StorageErrorKind::Serialization,
						source: Arc::new(anyhow::Error::from(e)),
					})?),
				None => None,
			}
		};
		Ok(session)
	}

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, _api_key: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Asynchronously fetches popular queries .
	async fn autogenerate_queries(
		&self,
		_max_suggestions: i32,
	) -> StorageResult<Vec<QuerySuggestion>> {
		// Return an empty vector
		Ok(Vec::new())
	}
}
