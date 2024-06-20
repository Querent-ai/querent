pub mod source;
use std::{
	io::{self, ErrorKind},
	path::{Path, PathBuf},
};
use tempfile::TempPath;
use tracing::error;

pub use source::*;
use tokio::fs::File;
pub mod azure;
pub mod drive;
pub mod email;
pub mod filesystem;
pub mod gcs;
pub mod s3;
pub mod onedrive;

async fn default_copy_to_file<S: Source + ?Sized>(
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
