// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

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
	pub _owned_permit: Option<tokio::sync::OwnedSemaphorePermit>,
	pub image_id: Option<String>,
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
		CollectedBytes {
			data,
			file,
			eof,
			doc_source,
			extension,
			size,
			source_id,
			_owned_permit: _permit,
			image_id: None,
		}
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
