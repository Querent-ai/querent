use std::{fmt::Debug, path::PathBuf, pin::Pin};

use tokio::io::AsyncRead;

pub struct CollectedBytes {
	pub data: Option<Pin<Box<dyn AsyncRead + Send>>>,
	pub file: Option<PathBuf>,
	pub eof: bool,
	pub doc_source: Option<String>,
	pub extension: Option<String>,
	pub size: Option<usize>,
	pub source_id: String,
}

impl Debug for CollectedBytes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CollectedBytes")
			.field("file", &self.file)
			.field("eof", &self.eof)
			.field("doc_source", &self.doc_source)
			.field("extension", &self.extension)
			.field("size", &self.size)
			.field("source_id", &self.source_id)
			.finish()
	}
}

impl CollectedBytes {
	pub fn new(
		file: Option<PathBuf>,
		data: Option<Pin<Box<dyn AsyncRead + Send>>>,
		eof: bool,
		doc_source: Option<String>,
		size: Option<usize>,
		source_id: String,
		_permit: Option<tokio::sync::OwnedSemaphorePermit>,
	) -> Self {
		let extension = file
			.as_ref()
			.and_then(|file| file.extension().and_then(|ext| ext.to_str().map(String::from)));
		CollectedBytes { data, file, eof, doc_source, extension, size, source_id }
	}

	pub fn is_eof(&self) -> bool {
		self.eof
	}

	pub fn get_file_path(&self) -> Option<&PathBuf> {
		self.file.as_ref()
	}

	pub fn get_extension(&self) -> Option<&String> {
		self.extension.as_ref()
	}

	pub fn unwrap(self) -> Pin<Box<dyn AsyncRead + Send>> {
		match self.data {
			Some(data) => data,
			None => panic!("Tried to unwrap an error CollectedBytes"),
		}
	}
}
