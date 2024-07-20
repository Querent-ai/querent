use std::path::PathBuf;

use crate::OwnedBytes;

#[derive(Debug, Clone)]
pub struct CollectedBytes {
	pub data: Option<OwnedBytes>,
	pub file: Option<PathBuf>,
	pub eof: bool,
	pub doc_source: Option<String>,
	pub extension: Option<String>,
	pub size: Option<usize>,
	pub source_id: String,
}

impl CollectedBytes {
	pub fn new(
		file: Option<PathBuf>,
		data: Option<OwnedBytes>,
		eof: bool,
		doc_source: Option<String>,
		size: Option<usize>,
		source_id: String,
	) -> Self {
		let extension = file
			.as_ref()
			.and_then(|file| file.extension().and_then(|ext| ext.to_str().map(String::from)));
		CollectedBytes { data, file, eof, doc_source, extension, size, source_id }
	}

	pub fn success(data: OwnedBytes) -> Self {
		CollectedBytes::new(None, Some(data), false, None, None, "".to_string())
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

	pub fn unwrap(self) -> OwnedBytes {
		match self.data {
			Some(data) => data,
			None => panic!("Tried to unwrap an error CollectedBytes"),
		}
	}

	pub fn unwrap_or(self, default: Vec<u8>) -> OwnedBytes {
		match self.data {
			Some(data) => data,
			None => OwnedBytes::new(default),
		}
	}
}
