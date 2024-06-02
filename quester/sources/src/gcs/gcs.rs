use std::{fmt, ops::Range, path::Path};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use opendal::{Metakey, Operator};
use proto::semantics::GcsCollectorConfig;
use tokio::io::{AsyncRead, AsyncWriteExt};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

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
	) -> Result<Self, SourceError> {
		let op = Operator::new(cfg)?.finish();
		Ok(Self { op, _bucket: bucket })
	}
}

#[async_trait]
impl Source for OpendalStorage {
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
		let path = path.as_os_str().to_string_lossy();
		let storage_content = self.op.read(&path).await?;

		Ok(storage_content)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let path = path.as_os_str().to_string_lossy();
		let meta = self.op.stat(&path).await?;
		Ok(meta.content_length())
	}

	async fn poll_data(&mut self, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let bucket_name = self._bucket.clone().unwrap_or_default();
		let object_lister = self
			.op
			.lister_with(bucket_name.as_str())
			.recursive(true)
			.metakey(Metakey::ContentLength)
			.await?;

		let stream = stream::unfold(object_lister, |mut object_lister| async move {
			match object_lister.next().await {
				Some(Ok(object)) => {
					let key = object.path().to_string();
					let om = object.metadata();
					let last_modified = match om.last_modified() {
						Some(t) => t.timestamp() as f64,
						None => 0_f64,
					};
					let total_size = om.content_length() as usize;
					let metadata = ObjectMetadata { key, last_modified, total_size };
					Some((Ok::<_, SourceError>(metadata), object_lister))
				},
				Some(Err(err)) => Some((Err(err.into()), object_lister)),
				None => None,
			}
		});

		let mut stream = Box::pin(stream);
		while let Some(metadata) = stream.next().await {
			let metadata = metadata?;
			let key = metadata.key;
			let mut storage_reader = self.op.reader(&key).await?;
			tokio::io::copy(&mut storage_reader, output).await?;
			output.flush().await?;
		}
		Ok(())
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
	let mut cfg = opendal::services::Gcs::default();
	cfg.credential_path(&gcs_config.credentials);
	cfg.bucket(&gcs_config.bucket);
	OpendalStorage::new_google_cloud_storage(cfg, Some(gcs_config.bucket))
}
