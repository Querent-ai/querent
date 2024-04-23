use std::{
	io,
	pin::Pin,
	task::{Context, Poll},
};

use once_cell::sync::Lazy;
use tokio::{
	io::{AsyncRead, ReadBuf},
	sync::Semaphore,
};

static REQUEST_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1000));

pub struct CollectedBytes<T: AsyncRead + Send + Unpin> {
	data: Option<T>,
	error: Option<String>,
	file: String,
	eof: bool,
	doc_source: String,
	extension: String,
	file_id: String,
	pub _permit: Result<tokio::sync::SemaphorePermit<'static>, tokio::sync::AcquireError>,
}

impl<T: AsyncRead + Send + Unpin> CollectedBytes<T> {
	pub fn new(
		file: String,
		data: Option<T>,
		error: Option<String>,
		eof: bool,
		doc_source: String,
	) -> Self {
		let mut extension = String::new();
		let mut file_id = String::new();
		if !file.is_empty() {
			let file_str = file.clone();
			let parts: Vec<&str> = file_str.split('.').collect();
			extension = parts.last().unwrap_or(&"").to_string();
			let file_parts: Vec<&str> = file_str.split('/').collect();
			file_id = file_parts.last().unwrap_or(&"").split('.').next().unwrap_or(&"").to_string();
		}
		CollectedBytes {
			data,
			error,
			file,
			eof,
			doc_source,
			extension,
			file_id,
			_permit: REQUEST_SEMAPHORE.clone().try_acquire(),
		}
	}

	pub fn is_error(&self) -> bool {
		self.error.is_some()
	}

	pub fn is_eof(&self) -> bool {
		self.eof
	}

	pub fn get_file_path(&self) -> &str {
		&self.file
	}

	pub fn get_extension(&self) -> &str {
		&self.extension
	}

	pub fn get_file_id(&self) -> &str {
		&self.file_id
	}

	pub fn success(data: Option<Box<dyn AsyncRead + Send + Sync + Unpin>>) -> Self {
		CollectedBytes::new(String::new(), data, None, false, String::new())
	}

	pub fn error(error: String) -> Self {
		CollectedBytes::new(
			String::new(),
			Some(Box::new(tokio::io::empty())),
			Some(error),
			false,
			String::new(),
		)
	}

	pub fn unwrap(self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
		match self.error {
			Some(error) => panic!("{}", error),
			None => self.data.unwrap(),
		}
	}

	pub fn unwrap_or(
		self,
		default: Box<dyn AsyncRead + Send + Sync + Unpin>,
	) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
		match self.error {
			Some(_) => default,
			None => self.data.unwrap(),
		}
	}
}

impl<T: AsyncRead + Send + Unpin> AsyncRead for CollectedBytes<T> {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		let self_unpin = self.get_mut();
		if self_unpin.data.is_none() {
			return Poll::Ready(Ok(()));
		}
		Pin::new(&mut self_unpin.data.unwrap()).poll_read(cx, buf)
	}
}
