use std::{io::{Cursor, Read}, ops::Range, path::{Path, PathBuf}, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use tokio::{io::AsyncRead, task};
use zip::ZipArchive;

use crate::{SendableAsync, DataSource, SourceError, SourceErrorKind, SourceResult};



#[derive(Clone, Debug)]
pub struct ZipSource {
    zip_bytes: Vec<u8>,
    source_id: String,
}


impl ZipSource {
	pub async fn new(zip_bytes: Vec<u8>, source_id: String) -> anyhow::Result<Self> {
		Ok(ZipSource {
            zip_bytes: zip_bytes,
            source_id: source_id,
		})
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


        let stream = stream! {
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
		let zip_source = ZipSource::new(
			bytes.into(),
            "File-system".to_string(),
		).await.unwrap();
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
		println!("Files are --- {:?}", count_files);
	}
}
