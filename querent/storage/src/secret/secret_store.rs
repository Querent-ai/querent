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

use crate::{SecretStorage, StorageError, StorageErrorKind, StorageResult, RIAN_API_KEY};
use async_trait::async_trait;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;

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
impl SecretStorage for SecretStore {
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

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, api_key: &String) -> StorageResult<()> {
		self.store_secret(&RIAN_API_KEY.to_string(), api_key).await?;
		Ok(())
	}

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>> {
		self.get_secret(&RIAN_API_KEY.to_string()).await
	}
}
