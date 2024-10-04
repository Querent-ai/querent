use async_trait::async_trait;
use postgresql_archive::{
	extractor::{zip_extract, ExtractDirectories},
	get_archive,
	repository::github::repository::GitHub,
};
use postgresql_extensions::{repository::Repository, zip_matcher, AvailableExtension, Result};
use regex::Regex;
use semver::{Version, VersionReq};
use std::{fmt::Debug, path::PathBuf};

pub const URL: &str = "https://github.com/querent-ai";

/// QuerentAI repository.
#[derive(Debug)]
pub struct QuerentAI;

impl QuerentAI {
	/// Creates a new TensorChord repository.
	///
	/// # Errors
	/// * If the repository cannot be created
	#[allow(clippy::new_ret_no_self)]
	pub fn new() -> Result<Box<dyn Repository>> {
		Ok(Box::new(Self))
	}

	/// Initializes the repository.
	///
	/// # Errors
	/// * If the repository cannot be initialized.
	pub fn initialize() -> Result<()> {
		postgresql_archive::matcher::registry::register(
			|url| Ok(url.starts_with(URL)),
			zip_matcher,
		)?;
		postgresql_archive::repository::registry::register(
			|url| Ok(url.starts_with(URL)),
			Box::new(GitHub::new),
		)?;
		Ok(())
	}
}

#[async_trait]
impl Repository for QuerentAI {
	fn name(&self) -> &str {
		"querent-ai"
	}

	async fn get_available_extensions(&self) -> Result<Vec<AvailableExtension>> {
		let extensions = vec![AvailableExtension::new(
			self.name(),
			"pgvecto.win",
			"Scalable, Low-latency and Hybrid-enabled Vector Search",
		)];
		Ok(extensions)
	}

	async fn get_archive(
		&self,
		postgresql_version: &str,
		name: &str,
		version: &VersionReq,
	) -> Result<(Version, Vec<u8>)> {
		let url = format!("{URL}/{name}?postgresql_version={postgresql_version}");
		let archive = get_archive(url.as_str(), version).await?;
		Ok(archive)
	}

	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	async fn install(
		&self,
		_name: &str,
		library_dir: PathBuf,
		extension_dir: PathBuf,
		archive: &[u8],
	) -> Result<Vec<PathBuf>> {
		let mut extract_directories = ExtractDirectories::default();
		extract_directories.add_mapping(Regex::new(r"\.(dll|dylib|so|lib|exp)$")?, library_dir);
		extract_directories.add_mapping(Regex::new(r"\.(control|sql)$")?, extension_dir);
		let bytes = &archive.to_vec();
		let files = zip_extract(bytes, extract_directories)?;
		Ok(files)
	}
}
