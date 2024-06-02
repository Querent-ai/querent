use std::{
	io::SeekFrom,
	ops::Range,
	path::{Path, PathBuf},
};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;
use tokio::{
	fs,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
};

#[derive(Clone)]
pub struct LocalFolderSource {
	folder_path: PathBuf,
	chunk_size: usize,
}

impl LocalFolderSource {
	pub fn new(folder_path: PathBuf, chunk_size: Option<usize>) -> Self {
		let chunk_size = chunk_size.unwrap_or(64000);
		Self { folder_path, chunk_size }
	}

	fn full_path(&self, path: &Path) -> PathBuf {
		self.folder_path.join(path)
	}

	async fn poll_data_recursive(
		&mut self,
		folder_path: PathBuf,
		output: &mut dyn SendableAsync,
	) -> SourceResult<()> {
		let mut entries = fs::read_dir(&folder_path).await.map_err(|e| SourceError::from(e))?; // Read the directory.
		while let Some(entry) = entries.next_entry().await.map_err(|e| SourceError::from(e))? {
			if let Ok(metadata) = entry.metadata().await {
				if metadata.is_file() {
					// If it's a file, read and write it to the output
					let file_path = entry.path();
					self.read_file_and_write_to_output(&file_path, output).await?;
				} else if metadata.is_dir() {
					let mut this = self.clone(); // Clone self to avoid mutable borrow issues
					Box::pin(this.poll_data_recursive(entry.path(), output)).await?;
				}
			} else {
				return Err(SourceError::new(
					SourceErrorKind::NotFound,
					anyhow::anyhow!("File not found").into(),
				)
				.into());
			}
		}
		Ok(())
	}

	async fn read_file_and_write_to_output(
		&self,
		file_path: &Path,
		output: &mut dyn SendableAsync,
	) -> SourceResult<()> {
		let file = fs::File::open(file_path).await.map_err(|e| SourceError::from(e))?;
		let mut reader = BufReader::new(file);
		let mut buffer = vec![0; self.chunk_size];
		loop {
			let bytes_read = reader.read(&mut buffer).await.map_err(|e| SourceError::from(e))?;
			if bytes_read == 0 {
				break;
			}
			output
				.write_all(&buffer[..bytes_read])
				.await
				.map_err(|e| SourceError::from(e))?;
		}
		output.flush().await.map_err(|e| SourceError::from(e))?;
		Ok(())
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
		let mut entries =
			fs::read_dir(&self.folder_path).await.map_err(|e| SourceError::from(e))?;
		entries.next_entry().await.map_err(|e| SourceError::from(e))?;
		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(|e| SourceError::from(e))?;
		file.seek(SeekFrom::Start(range.start as u64))
			.await
			.map_err(|e| SourceError::from(e))?;

		let mut buffer = vec![0; range.len()];
		file.read_exact(&mut buffer).await.map_err(|e| SourceError::from(e))?;
		Ok(buffer)
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(|e| SourceError::from(e))?;
		file.seek(SeekFrom::Start(range.start as u64))
			.await
			.map_err(|e| SourceError::from(e))?;

		let reader = BufReader::new(file.take(range.len() as u64));
		Ok(Box::new(reader))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let full_path = self.full_path(path);
		let mut file = fs::File::open(&full_path).await.map_err(|e| SourceError::from(e))?;

		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).await.map_err(|e| SourceError::from(e))?;
		Ok(buffer)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let full_path = self.full_path(path);
		let metadata = fs::metadata(&full_path).await.map_err(|e| SourceError::from(e))?;
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
			let file = fs::File::open(&full_path).await.map_err(|e| SourceError::from(e))?;
			let mut reader = BufReader::new(file);
			tokio::io::copy_buf(&mut reader, output)
				.await
				.map_err(|e| SourceError::from(e))?;
			output.flush().await.map_err(|e| SourceError::from(e))?;
			Ok(())
		})
	}
	async fn poll_data(&mut self, output: &mut dyn SendableAsync) -> SourceResult<()> {
		self.poll_data_recursive(self.folder_path.clone(), output).await
	}
}
