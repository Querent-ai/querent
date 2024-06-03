use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedBytes {
	pub data: Option<Vec<u8>>,
	pub file: Option<PathBuf>,
	pub eof: bool,
	pub doc_source: Option<String>,
	pub extension: Option<String>,
	pub size: Option<usize>,
}

impl CollectedBytes {
	pub fn new(
		file: Option<PathBuf>,
		data: Option<Vec<u8>>,
		eof: bool,
		doc_source: Option<String>,
		size: Option<usize>,
	) -> Self {
		let extension = file
			.as_ref()
			.and_then(|file| file.extension().and_then(|ext| ext.to_str().map(String::from)));
		CollectedBytes { data, file, eof, doc_source, extension, size }
	}

	pub fn success(data: Vec<u8>) -> Self {
		CollectedBytes::new(None, Some(data), false, None, None)
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

	pub fn unwrap(self) -> Vec<u8> {
		match self.data {
			Some(data) => data,
			None => panic!("Tried to unwrap an error CollectedBytes"),
		}
	}

	pub fn unwrap_or(self, default: Vec<u8>) -> Vec<u8> {
		match self.data {
			Some(data) => data,
			None => default,
		}
	}
}
