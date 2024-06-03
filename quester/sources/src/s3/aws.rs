use std::{io, ops::Range, path::Path};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;
use aws_sdk_s3::{
	config::Region,
	error::SdkError,
	operation::get_object::{GetObjectError, GetObjectOutput},
	primitives::ByteStream,
	Client as S3Client,
};

use common::CollectedBytes;
use once_cell::sync::Lazy;
use proto::semantics::S3CollectorConfig;
use tokio::{
	io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
	sync::{mpsc, Semaphore},
};

static REQUEST_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(10000));

#[derive(Debug, Clone)]
pub struct S3Source {
	pub bucket_name: String,
	pub region: Region,
	pub access_key: String,
	pub secret_key: String,
	pub chunk_size: usize,
	pub s3_client: Option<S3Client>,
	pub continuation_token: Option<String>,
}

impl S3Source {
	pub fn new(config: S3CollectorConfig) -> Self {
		let bucket_name = config.bucket.clone();
		let static_region_str = config.region.clone();
		let region = Region::new(static_region_str);
		let access_key = config.access_key.clone();
		let secret_key = config.secret_key.clone();
		let chunk_size = 1024 * 1024 * 10; // this is 10MB
		S3Source {
			bucket_name,
			region,
			access_key,
			secret_key,
			chunk_size,
			s3_client: None,
			continuation_token: None,
		}
	}

	async fn create_get_object_request(
		&self,
		path: &Path,
		range_opt: Option<Range<usize>>,
	) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
		let key = path.to_string_lossy().to_string();
		let range_str = range_opt.map(|range| format!("bytes={}-{}", range.start, range.end - 1));
		// Split key in bucket and path
		let mut parts = key.splitn(2, '/');
		let bucket = parts.next().unwrap();
		let key = parts.next().unwrap();

		let get_object_output = self
			.s3_client
			.as_ref()
			.unwrap()
			.get_object()
			.bucket(bucket)
			.key(key)
			.set_range(range_str)
			.send()
			.await?;
		Ok(get_object_output)
	}

	async fn get_to_vec(
		&self,
		path: &Path,
		range_opt: Option<Range<usize>>,
	) -> SourceResult<Vec<u8>> {
		let cap = range_opt.as_ref().map(Range::len).unwrap_or(0);
		let get_object_output =
			self.create_get_object_request(path, range_opt.clone()).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error getting object from S3: {:?}", err).into(),
				)
			})?;
		let mut buf: Vec<u8> = Vec::with_capacity(cap);
		download_all(get_object_output.body, &mut buf).await?;
		Ok(buf)
	}
}

#[async_trait]
impl Source for S3Source {
	async fn list_files(&self, path: &Path) -> SourceResult<Vec<String>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;

        let mut files = Vec::new();
		let mut continuation_token: Option<String> = None;
		loop {
            let resp = self
                .s3_client
                .as_ref()
                .unwrap()
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(path.to_str().unwrap_or(""))
                .set_max_keys(Some(1000))
                .send()
                .await
                .map_err(|err| {
                    SourceError::new(
                        SourceErrorKind::Io,
                        anyhow::anyhow!("Error listing objects from S3: {:?}", err).into(),
                    )
                })?;

            for obj in resp.contents().unwrap_or_default() {
                files.push(obj.key().unwrap().to_string());
            }

            continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(files)
	}
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		// we ignore error as we never close the semaphore
		let _permit = REQUEST_SEMAPHORE.acquire().await;

