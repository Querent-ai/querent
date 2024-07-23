use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::{semantics::SemanticPipelineRequest, DiscoverySessionRequest, InsightAnalystRequest};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use crate::postgres_index::QuerySuggestion;
use crate::{
	DiscoveredKnowledge, Storage, StorageError, StorageErrorKind, StorageResult, RIAN_API_KEY,
};

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("querent_secrets");

pub struct SecretStore {
	db: Arc<Database>,
}

impl Debug for SecretStore {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		writeln!(f, "ReDb")?;
		Ok(())
	}
}

impl SecretStore {
	pub fn new(dir_path: PathBuf) -> Self {
		if !dir_path.exists() {
			std::fs::create_dir_all(&dir_path)
				.expect("Failed to create directory to init key value store");
		}
		let db_path = dir_path.join("querent_secrets.redb");
		let db = Database::create(db_path).expect("Failed to init key value store");

		let write_txn = db.begin_write().unwrap();
		write_txn.open_table(TABLE).unwrap();
		write_txn.commit().unwrap();

		Self { db: Arc::new(db) }
	}
}

#[async_trait]
impl Storage for SecretStore {
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

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
		Ok(())
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

	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
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

	/// Store key value pair
	async fn store_secret(&self, key: &String, value: &String) -> StorageResult<()> {
		let bytes = rmp_serde::to_vec(value).map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		let write_txn = self.db.begin_write().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		{
			let mut table = write_txn.open_table(TABLE).unwrap();
			table.insert(key.as_str(), bytes.as_slice()).map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;
		}
		write_txn.commit().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;

		Ok(())
	}

	/// Get value for key
	async fn get_secret(&self, key: &String) -> StorageResult<Option<String>> {
		let read_txn = self.db.begin_read().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		let table = read_txn.open_table(TABLE).map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		let key_val = table.get(key.as_str()).map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		let value = match key_val {
			Some(bytes) => {
				let value = rmp_serde::from_slice(bytes.value()).map_err(|err| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				})?;
				Some(value)
			},
			None => {
				// If we dont get any source for the given key, we return a log message
				log::info!("No source found for given ID: {:?}", key.clone());
				None
			},
		};
		Ok(value)
	}

	async fn get_all_secrets(&self) -> StorageResult<Vec<(String, String)>> {
		let read_txn = self.db.begin_read().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;

		let table = read_txn.open_table(TABLE).map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;

		let mut res = Vec::new();

		let iter = table.iter().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;
		for result in iter {
			let (key_access_guard, value_access_guard) = result.map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;

			let key = key_access_guard.value();
			let value = value_access_guard.value();

			let key_str =
				String::from_utf8(key.as_bytes().to_vec()).map_err(|err| StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				})?;

			let value: String = rmp_serde::from_slice(value).map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;

			res.push((key_str, value));
		}
		Ok(res)
	}

	async fn delete_secret(&self, key: &String) -> StorageResult<()> {
		let delete_txn = self.db.begin_write().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;

		{
			let mut table = delete_txn.open_table(TABLE).map_err(|err| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(err)),
			})?;

			let _ = table.remove(key.as_str());
		}

		delete_txn.commit().map_err(|err| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(err)),
		})?;

		Ok(())
	}

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<(String, SemanticPipelineRequest)>> {
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
	async fn get_all_discovery_sessions(
		&self,
	) -> StorageResult<Vec<(String, DiscoverySessionRequest)>> {
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
	async fn get_all_insight_sessions(
		&self,
	) -> StorageResult<Vec<(String, InsightAnalystRequest)>> {
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

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, api_key: &String) -> StorageResult<()> {
		// Inner Key: RIAN_API_KEY
		self.store_secret(&RIAN_API_KEY.to_string(), api_key).await?;
		Ok(())
	}

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>> {
		// Inner Key: RIAN_API_KEY
		self.get_secret(&RIAN_API_KEY.to_string()).await
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
