use std::{io, ops::Range, path::Path, pin::Pin};

use crate::{
	s3::retry::aws_retry, SendableAsync, Source, SourceError, SourceErrorKind, SourceResult,
	REQUEST_SEMAPHORE,
};
use async_stream::stream;
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::{
	config::Region,
	error::SdkError,
	operation::get_object::{GetObjectError, GetObjectOutput},
	primitives::ByteStream,
	Client as S3Client,
};

use common::{CollectedBytes, RetryParams};
use futures::Stream;
use proto::semantics::S3CollectorConfig;
use tokio::io::{AsyncRead, AsyncWriteExt, BufReader};

#[derive(Debug, Clone)]
pub struct S3Source {
	pub bucket_name: String,
	pub region: Region,
	pub access_key: String,
	pub secret_key: String,
	pub s3_client: Option<S3Client>,
	pub continuation_token: Option<String>,
	pub source_id: String,
	pub retry_params: RetryParams,
}

impl S3Source {
	pub async fn new(config: S3CollectorConfig) -> Self {
		let bucket_name = config.bucket.clone();
		let static_region_str = config.region.clone();
		let region = Region::new(static_region_str.clone());
		let access_key = config.access_key.clone();
		let secret_key = config.secret_key.clone();
		let source_id = config.id.clone();
		let retry_params = RetryParams::aggressive();
		let mut s3 = S3Source {
			bucket_name,
			region,
			access_key,
			secret_key,
			s3_client: None,
			continuation_token: None,
			source_id,
			retry_params,
		};

		let credentials = Credentials::new(
			config.access_key.clone(),
			config.secret_key.clone(),
			None,
			None,
			"manual",
		);
		let config = aws_config::from_env()
			.credentials_provider(credentials)
			.region(Region::new(static_region_str))
			.load()
			.await;

		s3.s3_client = Some(S3Client::new(&config));
		s3
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
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self
			.s3_client
			.as_ref()
			.unwrap()
			.list_objects_v2()
			.bucket(self.bucket_name.clone())
			.send()
			.await?;

		Ok(())
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
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
		self.get_to_vec(path, Some(range.clone())).await
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
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
		let bytes = self.get_to_vec(path, None).await?;
		Ok(bytes)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
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
		Ok(head_object_output.content_length().unwrap_or(0) as u64)
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let s3_client = self.s3_client.clone().unwrap();
		let bucket_name = self.bucket_name.clone();
		let continuation_token_start = self.continuation_token.clone();
		let source_id = self.source_id.clone();

		let stream = create_poll_data_stream(
			s3_client,
			bucket_name,
			continuation_token_start,
			source_id,
			self.retry_params,
		)
		.await;

		Ok(Box::pin(stream))
	}
}

