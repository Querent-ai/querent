use std::{
	io::SeekFrom,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{stream, Stream};
use tokio::{
	fs,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
};

#[derive(Clone, Debug)]
pub struct LocalFolderSource {
	folder_path: PathBuf,
	chunk_size: usize,
}

impl LocalFolderSource {
	pub fn new(folder_path: PathBuf, chunk_size: Option<usize>) -> Self {
		let chunk_size = chunk_size.unwrap_or(1024 * 1024 * 10); // Default chunk size is 10MB
		Self { folder_path, chunk_size }
	}

	fn full_path(&self, path: &Path) -> PathBuf {
		self.folder_path.join(path)
	}

	async fn poll_data_recursive(
		&self,
		folder_path: PathBuf,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
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
		let chunk_size = self.chunk_size;
		let stream = stream::unfold(
			(file_path, chunk_size, 0, false),
			move |(file_path, chunk_size, offset, eof_reached)| async move {
				if eof_reached {
					return None;
				}
				let file = match fs::File::open(&file_path).await {
					Ok(f) => f,
					Err(e) =>
						return Some((
							Err(SourceError::from(e)),
							(file_path, chunk_size, offset, false),
						)),
				};
				let mut reader = BufReader::new(file);
				let mut buffer = vec![0; chunk_size];
				if let Err(e) = reader.seek(SeekFrom::Start(offset as u64)).await {
					return Some((
						Err(SourceError::from(e)),
						(file_path, chunk_size, offset, false),
					));
				}

				let bytes_read = match reader.read(&mut buffer).await {
					Ok(br) => br,
					Err(e) =>
						return Some((
							Err(SourceError::from(e)),
							(file_path, chunk_size, offset, false),
						)),
				};

				if bytes_read == 0 {
					// End of file reached
					let eof_collected_bytes = CollectedBytes::new(
						Some(file_path.clone()),
						None,
						true,
						Some(file_path.to_string_lossy().to_string()),
						None,
					);
					return Some((Ok(eof_collected_bytes), (file_path, chunk_size, offset, true)));
				}

				let collected_bytes = CollectedBytes::new(
					Some(file_path.clone()),
					Some(buffer[..bytes_read].to_vec()),
					false,
					Some(file_path.to_string_lossy().to_string()),
					Some(bytes_read),
				);

				Some((Ok(collected_bytes), (file_path, chunk_size, offset + bytes_read, false)))
			},
		);

		Ok(Box::pin(stream))
	}
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
		// Check if path is a folder and has entries
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
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		self.poll_data_recursive(self.folder_path.clone()).await
	}
}

// #[cfg(test)]
// mod tests {

//     use std::{collections::HashSet, path::PathBuf};
// 	use futures::StreamExt;

// use crate::Source;

// use super::LocalFolderSource;

// 	#[tokio::test]
// 	async fn test_local_file_collector() {
// 		let directory_path = "/home/ansh/querent/quester/quester/storage/sql".to_string();
// 		let root_path = PathBuf::from(directory_path);

// 		let local_storage = LocalFolderSource::new(root_path, None);

// 		println!("Connectivity :- {:?}", local_storage.check_connectivity().await);

// 		let result = local_storage.poll_data().await;

// 		let mut stream = result.unwrap();
// 		let mut count_files: HashSet<String> = HashSet::new();
// 		while let Some(item) = stream.next().await {
// 			match item {
// 				Ok(collected_bytes) => {

// 					if let Some(pathbuf) = collected_bytes.file {
// 						if let Some(str_path) = pathbuf.to_str() {
// 							count_files.insert(str_path.to_string());
// 						}
// 					}
// 				}
// 				Err(_) => panic!("Expected successful data collection"),
// 				// None => println!("Received none");
// 			}
// 		}
// 		println!("Files are --- {:?}", count_files);

// 	}
// }