		let _ = self.s3_client.as_ref().unwrap().list_objects_v2().bucket(self.bucket_name.clone()).send().await;
		Ok(())
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;
		let get_object_output =
			self.create_get_object_request(path, None).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error getting object from S3: {:?}", err).into(),
				)
			})?;
		let mut body_read = BufReader::new(get_object_output.body.into_async_read());
		tokio::io::copy_buf(&mut body_read, output).await?;
		output.flush().await?;
		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;
		self.get_to_vec(path, Some(range.clone())).await
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;
		let get_object_output =
			self.create_get_object_request(path, Some(range.clone())).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error getting object from S3: {:?}", err).into(),
				)
			})?;

		let body = get_object_output.body.into_async_read();
		Ok(Box::new(body))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;
		let bytes = self.get_to_vec(path, None).await?;
		Ok(bytes)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;
		let key = path.to_string_lossy().to_string();
		let mut parts = key.splitn(2, '/');
		let bucket = parts.next().unwrap();
		let key = parts.next().unwrap();
		let head_object_output = self
			.s3_client
			.as_ref()
			.unwrap()
			.head_object()
			.bucket(bucket)
			.key(key)
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error getting object from S3: {:?}", err).into(),
				)
			})?;
		Ok(head_object_output.content_length() as u64)
	}

	async fn poll_data(&self, output: mpsc::Sender<CollectedBytes>) -> SourceResult<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await;

		let mut continuation_token = self.continuation_token.clone();
		loop {
			let list_objects_v2 = self
				.s3_client
				.as_ref()
				.unwrap()
				.list_objects_v2()
				.bucket(&self.bucket_name)
				.set_continuation_token(continuation_token.clone());

			let list_objects_v2_output = list_objects_v2.send().await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error listing objects from S3: {:?}", err).into(),
				)
			})?;

			if let Some(contents) = list_objects_v2_output.contents {
				for object in contents {
					if let Some(key) = object.key {
						let get_object_output = self
							.s3_client
							.as_ref()
							.unwrap()
							.get_object()
							.bucket(&self.bucket_name)
							.key(&key)
							.send()
							.await
							.map_err(|err| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error getting object from S3: {:?}", err)
										.into(),
								)
							})?;

						let mut body_stream_reader =
							BufReader::new(get_object_output.body.into_async_read());
						let mut buffer = vec![0; 1024 * 1024 * 10]; // 10MB buffer

						loop {
							let bytes_read = body_stream_reader.read(&mut buffer).await?;

							// Break the loop if EOF is reached
							if bytes_read == 0 {
								break;
							}
							// Only process and serialize if bytes were read

							let collected_bytes = CollectedBytes::new(
								Some(Path::new(&key).to_path_buf()),
								Some(buffer[..bytes_read].to_vec()),
								false,
								Some(self.bucket_name.clone()),
								Some(bytes_read),
							);

							output.send(collected_bytes).await.map_err(|e| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error sending collected bytes: {:?}", e)
										.into(),
								)
							})?;
						}
						// Mark the end of file for the current object
						let eof_collected_bytes = CollectedBytes::new(
							Some(Path::new(&key).to_path_buf()),
							None,
							true,
							Some(self.bucket_name.clone()),
							None,
						);

						output.send(CollectedBytes::from(eof_collected_bytes)).await.map_err(
							|e| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error sending collected bytes: {:?}", e)
										.into(),
								)
							},
						)?;
					}
				}
			}

			if list_objects_v2_output.next_continuation_token.is_none() {
				break;
			}
			continuation_token = list_objects_v2_output.next_continuation_token;
		}
		Ok(())
	}
}

async fn download_all(byte_stream: ByteStream, output: &mut Vec<u8>) -> io::Result<()> {
	output.clear();
	let mut body_stream_reader = BufReader::new(byte_stream.into_async_read());
	tokio::io::copy_buf(&mut body_stream_reader, output).await?;
	output.shrink_to_fit();
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use aws_credential_types::Credentials;

	#[tokio::test]
	async fn test_aws_collector() {
		let aws_config = S3CollectorConfig {
			access_key: "AKIAU6GDY2RDMGC2RNTK".to_string(),
			secret_key: "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".to_string(),
			region: "ap-south-1".to_string(),
			bucket: "querentbucket1".to_string(),
		};

		let credentials = Credentials::new(aws_config.access_key.clone(), aws_config.secret_key.clone(), None, None, "manual");

		let mut s3_storage = S3Source::new(aws_config);

		let config = aws_config::from_env().credentials_provider(credentials).region(s3_storage.region.clone()).load().await;

		s3_storage.s3_client = Some(S3Client::new(&config));

		let result = s3_storage.check_connectivity().await;
    	assert!(result.is_ok());

		let path = Path::new("");
		let files = s3_storage.list_files(path).await.unwrap();

		println!("Total files {:?}", files);
	}
}