async fn create_poll_data_stream(
	s3_client: S3Client,
	bucket_name: String,
	continuation_token_start: Option<String>,
	source_id: String,
	retry_params: RetryParams,
) -> impl Stream<Item = SourceResult<CollectedBytes>> + Send + 'static {
	stream! {
		let mut continuation_token = continuation_token_start;
		loop {
			let list_objects_v2_output =  aws_retry(&retry_params, || async { s3_client
				.list_objects_v2()
				.bucket(&bucket_name)
				.set_continuation_token(continuation_token.clone())
				.send().await
			}).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Connection,
					anyhow::anyhow!("Error listing objects from S3: {:?}", err).into(),
				)
			})?;
			if let Some(contents) = list_objects_v2_output.contents {
				for object in contents {
					let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
					if let Some(key) = object.key {
						let get_object_output = s3_client
							.get_object()
							.bucket(&bucket_name)
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
						let file_size = get_object_output.content_length();
						let collected_bytes = CollectedBytes::new(
							Some(Path::new(&key).to_path_buf()),
							Some(Box::pin(get_object_output.body.into_async_read())),
							true,
							Some(bucket_name.clone()),
							Some(file_size.unwrap_or(0) as usize),
							source_id.clone(),
							None,
						);
						yield Ok(collected_bytes);
					}
				}
			}

			if list_objects_v2_output.next_continuation_token.is_none() {
				break;
			}
			continuation_token = list_objects_v2_output.next_continuation_token;
		}
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

	use std::{collections::HashSet, env};

	use super::*;

	use dotenv::dotenv;
	use futures::StreamExt;

	#[tokio::test]
	async fn test_aws_collector() {
		dotenv().ok();
		let aws_config = S3CollectorConfig {
			access_key: env::var("AWS_ACCESS_KEY").unwrap_or_else(|_| "".to_string()),

			secret_key: env::var("AWS_SECRET_KEY").unwrap_or_else(|_| "".to_string()),

			region: "ap-south-1".to_string(),

			bucket: "querentbucket1".to_string(),
			id: "AWS-source".to_string(),
		};

		let s3_storage = S3Source::new(aws_config).await;

		let result = s3_storage.check_connectivity().await;
		assert!(result.is_ok());

		let result = s3_storage.poll_data().await;

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
				Err(_) => panic!("Expected successful data collection"),
			}
		}
		assert!(count_files.len() > 0, "Expected successful data collection");
	}

	#[tokio::test]
	async fn test_aws_collector_invalid_credentials() {
		dotenv().ok();
		let aws_config = S3CollectorConfig {
			access_key: "invalid_access_key".to_string(),
			secret_key: "invalid_secret_key".to_string(),
			region: "ap-south-1".to_string(),
			bucket: "querentbucket1".to_string(),
			id: "AWS-source".to_string(),
		};

		let s3_storage = S3Source::new(aws_config).await;
		let result = s3_storage.check_connectivity().await;

		assert!(result.is_err(), "Expected connection to fail with invalid credentials");
	}

	#[tokio::test]
	async fn test_aws_collector_invalid_bucket() {
		dotenv().ok();
		let aws_config = S3CollectorConfig {
			access_key: env::var("AWS_ACCESS_KEY").unwrap_or_else(|_| "".to_string()),
			secret_key: env::var("AWS_SECRET_KEY").unwrap_or_else(|_| "".to_string()),
			region: "ap-south-1".to_string(),
			bucket: "randombucketname".to_string(),
			id: "AWS-source".to_string(),
		};

		let s3_storage = S3Source::new(aws_config).await;
		let result = s3_storage.check_connectivity().await;

		assert!(result.is_err(), "Expected connection to fail with an invalid bucket name");
	}

	// #[tokio::test]
	// async fn test_aws_collector_empty_bucket() {
	// 	dotenv().ok();
	// 	let aws_config = S3CollectorConfig {
	// 		access_key: env::var("AWS_ACCESS_KEY").unwrap_or_else(|_| "".to_string()),
	// 		secret_key: env::var("AWS_SECRET_KEY").unwrap_or_else(|_| "".to_string()),
	// 		region: "ap-south-1".to_string(),
	// 		bucket: "empty-bucket".to_string(),
	// 		id: "AWS-source".to_string(),
	// 	};

	// 	let s3_storage = S3Source::new(aws_config).await;
	// 	let result = s3_storage.poll_data().await;
	// 	assert!(result.is_ok(), "Expected successful connectivity");

	// 	let mut stream = result.unwrap();
	// 	let mut count_files: HashSet<String> = HashSet::new();
	// 	while let Some(item) = stream.next().await {
	// 		match item {
	// 			Ok(collected_bytes) => {
	// 				if let Some(pathbuf) = collected_bytes.file {
	// 					if let Some(str_path) = pathbuf.to_str() {
	// 						count_files.insert(str_path.to_string());
	// 					}
	// 				}
	// 			},
	// 			Err(e) => panic!("Expected successful data collection {:?}", e),
	// 		}
	// 	}
	// 	assert!(count_files.len() == 0, "Expected successful data collection");
	// }
}
