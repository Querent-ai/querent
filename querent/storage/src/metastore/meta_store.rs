// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use crate::{MetaStorage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;
use proto::{
	discovery::DiscoverySessionRequest, layer::LayerSessionRequest,
	semantics::SemanticPipelineRequest, InsightAnalystRequest,
};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;

const TABLE_PIPELINES: TableDefinition<&str, &[u8]> = TableDefinition::new("querent_pipelines");
const TABLE_DISCOVERY_SESSIONS: TableDefinition<&str, &[u8]> =
	TableDefinition::new("querent_discovery_sessions");
const TABLE_INSIGHT_SESSIONS: TableDefinition<&str, &[u8]> =
	TableDefinition::new("querent_insight_sessions");

const TABLE_LAYER_SESSIONS: TableDefinition<&str, &[u8]> =
	TableDefinition::new("querent_layer_sessions");

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
impl MetaStorage for MetaStore {
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

	/// get all layer sessions
	async fn get_all_layer_sessions(&self) -> StorageResult<Vec<(String, LayerSessionRequest)>> {
		let read_txn = self.db.begin_read().map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		let mut sessions = Vec::new();
		{
			let table = read_txn.open_table(TABLE_LAYER_SESSIONS).map_err(|e| StorageError {
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
				let session: LayerSessionRequest =
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

	/// Set Layer session ran by this node
	/// Set Discovery session ran by this node
	async fn set_layer_session(
		&self,
		session_id: &String,
		session: LayerSessionRequest,
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
				write_txn.open_table(TABLE_LAYER_SESSIONS).map_err(|e| StorageError {
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
}
