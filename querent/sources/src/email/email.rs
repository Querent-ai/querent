use std::{
	io::{self, Cursor},
	net::TcpStream,
	ops::Range,
	path::Path,
	pin::Pin,
	sync::Arc,
};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use async_trait::async_trait;

use common::CollectedBytes;
use futures::stream::{self, Stream, StreamExt};
use imap::Session;
use native_tls::TlsStream;
use proto::semantics::EmailCollectorConfig;
use tokio::{
	fs::File,
	io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
	sync::Mutex,
};

#[derive(Debug, Clone)]
pub struct EmailSource {
	pub imap_server: String,
	pub imap_port: i32,
	pub imap_username: String,
	pub imap_password: String,
	pub imap_folder: String,
	pub imap_session: Arc<Mutex<Session<TlsStream<TcpStream>>>>,
	pub source_id: String,
}

impl EmailSource {
	pub async fn new(config: EmailCollectorConfig) -> anyhow::Result<Self> {
		let tls = native_tls::TlsConnector::builder().build()?;
		let client = imap::connect(
			(config.imap_server.as_str(), config.imap_port as u16),
			config.imap_server.as_str(),
			&tls,
		)?;

		let imap_session = client
			.login(config.imap_username.clone(), config.imap_password.clone())
			.map_err(|e| e.0)?;

		Ok(EmailSource {
			imap_server: config.imap_server,
			imap_port: config.imap_port,
			imap_username: config.imap_username,
			imap_password: config.imap_password,
			imap_folder: config.imap_folder,
			imap_session: Arc::new(Mutex::new(imap_session)),
			source_id: config.id.clone(),
		})
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
		session.select(self.imap_folder.as_str()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error selecting folder: {:?}", err).into(),
			)
		})?;

		let messages = session.fetch("1:*", "RFC822").map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error fetching emails: {:?}", err).into(),
			)
		})?;

		for message in messages.iter() {
			if let Some(body) = message.body() {
				let mut reader = &body[..]; // Convert &[u8] to a slice
				tokio::io::copy_buf(&mut reader, output).await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error writing email body: {:?}", err).into(),
					)
				})?;
				output.flush().await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error flushing output: {:?}", err).into(),
					)
				})?;
			}
		}

		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(Vec::new())
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let file = File::open(path).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error opening file: {:?}", err).into(),
			)
		})?;

		let mut file = file;

		file.seek(io::SeekFrom::Start(range.start as u64)).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error seeking file: {:?}", err).into(),
			)
		})?;

		let stream = file.take(range.len() as u64);

		Ok(Box::new(stream) as Box<dyn AsyncRead + Send + Unpin>)
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(Vec::new())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(0)
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let session_lock = self.imap_session.clone();
		let mut session = session_lock.lock().await;

		session.select(self.imap_folder.as_str()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error selecting folder: {:?}", err).into(),
			)
		})?;

		let fetches = session.fetch("1", "RFC822").map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error fetching email: {:?}", err).into(),
			)
		})?;
		let collected_messages: Vec<CollectedBytes> = fetches
			.iter()
			.filter_map(|message| {
				message.body().map(|body| {
					let cursor = Cursor::new(body.to_vec());
					CollectedBytes {
						file: None,
						eof: true,
						doc_source: Some("email://unknown_sender".to_string()),
						extension: Some("txt".to_string()),
						size: Some(body.len() as usize),
						source_id: self.source_id.clone(),
						data: Some(Box::pin(cursor)),
					}
				})
			})
			.collect();

		let message_streams = collected_messages
			.into_iter()
			.map(|message| stream::once(async { Ok(message) }).boxed())
			.collect::<Vec<_>>();

		let stream = Box::pin(stream::iter(message_streams).flatten());

		Ok(stream)
	}
}
