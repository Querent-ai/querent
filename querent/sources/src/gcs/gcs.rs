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

use std::{fmt, ops::Range, path::Path, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use common::{retry, CollectedBytes};
use futures::{Stream, StreamExt};
use opendal::{Metakey, Operator};
use proto::semantics::GcsCollectorConfig;
use tokio::io::{AsyncRead, AsyncWriteExt};

use crate::{
	DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMetadata {
	// Full path
	pub key: String,
	// Seconds since unix epoch.
	pub last_modified: f64,
	pub total_size: usize,
}

#[derive(Clone)]
pub struct OpendalStorage {
	op: Operator,
	_bucket: Option<String>,
	source_id: String,
	retry_params: common::RetryParams,
}

impl fmt::Debug for OpendalStorage {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter
			.debug_struct("OpendalStorage")
			.field("operator", &self.op.info())
			.finish()
	}
}

impl OpendalStorage {
	/// Create a new google cloud storage.
	pub fn new_google_cloud_storage(
		cfg: opendal::services::Gcs,
		bucket: Option<String>,
		source_id: String,
	) -> Result<Self, SourceError> {
		let op = Operator::new(cfg)?.finish();
		Ok(Self { op, _bucket: bucket, source_id, retry_params: common::RetryParams::aggressive() })
	}
}

#[async_trait]
impl DataSource for OpendalStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.op.check().await?;
		Ok(())
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let path = path.as_os_str().to_string_lossy();
		let mut storage_reader = self.op.reader(&path).await?;
		tokio::io::copy(&mut storage_reader, output).await?;
		output.flush().await?;
		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_content = self.op.read_with(&path).range(range).await?;

		Ok(storage_content)
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_reader = self.op.reader_with(&path).range(range).await?;

		Ok(Box::new(storage_reader))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		// let path = path.as_os_str().to_string_lossy();
		let path_str = path.to_string_lossy();
		let storage_content = self.op.read(&path_str).await?;

		Ok(storage_content)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let path = path.as_os_str().to_string_lossy();
		let meta = self.op.stat(&path).await?;
		Ok(meta.content_length())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let op = self.op.clone();
		let source_id = self.source_id.clone();
		let stream = stream! {
			let mut object_lister = retry(&self.retry_params, || async {
				op.lister_with("")
				.recursive(true)
				.metakey(Metakey::ContentLength)
				.await
				.map_err(
					|e| SourceError::new(SourceErrorKind::Connection, anyhow::anyhow!("Error listing objects: {:?}", e).into())
				)
			}).await?;

			while let Some(object) = object_lister.next().await {
				let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
				match object {
					Ok(object) => {
						let key = object.path().to_string();
						let meta = op.stat(&key).await?;

						let file_size = meta.content_length() as usize;
						let storage_reader: opendal::Reader = op.reader_with(&key).await?;
						yield Ok(CollectedBytes::new(
							Some(Path::new(&key).to_path_buf()),
							Some(Box::pin(storage_reader)),
							true,
							Some(key.clone()),
							Some(file_size),
							source_id.clone(),
							None,
						));
					}
					Err(e) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error listing object: {:?}", e).into()
						));
					}
				}
			}
		};
		Ok(Box::pin(stream))
	}
}

impl From<opendal::Error> for SourceError {
	fn from(err: opendal::Error) -> Self {
		match err.kind() {
			opendal::ErrorKind::NotFound => SourceError::new(
				SourceErrorKind::NotFound,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::PermissionDenied => SourceError::new(
				SourceErrorKind::Unauthorized,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::ConfigInvalid => SourceError::new(
				SourceErrorKind::NotSupported,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			_ => SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
		}
	}
}

pub fn get_gcs_storage(gcs_config: GcsCollectorConfig) -> Result<OpendalStorage, SourceError> {
	let credentials = general_purpose::STANDARD.encode(&gcs_config.credentials);
	let mut cfg = opendal::services::Gcs::default();
	cfg.credential(&credentials);
	cfg.bucket(&gcs_config.bucket);
	OpendalStorage::new_google_cloud_storage(cfg, Some(gcs_config.bucket), gcs_config.id.clone())
}

// #[cfg(test)]
// mod tests {

// 	use std::{collections::HashSet, env};

// 	use super::*;
// 	use dotenv::dotenv;

// 	#[tokio::test]
// 	async fn test_gcs_collector() {
// 		dotenv().ok();

// 		// Configure the GCS collector config with a mock credential
// 		let gcs_config = GcsCollectorConfig {
// 			bucket: "querent-testing-api".to_string(),
// 			credentials: env::var("GCS_CREDENTIALS").unwrap_or_else(|_| "{}".to_string()),
// 			id: "GCS-source".to_string(),
// 		};

// 		// Initialize the GCS storage
// 		match get_gcs_storage(gcs_config) {
// 			Ok(gcs_storage) => {
// 				assert!(gcs_storage.check_connectivity().await.is_ok(), "Failed to connect");

// 				let result = gcs_storage.poll_data().await;

// 				let mut stream = result.unwrap();
// 				let mut count_files: HashSet<String> = HashSet::new();
// 				while let Some(item) = stream.next().await {
// 					match item {
// 						Ok(collected_bytes) =>
// 							if let Some(pathbuf) = collected_bytes.file {
// 								if let Some(str_path) = pathbuf.to_str() {
// 									count_files.insert(str_path.to_string());
// 								}
// 							},
// 						Err(err) => eprintln!("Expected successful data collection {:?}", err),
// 					}
// 				}
// 				println!("Files are --- {:?}", count_files);
// 			},
// 			Err(e) => {
// 				println!("Failed to initialize GCS storage: {:?}", e);
// 				assert!(false, "Storage initialization failed with error: {:?}", e);
// 			},
// 		}
// 	}
// }
