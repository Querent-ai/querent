use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use redb::{Database, TableDefinition};
use std::path::PathBuf;

use crate::{Storage, StorageError, StorageErrorKind, StorageResult};

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
		std::fs::create_dir_all(&dir_path)
			.expect("Failed to create directory to init key value store");
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

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
		Ok(())
	}

	/// Fetch DiscoveryPayload from storage
	async fn fetch_discovered_knowledge(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DocumentPayload>> {
		// Implement Neo4j similarity search logic (if needed)
		Ok(vec![])
	}

	async fn similarity_search_l2(
		&self,
		_session_id: String,
		_collection_id: String,
		_payload: &Vec<f32>,
		_max_results: i32,
	) -> StorageResult<Vec<DocumentPayload>> {
		// Implement Neo4j similarity search logic (if needed)
		Ok(vec![])
	}

	/// Store key value pair
	async fn store_kv(&self, key: &String, value: &String) -> StorageResult<()> {
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
	async fn get_kv(&self, key: &String) -> StorageResult<Option<String>> {
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
			None => None,
		};
		Ok(value)
	}
}
