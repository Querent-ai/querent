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
	io::{Cursor, Read},
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use flate2::read::GzDecoder;
use futures::Stream;
use tar::Archive;
use tokio::{io::AsyncRead, task};
use zip::ZipArchive;

use crate::{DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult};

#[derive(Clone, Debug)]
pub struct ZipSource {
	zip_bytes: Vec<u8>,
	source_id: String,
	extension: String,
}

impl ZipSource {
	pub async fn new(
		zip_bytes: Vec<u8>,
		source_id: String,
		extension: String,
	) -> anyhow::Result<Self> {
		Ok(ZipSource { zip_bytes, source_id, extension })
	}
}

fn string_to_async_read(data: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(data.into_bytes())
}

#[async_trait]
impl DataSource for ZipSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		Ok(Box::new(string_to_async_read("".to_string())))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(0)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let source_id = self.source_id.clone();
		let zip_data = self.zip_bytes.clone();
		let extension = self.extension.clone();
		let zip_files = vec!["zip", "zipx", "jar", "war", "ear"];

		let stream = stream! {
			if zip_files.contains(&extension.as_str()) {
				// File is zip
				let archive_length = match ZipArchive::new(Cursor::new(zip_data.clone())) {
					Ok(archive) => archive.len(),
					Err(err) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Failed to read zip archive length: {:?}", err).into(),
						));
						return;
					}
				};

				for i in 0..archive_length {
					let zip_data = zip_data.clone();
					let (file_data, file_name) = match task::spawn_blocking(move || {
						let mut archive = ZipArchive::new(Cursor::new(zip_data)).unwrap();
						let mut file = archive.by_index(i).unwrap();
						let file_name = file.name().to_string();

						let mut buffer = Vec::new();
						file.read_to_end(&mut buffer).map(|_| (buffer, file_name))
					}).await {
						Ok(Ok((buffer, file_name))) => (buffer, file_name),
						Ok(Err(err)) => {
							yield Err(SourceError::new(
								SourceErrorKind::Io,
								anyhow::anyhow!("Failed to read file data: {:?}", err).into(),
							));
							return;
						}
						Err(join_err) => {
							yield Err(SourceError::new(
								SourceErrorKind::Io,
								anyhow::anyhow!("Join error: {:?}", join_err).into(),
							));
							return;
						}
					};

					if file_data.len() == 0 {
						continue;
					}

					let extension = file_name.split('.').last().unwrap_or("").to_string();

					let file_path = PathBuf::from(file_name.clone());
					let doc_source = Some(format!("filesystem://{}", file_name.clone()));

					let collected_bytes = CollectedBytes {
						file: Some(file_path),
						data: Some(Box::pin(Cursor::new(file_data.clone()))),
						eof: true,
						doc_source,
						size: Some(file_data.len()),
						source_id: source_id.clone(),
						_owned_permit: None,
						image_id: None,
						extension: Some(extension),
					};

					yield Ok(collected_bytes);
				}
			} else {
				// File is tar
				let is_gzipped = extension == "gz";
				let tar_data_cursor = Cursor::new(zip_data.clone());

				let decompressed_data = if is_gzipped {
					let mut decompressed_buffer = Vec::new();
					let mut decoder = GzDecoder::new(tar_data_cursor);

					if let Err(err) = decoder.read_to_end(&mut decompressed_buffer) {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Failed to decompress gzip data: {:?}", err).into(),
						));
						return;
					}

					Cursor::new(decompressed_buffer)
				} else {
					tar_data_cursor
				};

				let archive_result = task::spawn_blocking(move || {
					let mut archive = Archive::new(decompressed_data);

					let mut collected_entries = Vec::new();
					for entry_result in archive.entries()? {
						let mut entry = entry_result?;
						let file_name = entry.path()?.to_string_lossy().into_owned();
						let extension = file_name.split('.').last().unwrap_or("").to_string();

						let mut buffer = Vec::new();
						entry.read_to_end(&mut buffer)?;

						collected_entries.push((file_name, buffer, extension));
					}
					Ok::<_, std::io::Error>(collected_entries)
				}).await;

				let entries = match archive_result {
					Ok(Ok(entries)) => entries,
					Ok(Err(err)) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Failed to process tar archive: {:?}", err).into(),
						));
						return;
					},
					Err(join_err) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Join error: {:?}", join_err).into(),
						));
						return;
					},
				};

				for (file_name, buffer, extension) in entries {
					if buffer.len() == 0 {
						continue;
					}

					let file_path = PathBuf::from(&file_name);
					let doc_source = Some(format!("filesystem://{}", file_name));

					let collected_bytes = CollectedBytes {
						file: Some(file_path),
						data: Some(Box::pin(Cursor::new(buffer.clone()))),
						eof: true,
						doc_source,
						size: Some(buffer.len()),
						source_id: source_id.clone(),
						_owned_permit: None,
						image_id: None,
						extension: Some(extension),
					};

					yield Ok(collected_bytes);
				}
			}

		};

		Ok(Box::pin(stream))
	}
}

#[cfg(test)]
mod tests {

	use std::collections::HashSet;

	use futures::StreamExt;

	use crate::{zip::zip::ZipSource, DataSource};

	#[tokio::test]
	async fn test_zip_collector() {
		let bytes = include_bytes!("../../../../test_data/test-3.zip");
		let zip_source = ZipSource::new(bytes.into(), "File-system".to_string(), "zip".to_string())
			.await
			.unwrap();
		let connectivity = zip_source.check_connectivity().await;

		assert!(connectivity.is_ok(), "Got connectivity error");

		let result = zip_source.poll_data().await;

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
		assert!(count_files.len() > 0, "No files found");
	}

	#[tokio::test]
	async fn test_zip_collector_with_tar_file() {
		let bytes = include_bytes!("../../../../test_data/test4.tar.gz");
		let zip_source = ZipSource::new(bytes.into(), "File-system".to_string(), "gz".to_string())
			.await
			.unwrap();
		let connectivity = zip_source.check_connectivity().await;

		assert!(connectivity.is_ok(), "Got connectivity error");

		let result = zip_source.poll_data().await;

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
		assert!(count_files.len() > 0, "No files found");
	}
}
