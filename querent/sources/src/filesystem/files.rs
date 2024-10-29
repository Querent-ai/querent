use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE};
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{stream, Stream};
use std::{
	io::{Cursor, Read, SeekFrom},
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};
use tokio::{
	fs,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
};
use zip::ZipArchive;

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
		let extension =
			file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_string();

		let stream = async_stream::stream! {
			let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
			if extension == "zip" {
				let mut collected_files = Vec::new();
				process_zip_file(file_path.clone(), source_id.clone(), &mut collected_files).await?;
				for collected_bytes in collected_files {
					yield Ok(collected_bytes);
				}
			} else {
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
			}
		};

		Ok(Box::pin(stream))
	}
}

async fn process_zip_file(
	file_path: PathBuf,
	source_id: String,
	collected_files: &mut Vec<CollectedBytes>,
) -> SourceResult<()> {
	let mut files_to_process = vec![fs::read(&file_path).await.map_err(SourceError::from)?];

	while let Some(file_data) = files_to_process.pop() {
		let cursor = Cursor::new(file_data);
		let mut archive = ZipArchive::new(cursor).map_err(|e| {
			SourceError::new(
				SourceErrorKind::Io,
				Arc::new(anyhow::anyhow!("Failed to read the file: {:?}", e).into()),
			)
		})?;

		for i in 0..archive.len() {
			let mut file = archive.by_index(i).map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					Arc::new(anyhow::anyhow!("Failed to unzip the file: {:?}", e).into()),
				)
			})?;

			if !file.is_dir() {
				let file_name = file.name().to_string();
				let file_size = file.size() as usize;
				let mut file_buffer = Vec::new();

				file.read_to_end(&mut file_buffer).map_err(SourceError::from)?;

				if !file_name.ends_with(".zip") {
					let collected_bytes = CollectedBytes::new(
						Some(PathBuf::from(file_name.clone())),
						Some(Box::pin(BufReader::new(Cursor::new(file_buffer)))),
						true,
						Some(file_name),
						Some(file_size),
						source_id.clone(),
						None,
					);

					collected_files.push(collected_bytes);
				} else {
					files_to_process.push(file_buffer);
				}
			}
		}
	}

	Ok(())
}

#[async_trait]
impl Source for LocalFolderSource {
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

	use crate::Source;

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
}
