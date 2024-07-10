use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;

use crate::{Storage, StorageError, StorageErrorKind, StorageResult};

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
	async fn get_all_pipelines(&self) -> StorageResult<Vec<SemanticPipelineRequest>> {
		Ok(Vec::new())
	}

	/// Set SemanticPipeline ran by this node
		async fn set_pipeline(&self, pipeline_id: &String, pipeline: SemanticPipelineRequest)
		-> StorageResult<()> {
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
	async fn set_discovery_session(&self, _session_id: &String, _session: DiscoverySessionRequest) -> StorageResult<()> {
		Ok(())
	}

	/// Get Discovery session by id
	async fn get_discovery_session(&self, _session_id: &String) -> StorageResult<Option<DiscoverySessionRequest>> {
		Ok(None)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(&self) -> StorageResult<Vec<InsightAnalystRequest>> {
		Ok(Vec::new())
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(&self, _session_id: &String, _session: InsightAnalystRequest) -> StorageResult<()> {
		Ok(())
	}

	/// Get Insight session by id
	async fn get_insight_session(&self, _session_id: &String) -> StorageResult<Option<InsightAnalystRequest>> {
		Ok(None)
	}
}
