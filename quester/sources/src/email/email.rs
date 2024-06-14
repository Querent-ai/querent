use std::{
	io::{self},
	net::TcpStream,
	ops::Range,
	path::Path,
	pin::Pin,
	sync::Arc,
};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;

use common::CollectedBytes;
use futures::{
	stream::{self},
	Stream, StreamExt,
};
use imap::Session;
use native_tls::TlsStream;
use once_cell::sync::Lazy;
use proto::semantics::EmailCollectorConfig;
use std::result::Result::Ok;
use tokio::{
	fs::File,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
	sync::{Mutex, Semaphore},
};

#[derive(Debug, Clone)]
pub struct EmailSource {
	pub imap_server: String,
	pub imap_port: i32,
	pub imap_username: String,
	pub imap_password: String,
	pub imap_folder: String,
	pub imap_session: Arc<Mutex<Session<TlsStream<TcpStream>>>>,
}

impl EmailSource {
	pub async fn new(config: EmailCollectorConfig) -> Self {
		let tls = native_tls::TlsConnector::builder().build().unwrap();

		let client = imap::connect(
			(config.imap_server.as_str(), config.imap_port as u16),
			config.imap_server.as_str(),
			&tls,
		)
		.unwrap();

		// Login to the IMAP server
		let imap_session = client
			.login(config.imap_username.clone(), config.imap_password.clone())
			.map_err(|e| e.0)
			.unwrap();

		println!("{:?}", imap_session);

		let source = EmailSource {
			imap_server: config.imap_server,
			imap_port: config.imap_port,
			imap_username: config.imap_username,
			imap_password: config.imap_password,
			imap_folder: config.imap_folder,
			imap_session: Arc::new(Mutex::new(imap_session)),
		};

		source
	}
}

#[async_trait]
impl Source for EmailSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let session_lock = self.imap_session.clone();
		let mut session = session_lock.lock().await;

		let _ = session.select(self.imap_folder.as_str()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error getting emails: {:?}", err).into(),
			)
		});
		let messages = session.fetch("1:*", "RFC822").map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error getting emails: {:?}", err).into(),
			)
		});

		let messages = messages.unwrap();

		for message in messages.iter() {
			if let Some(body) = message.body() {
				let data = body.to_vec();
				let mut reader = &data[..]; // Convert Vec<u8> to a slice
				let _ = tokio::io::copy_buf(&mut reader, output).await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error getting emails: {:?}", err).into(),
					)
				});
				let _ = output.flush().await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error getting emails: {:?}", err).into(),
					)
				});
			}
		}

		SourceResult::Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		SourceResult::Ok(Vec::new())
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let file = File::open(path).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error getting emails: {:?}", err).into(),
			)
		});

		let mut file = file.unwrap();

		// Seek to the start position of the range
		let _ = file.seek(io::SeekFrom::Start(range.start as u64)).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error getting emails: {:?}", err).into(),
			)
		});

		// Create a stream that reads only the specified range
		let stream = file.take(range.len() as u64);

		// Return the stream as a boxed trait object
		SourceResult::Ok(Box::new(stream) as Box<dyn AsyncRead + Send + Unpin>)
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		SourceResult::Ok(Vec::new())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		SourceResult::Ok(u64::MIN)
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let session_lock = self.imap_session.clone();
		let mut session = session_lock.lock().await;

		let _ = session.select(self.imap_folder.as_str());

		let fetches = session.fetch("1", "RFC822").unwrap();

		// Collect all message streams first, ensuring all data is cloned and owned
		let message_streams: Vec<_> = fetches
			.iter()
			.filter_map(|message| {
				message.body().map(|body| {
					let body_owned = body.to_vec(); // Clone the body data to make it owned
					let body_str = std::str::from_utf8(&body_owned).unwrap().to_string(); // Convert and own the string
					println!("Body str {:?}", body_str);

					stream::once(async move {
						Ok(CollectedBytes {
							data: Some(body_owned.clone()),
							file: None,
							eof: true,
							doc_source: Some("email://unknown_sender".to_string()),
							extension: Some("txt".to_string()),
							size: Some(body_owned.len()),
						})
					})
					.boxed()
				})
			})
			.collect();

		// Create a single stream that flattens all message streams
		let stream: Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>> =
			Box::pin(stream::iter(message_streams).flatten());
		Ok(stream)
	}
}

// #[cfg(test)]
// mod tests {

// 	use super::*;

// 	#[tokio::test]
// 	async fn test_email_collector() {
// 		// Configure the GCS collector config with a mock credential
// 		let email_config = EmailCollectorConfig{
//             imap_server: "imap.gmail.com".to_string(),
//             imap_port: 993,
//             imap_username: "mail@email.com".to_string(),
//             imap_password: "password".to_string(),
//             imap_folder: "Inbox".to_string(),
//             imap_certfile: "cert.pem".to_string(),
//             imap_keyfile: "key.pem".to_string(),
//         };

//         let source = EmailSource::new(email_config).await;

//         let mut stream = source.poll_data().await.expect("Should not fail");

//         // Prepare a vector to hold the results
//         let mut collected_bytes = Vec::new();

//         // Consume the stream asynchronously
//         while let Some(result) = stream.next().await {
//             collected_bytes.push(result.expect("Expected valid CollectedBytes result"));
//         }

//         println!("Collected bytes: {:?}", collected_bytes);

// 	}
// }
