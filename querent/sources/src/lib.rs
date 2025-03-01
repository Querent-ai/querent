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

pub mod source;
use std::{
	io::{self, Cursor, ErrorKind},
	path::{Path, PathBuf},
};
use tempfile::TempPath;
use tokio::io::AsyncRead;
use tracing::error;

pub use source::*;
use tokio::fs::File;
pub mod azure;
pub mod drive;
pub mod email;
pub mod filesystem;
pub mod gcs;
pub mod jira;
pub mod news;
pub mod notion;
pub mod onedrive;
pub mod osdu;
pub mod s3;
pub mod sales_force;
pub mod slack;
pub mod zip;
use once_cell::sync::Lazy;
use tokio::sync::Semaphore;

static REQUEST_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1000));

async fn default_copy_to_file<S: DataSource + ?Sized>(
	storage: &S,
	path: &Path,
	output_path: &Path,
) -> SourceResult<u64> {
	let mut download_temp_file =
		DownloadTempFile::with_target_path(output_path.to_path_buf()).await?;
	storage.copy_to(path, download_temp_file.as_mut()).await?;
	let num_bytes = download_temp_file.persist().await?;
	Ok(num_bytes)
}

struct DownloadTempFile {
	target_filepath: PathBuf,
	temp_filepath: PathBuf,
	file: File,
	has_attempted_deletion: bool,
}

impl DownloadTempFile {
	/// Creates or truncate temp file.
	pub async fn with_target_path(target_filepath: PathBuf) -> io::Result<DownloadTempFile> {
		let Some(filename) = target_filepath.file_name() else {
			return Err(io::Error::new(
				ErrorKind::Other,
				"Target filepath is not a directory path. Expected a filepath.",
			));
		};
		let filename: &str = filename.to_str().ok_or_else(|| {
			io::Error::new(ErrorKind::Other, "Target filepath is not a valid UTF-8 string.")
		})?;
		let mut temp_filepath = target_filepath.clone();
		temp_filepath.set_file_name(format!("{filename}.temp"));
		let file = tokio::fs::File::create(temp_filepath.clone()).await?;
		Ok(DownloadTempFile { target_filepath, temp_filepath, file, has_attempted_deletion: false })
	}

	pub async fn persist(mut self) -> io::Result<u64> {
		TempPath::from_path(&self.temp_filepath).persist(&self.target_filepath)?;
		self.has_attempted_deletion = true;
		let num_bytes = std::fs::metadata(&self.target_filepath)?.len();
		Ok(num_bytes)
	}
}

impl Drop for DownloadTempFile {
	fn drop(&mut self) {
		if self.has_attempted_deletion {
			return;
		}
		let temp_filepath = self.temp_filepath.clone();
		self.has_attempted_deletion = true;
		tokio::task::spawn_blocking(move || {
			if let Err(io_error) = std::fs::remove_file(&temp_filepath) {
				error!(temp_filepath=%temp_filepath.display(), io_error=?io_error, "Failed to remove temporary file");
			}
		});
	}
}

impl AsMut<File> for DownloadTempFile {
	fn as_mut(&mut self) -> &mut File {
		&mut self.file
	}
}

pub fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

pub fn resolve_ingestor_with_extension(extension: &str) -> SourceResult<String> {
	let programming_languages = vec![
		"py", "pyw", "pyp", "js", "mjs", "java", "cpp", "h", "hpp", "c", "cs", "rb", "swift",
		"php", "php3", "php4", "php5", "phtml", "html", "htm", "css", "go", "rs", "kt", "ts", "pl",
		"sql", "r", "m", "sh", "bash", "zsh", "dart", "scala", "groovy", "lua", "vb", "pdf", "csv",
		"xml", "docx", "doc", "jpeg", "jpg", "png", "json", "pptx", "odp", "dds", "news",
	];

	if let Some(found) = programming_languages.iter().find(|&&ext| extension.contains(ext)) {
		return Ok(found.to_string());
	}

	Ok("".to_string())
}
