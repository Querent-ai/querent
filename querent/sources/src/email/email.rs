use std::{
	io::{self, Cursor},
	net::TcpStream,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};

use crate::{
	DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE,
};
use async_trait::async_trait;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use common::{retry, CollectedBytes};
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
	pub retry_params: common::RetryParams,
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
			retry_params: common::RetryParams::aggressive(),
		})
	}

	fn extract_attachments(&self, email_content: &str) -> Vec<(String, Vec<u8>)> {
		let mut attachments = Vec::new();

		// Extract the boundary string
		let boundary = email_content
			.lines()
			.find(|line| line.starts_with("Content-Type: multipart"))
			.and_then(|line| line.split("boundary=").nth(1))
			.map(|b| b.trim_matches(|c: char| c == '"' || c.is_whitespace()));

		if let Some(boundary) = boundary {
			// Split the email content into MIME parts using the extracted boundary
			let parts: Vec<&str> = email_content.split(&format!("--{}", boundary)).collect();

			for part in parts {
				// Check if this part contains an attachment
				if part.contains("Content-Disposition: attachment") {
					let mut filename = String::new();
					let mut base64_content = String::new();

					// Extract filename and content type
					for line in part.lines() {
						if line.contains("filename=") {
							filename = line
								.split("filename=")
								.nth(1)
								.unwrap_or("")
								.trim_matches(|c: char| c == '"' || c.is_whitespace())
								.to_string();
						}
					}

					// Extract base64 content
					if let Some(base64_section) = part.split("\r\n\r\n").nth(1) {
						base64_content = base64_section
							.lines()
							.filter(|line| !line.starts_with("--"))
							.collect::<Vec<&str>>()
							.join("")
							.chars()
							.filter(|c| {
								c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '='
							})
							.collect();
					}
					if let Ok(decoded) = BASE64.decode(&base64_content) {
						attachments.push((filename, decoded));
					} else {
						eprintln!("Base64 decoding error for file: {}", filename);
					}
				}
			}
		} else {
			eprintln!("Could not find multipart boundary in email content");
		}

		attachments
	}
}

#[async_trait]
impl DataSource for EmailSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let tls = native_tls::TlsConnector::builder().build()?;

		let client = imap::connect(
			(self.imap_server.as_str(), self.imap_port as u16),
			self.imap_server.as_str(),
			&tls,
		)?;
		let imap_session = client
			.login(self.imap_username.clone(), self.imap_password.clone())
			.map_err(|e| e.0)?;
		// Replace the current session with the new one
		*self.imap_session.lock().await = imap_session;
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
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let session_lock = self.imap_session.clone();
		retry(&self.retry_params, || async {
			self.check_connectivity()
				.await
				.map_err(|err| SourceError::new(SourceErrorKind::Connection, err.into()))
		})
		.await?;
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
		let mut collected_messages = Vec::new();
		for fetch in fetches.into_iter() {
			if let Some(body) = fetch.body() {
				let email_content = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");
				let message_id = email_content
					.split("Message-ID: <")
					.nth(1)
					.and_then(|s| s.split('>').next())
					.and_then(|s| s.split('@').next())
					.unwrap_or("");

				let subject = email_content
					.lines()
					.find(|line| line.starts_with("Subject:"))
					.map(|line| line.trim_start_matches("Subject:").trim())
					.unwrap_or("");
				let main_text = email_content
					.split("Content-Type: text/plain; charset=\"UTF-8\"\r\n\r\n")
					.nth(1)
					.and_then(|s| s.split("\r\n\r\n--").next())
					.unwrap_or("");
				let combined_text = format!("{}, {}", subject, main_text);

				for attachment in self.extract_attachments(&email_content) {
					let (filename, content) = attachment;
					let file_extension = Path::new(&filename)
						.extension()
						.and_then(|ext| ext.to_str())
						.map(|ext| ext.to_string())
						.unwrap_or_else(|| String::new());

					let cursor = Cursor::new(content.to_vec());
					collected_messages.push(CollectedBytes {
						data: Some(Box::pin(cursor)),
						file: Some(PathBuf::from(filename.clone())),
						source_id: self.source_id.clone(),
						eof: true,
						doc_source: Some(format!("email://{}", filename)),
						extension: Some(file_extension),
						size: Some(content.len()),
						_owned_permit: None,
						image_id: None,
					});
				}

				let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
				let cursor = Cursor::new(combined_text.as_bytes().to_vec());
				collected_messages.push(CollectedBytes {
					source_id: self.source_id.clone(),
					data: Some(Box::pin(cursor)),
					file: Some(PathBuf::from(format!("{}.email", message_id))),
					eof: true,
					doc_source: Some("email://unknown_sender".to_string()),
					extension: Some("txt".to_string()),
					size: Some(body.len() as usize),
					_owned_permit: None,
					image_id: None,
				});
			}
		}

		let message_streams = collected_messages
			.into_iter()
			.map(|message| stream::once(async { Ok(message) }).boxed())
			.collect::<Vec<_>>();

		let stream = Box::pin(stream::iter(message_streams).flatten());

		Ok(stream)
	}
}

#[cfg(test)]
mod tests {

	use std::{collections::HashSet, env};

	use super::*;
	use dotenv::dotenv;

	#[tokio::test]
	async fn test_email_collector() {
		dotenv().ok();
		// Configure the Email collector config with a mock credential
		let email_config = EmailCollectorConfig {
			imap_server: "imap.gmail.com".to_string(),
			imap_port: 993,
			imap_username: env::var("IMAP_USERNAME").unwrap_or_else(|_| "".to_string()),
			imap_password: env::var("IMAP_PASSWORD").unwrap_or_else(|_| "".to_string()),
			imap_folder: "[Gmail]/Drafts".to_string(),
			id: "Email-source-id".to_string(),
		};

		let email_source = EmailSource::new(email_config).await.unwrap();

		assert!(
			email_source.check_connectivity().await.is_ok(),
			"Failed to connect to email source"
		);

		let result = email_source.poll_data().await;

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
		println!("Files are --- {:?}", count_files);
	}
}
