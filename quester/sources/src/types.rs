use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CollectedBytes {
	data: Vec<u8>,
	error: Option<String>,
	file: String,
	eof: bool,
	doc_source: String,
	extension: String,
	file_id: String,
}

impl CollectedBytes {
	pub fn new(
		file: String,
		data: Vec<u8>,
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
		CollectedBytes { data, error, file, eof, doc_source, extension, file_id }
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

	pub fn success(data: Vec<u8>) -> Self {
		CollectedBytes::new(String::new(), data, None, false, String::new())
	}

	pub fn error(error: String) -> Self {
		CollectedBytes::new(String::new(), Vec::new(), Some(error), false, String::new())
	}

	pub fn unwrap(self) -> Vec<u8> {
		match self.error {
			Some(error) => panic!("{}", error),
			None => self.data,
		}
	}

	pub fn unwrap_or(self, default: Vec<u8>) -> Vec<u8> {
		match self.error {
			Some(_) => default,
			None => self.data,
		}
	}
}
