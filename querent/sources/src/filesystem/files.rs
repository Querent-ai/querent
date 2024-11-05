use crate::{
	DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE,
};
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{stream, Stream};
use std::{
	io::SeekFrom,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};
use tokio::{
	fs,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
};

#[derive(Clone, Debug)]
pub struct LocalFolderSource {
	folder_path: PathBuf,
	source_id: String,
}

impl LocalFolderSource {
	pub fn new(folder_path: PathBuf, source_id: String) -> Self {
		Self { folder_path, source_id }
	}

	fn full_path(&self, path: &Path) -> PathBuf {
		self.folder_path.join(path)
	}

	async fn poll_data_recursive<'life0>(
		&self,
		folder_path: PathBuf,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let mut streams: Vec<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send>>> =
			Vec::new();

		let mut stack: Vec<PathBuf> = vec![folder_path];
		while let Some(current_path) = stack.pop() {
			let mut entries = match fs::read_dir(&current_path).await {
				Ok(entries) => entries,
				Err(e) => {
					return Err(SourceError::from(e));
				},
			};

			while let Ok(Some(entry)) = entries.next_entry().await {
				let metadata = match entry.metadata().await {
					Ok(metadata) => metadata,
					Err(e) => {
						return Err(SourceError::from(e));
					},
				};

				if metadata.is_file() {
					let file_path = entry.path();
					let file_stream = self.read_file_and_stream_output(file_path.clone()).await?;
					streams.push(file_stream);
				} else if metadata.is_dir() {
					stack.push(entry.path());
				}
			}
		}
		let combined_stream = stream::select_all(streams);
		Ok(Box::pin(combined_stream))
	}

	async fn read_file_and_stream_output(
		&self,
		file_path: PathBuf,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send>>> {
		let source_id = self.source_id.clone();

		let stream = async_stream::stream! {
			let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
			let file_metadata = fs::metadata(&file_path).await.map_err(SourceError::from)?;
			let file_size = file_metadata.len() as usize;
			let file_name = file_path.to_string_lossy().to_string();
			let file = fs::File::open(&file_path).await.map_err(SourceError::from)?;
			let reader = BufReader::new(file);

			let collected_bytes = CollectedBytes::new(
				Some(file_path.clone()),
				Some(Box::pin(reader)),
				true,
				Some(file_name),
				Some(file_size),
				source_id.clone(),
				None,
			);

			yield Ok(collected_bytes);
		};

		Ok(Box::pin(stream))
	}
}

#[async_trait]
impl DataSource for LocalFolderSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		if !self.folder_path.exists() {
			return Err(SourceError::new(
				SourceErrorKind::NotFound,
				anyhow::anyhow!("Folder not found").into(),
			)
			.into());
		}
		let mut entries = fs::read_dir(&self.folder_path).await.map_err(SourceError::from)?;
		entries.next_entry().await.map_err(SourceError::from)?;

		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(SourceError::from)?;
		file.seek(SeekFrom::Start(range.start as u64))
			.await
			.map_err(SourceError::from)?;

		let mut buffer = vec![0; range.len()];
		file.read_exact(&mut buffer).await.map_err(SourceError::from)?;

		Ok(buffer)
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(SourceError::from)?;
		file.seek(SeekFrom::Start(range.start as u64))
			.await
			.map_err(SourceError::from)?;

		let reader = BufReader::new(file.take(range.len() as u64));

		Ok(Box::new(reader))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(SourceError::from)?;

		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).await.map_err(SourceError::from)?;

		Ok(buffer)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let full_path = self.full_path(path);
		let metadata = fs::metadata(&full_path).await.map_err(SourceError::from)?;

		Ok(metadata.len())
	}

	fn copy_to<'life0, 'life1, 'life2, 'async_trait>(
		&'life0 self,
		path: &'life1 Path,
		output: &'life2 mut dyn SendableAsync,
	) -> ::core::pin::Pin<
		Box<
			dyn ::core::future::Future<Output = SourceResult<()>>
				+ ::core::marker::Send
				+ 'async_trait,
		>,
	>
	where
		'life0: 'async_trait,
		'life1: 'async_trait,
		'life2: 'async_trait,
		Self: 'async_trait,
	{
		Box::pin(async move {
			let full_path = self.full_path(path);
			let file = fs::File::open(&full_path).await.map_err(SourceError::from)?;
			let mut reader = BufReader::new(file);
			tokio::io::copy_buf(&mut reader, output).await.map_err(SourceError::from)?;
			output.flush().await.map_err(SourceError::from)?;

			Ok(())
		})
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let result = self.poll_data_recursive(self.folder_path.clone()).await;

		result
	}
}

#[cfg(test)]
mod tests {

	use std::{collections::HashSet, path::PathBuf};

	use futures::StreamExt;
	use tempfile::TempDir;

	use crate::DataSource;

	use super::LocalFolderSource;

	#[tokio::test]
	async fn test_local_file_collector() {
		let local_storage = LocalFolderSource::new(
			PathBuf::from("src/filesystem/".to_string()),
			"LocalFolderSource".to_string(),
		);
		let connectivity = local_storage.check_connectivity().await;

		assert!(connectivity.is_ok(), "Got connectivity error");

		let result = local_storage.poll_data().await;

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) =>
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					},
				Err(err) => eprintln!("Expected successful data collection {:?}", err),
			}
		}
		println!("Files are --- {:?}", count_files);
	}

	#[tokio::test]
	async fn test_local_file_collector_invalid_path() {
		let local_storage = LocalFolderSource::new(
			PathBuf::from("src/invalid_path/"),
			"LocalFolderSource".to_string(),
		);
		let connectivity = local_storage.check_connectivity().await;

		// println!("Error is  {:?}", connectivity.err());

		assert!(connectivity.is_err(), "Expected connectivity check to fail with an invalid path");
	}

	#[tokio::test]
	async fn test_local_file_collector_no_permissions() {
		let local_storage =
			LocalFolderSource::new(PathBuf::from("/root/"), "LocalFolderSource".to_string());
		let connectivity = local_storage.check_connectivity().await;

		// println!("Error is {:?}", connectivity.err());

		assert!(
			connectivity.is_err(),
			"Expected connectivity check to fail due to lack of permissions"
		);
	}

	#[tokio::test]
	async fn test_local_file_collector_empty_directory() {
		let temp_dir = TempDir::new().expect("Failed to create temp directory");
		let temp_path = temp_dir.path().to_path_buf();

		let local_storage = LocalFolderSource::new(temp_path, "LocalFolderSource".to_string());

		let result = local_storage.poll_data().await;
		assert!(result.is_ok(), "Expected successful connectivity");

		let mut stream = result.unwrap();
		assert!(
			stream.next().await.is_none(),
			"Expected no files to be collected in an empty directory"
		);
	}
}
