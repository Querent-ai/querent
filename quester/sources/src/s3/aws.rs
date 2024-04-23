use crate::{CollectedBytes, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_s3::{
	config::{Credentials, Region},
	Client as S3Client,
};
use futures_util::Stream;
use proto::semantics::S3CollectorConfig;

#[derive(Debug)]
pub struct S3Source {
	bucket_name: String,
	region: Region,
	access_key: String,
	secret_key: String,
	chunk_size: usize,
	s3_client: Option<S3Client>,
}

impl S3Source {
	pub fn new(config: S3CollectorConfig, prefix: &str) -> Self {
		let bucket_name = config.bucket.clone();
		let region = Region::from_static(config.region.as_str());
		let access_key = config.access_key.clone();
		let secret_key = config.secret_key.clone();
		let chunk_size = 1024 * 1024 * 10; // this is 10MB
		S3Source { bucket_name, region, access_key, secret_key, chunk_size, s3_client: None }
	}

	async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
		let credentials = Credentials::new(
			self.access_key.clone(),
			self.secret_key.clone(),
			None,
			None,
			"querent",
		);
		let mut s3_config = aws_sdk_s3::Config::builder().region(self.region.clone());
		s3_config.set_credentials_provider(Some(SharedCredentialsProvider::new(credentials)));
		let s3_client = S3Client::from_conf(s3_config.build());
		self.s3_client = Some(s3_client);
		Ok(())
	}

	async fn disconnect(&mut self) {
		// No asynchronous disconnect needed for rusoto
	}

	async fn download_object_as_byte_stream(
		&self,
		object_key: String,
	) -> Result<impl tokio::io::AsyncRead + Send + Sync, Box<dyn std::error::Error>> {
		let s3_client = self.s3_client.as_ref().unwrap();
		let result = s3_client
			.get_object()
			.bucket(self.bucket_name.clone())
			.key(object_key)
			.send()
			.await?;
		let body = result.body;
		Ok(body.into_async_read())
	}
}

#[async_trait]
impl Source for S3Source {
	async fn connect(&self) -> SourceResult<()> {
		if self.s3_client.is_none() {
			self.connect().await?;
		}
		Ok(())
	}

	async fn poll(&self) -> SourceResult<Box<dyn Stream<Item = CollectedBytes> + Unpin + Send>> {
		if self.s3_client.is_none() {
			self.connect().await?;
		}

		let s3_client = self.s3_client.as_ref().unwrap();

		let mut object_keys = Vec::new();

		match s3_client.list_objects_v2().bucket(self.bucket_name.clone()).send().await {
			Ok(output) =>
				if let Some(contents) = output.contents {
					for obj in contents {
						if let Some(key) = obj.key {
							object_keys.push(key);
						}
					}
				},
			Err(err) => {
				eprintln!("Error listing S3 objects: {:?}", err);
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error listing S3 objects: {:?}", err).into(),
				));
			},
		};

		// Create a stream that will read the objects from S3
		let stream = futures_util::stream::iter(object_keys.into_iter().map(move |key| {
			let s3_client = self.s3_client.as_ref().unwrap();
			let bucket_name = self.bucket_name.clone();
			let chunk_size = self.chunk_size;
			async move {
				let object_stream = s3_client
					.get_object()
					.bucket(bucket_name.clone())
					.key(key.clone())
					.send()
					.await
					.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error getting object from S3: {:?}", err).into(),
						)
					});
				let bytes_stream = match object_stream {
					Ok(result) => {
						let body = result.body;
						let bytes_stream = body.into_async_read();
						Some(Box::new(bytes_stream))
					},
					Err(err) => None,
				};
				CollectedBytes::new(key, bytes_stream, None, false, String::new())
			}
		}));

		Ok(Box::new(stream))
	}

	async fn disconnect(&self) -> SourceResult<()> {
		self.disconnect().await;
		Ok(())
	}
}
