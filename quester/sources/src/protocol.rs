use std::{
	borrow::Cow,
	env,
	ffi::OsStr,
	fmt::{Debug, Display},
	hash::Hash,
	path::{Component, Path, PathBuf},
	str::FromStr,
};

use anyhow::{bail, Context};
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{de::Error, Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Protocol {
	Azure = 1,
	File = 2,
	Gcs = 3,
	PostgreSQL = 4,
	Ram = 5,
	S3 = 6,
	Webscraper = 7,
	Slack = 8,
	DropBox = 9,
	Github = 10,
	Drive = 11,
	Email = 12,
	Jira = 13,
	News = 14,
}

impl Protocol {
	pub fn as_str(&self) -> &str {
		match self {
			Protocol::Azure => "azure",
			Protocol::File => "file",
			Protocol::Gcs => "gcs",
			Protocol::PostgreSQL => "postgres",
			Protocol::Ram => "ram",
			Protocol::S3 => "s3",
			Protocol::Webscraper => "webscraper",
			Protocol::Slack => "slack",
			Protocol::DropBox => "dropbox",
			Protocol::Github => "github",
			Protocol::Drive => "drive",
			Protocol::Email => "email",
			Protocol::Jira => "jira",
			Protocol::News => "news",
		}
	}

	pub fn is_file(&self) -> bool {
		matches!(&self, Protocol::File)
	}

	pub fn is_gcs(&self) -> bool {
		matches!(&self, Protocol::Gcs)
	}

	pub fn is_s3(&self) -> bool {
		matches!(&self, Protocol::S3)
	}

	pub fn is_azure(&self) -> bool {
		matches!(&self, Protocol::Azure)
	}

	pub fn is_ram(&self) -> bool {
		matches!(&self, Protocol::Ram)
	}

	pub fn is_database(&self) -> bool {
		matches!(&self, Protocol::PostgreSQL)
	}

	pub fn is_web_scraper(&self) -> bool {
		matches!(&self, Protocol::Webscraper)
	}

	pub fn is_slack(&self) -> bool {
		matches!(&self, Protocol::Slack)
	}

	pub fn is_dropbox(&self) -> bool {
		matches!(&self, Protocol::DropBox)
	}

	pub fn is_github(&self) -> bool {
		matches!(&self, Protocol::Github)
	}

	pub fn is_drive(&self) -> bool {
		matches!(&self, Protocol::Drive)
	}

	pub fn is_email(&self) -> bool {
		matches!(&self, Protocol::Email)
	}

	pub fn is_jira(&self) -> bool {
		matches!(&self, Protocol::Jira)
	}

	pub fn is_news(&self) -> bool {
		matches!(&self, Protocol::News)
	}

	pub fn is_supported(&self) -> bool {
		matches!(
			&self,
			Protocol::Azure |
				Protocol::File | Protocol::Gcs |
				Protocol::PostgreSQL |
				Protocol::Ram | Protocol::S3 |
				Protocol::Webscraper |
				Protocol::Slack | Protocol::DropBox |
				Protocol::Github | Protocol::Drive |
				Protocol::Email | Protocol::Jira |
				Protocol::News
		)
	}

	pub fn is_not_supported(&self) -> bool {
		!self.is_supported()
	}
}

impl Display for Protocol {
	fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

impl FromStr for Protocol {
	type Err = anyhow::Error;

	fn from_str(protocol: &str) -> anyhow::Result<Self> {
		match protocol {
			"azure" => Ok(Protocol::Azure),
			"file" => Ok(Protocol::File),
			"gcs" => Ok(Protocol::Gcs),
			"postgres" => Ok(Protocol::PostgreSQL),
			"ram" => Ok(Protocol::Ram),
			"s3" => Ok(Protocol::S3),
			"webscraper" => Ok(Protocol::Webscraper),
			"slack" => Ok(Protocol::Slack),
			"dropbox" => Ok(Protocol::DropBox),
			"github" => Ok(Protocol::Github),
			"drive" => Ok(Protocol::Drive),
			"email" => Ok(Protocol::Email),
			"jira" => Ok(Protocol::Jira),
			"news" => Ok(Protocol::News),
			_ => bail!("Unsupported protocol: {}", protocol),
		}
	}
}

const PROTOCOL_SEPARATOR: &str = "://";

/// Encapsulates the URI type.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Uri {
	pub uri: String,
	pub protocol: Protocol,
}

impl Uri {
	/// Returns the extension of the URI.
	pub fn extension(&self) -> Option<&str> {
		Path::new(&self.uri).extension().and_then(OsStr::to_str)
	}

	/// Returns the URI as a string slice.
	pub fn as_str(&self) -> &str {
		&self.uri
	}

	/// Returns the protocol of the URI.
	pub fn protocol(&self) -> Protocol {
		self.protocol
	}

	/// Strips sensitive information such as credentials from URI.
	fn as_redacted_str(&self) -> Cow<str> {
		if self.protocol().is_database() {
			static DATABASE_URI_PATTERN: OnceCell<Regex> = OnceCell::new();
			DATABASE_URI_PATTERN
				.get_or_init(|| {
					Regex::new("(?P<before>^.*://.*)(?P<password>:.*@)(?P<after>.*)")
						.expect("regular expression should compile")
				})
				.replace(&self.uri, "$before:***redacted***@$after")
		} else {
			Cow::Borrowed(&self.uri)
		}
	}

	pub fn redact(&mut self) {
		self.uri = self.as_redacted_str().into_owned();
	}

	/// Consumes the [`Uri`] struct and returns the normalized URI as a string.
	pub fn into_string(self) -> String {
		self.uri
	}

	fn parse_str(uri_str: &str) -> anyhow::Result<Self> {
		// CAUTION: Do not display the URI in error messages to avoid leaking credentials.
		if uri_str.is_empty() {
			bail!("failed to parse empty URI");
		}
		let (protocol, mut path) = match uri_str.split_once(PROTOCOL_SEPARATOR) {
			None => (Protocol::File, uri_str.to_string()),
			Some((protocol, path)) => (Protocol::from_str(protocol)?, path.to_string()),
		};

		if protocol == Protocol::File {
			if path.starts_with('~') {
				// We only accept `~` (alias to the home directory) and `~/path/to/something`.
				// If there is something following the `~` that is not `/`, we bail.
				if path.len() > 1 && !path.starts_with("~/") {
					bail!("failed to normalize URI: tilde expansion is only partially supported");
				}

				let home_dir_path = home::home_dir()
					.context("failed to normalize URI: could not resolve home directory")?
					.to_string_lossy()
					.to_string();

				path.replace_range(0..1, &home_dir_path);
			}
			if Path::new(&path).is_relative() {
				let current_dir = env::current_dir().context(
					"failed to normalize URI: could not resolve current working directory. the \
                     directory does not exist or user has insufficient permissions",
				)?;
				path = current_dir.join(path).to_string_lossy().to_string();
			}
			path = normalize_path(Path::new(&path)).to_string_lossy().to_string();
		}
		Ok(Self { uri: format!("{protocol}{PROTOCOL_SEPARATOR}{path}"), protocol })
	}
}

impl AsRef<str> for Uri {
	fn as_ref(&self) -> &str {
		&self.uri
	}
}

impl Debug for Uri {
	fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.debug_struct("Uri").field("uri", &self.as_redacted_str()).finish()
	}
}

impl Display for Uri {
	fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(formatter, "{}", self.as_redacted_str())
	}
}

impl FromStr for Uri {
	type Err = anyhow::Error;

	fn from_str(uri_str: &str) -> anyhow::Result<Self> {
		Uri::parse_str(uri_str)
	}
}

impl PartialEq<&str> for Uri {
	fn eq(&self, other: &&str) -> bool {
		&self.uri == other
	}
}

impl PartialEq<String> for Uri {
	fn eq(&self, other: &String) -> bool {
		&self.uri == other
	}
}

impl<'de> Deserialize<'de> for Uri {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let uri_str: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
		let uri = Uri::from_str(&uri_str).map_err(D::Error::custom)?;
		Ok(uri)
	}
}

impl Serialize for Uri {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.uri)
	}
}

/// Normalizes a path by resolving the components like (., ..).
/// This helper does the same thing as `Path::canonicalize`.
/// It only differs from `Path::canonicalize` by not checking file existence
/// during resolution.
/// <https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61>
fn normalize_path(path: &Path) -> PathBuf {
	let mut components = path.components().peekable();
	let mut resulting_path_buf =
		if let Some(component @ Component::Prefix(..)) = components.peek().cloned() {
			components.next();
			PathBuf::from(component.as_os_str())
		} else {
			PathBuf::new()
		};

	for component in components {
		match component {
			Component::Prefix(..) => unreachable!(),
			Component::RootDir => {
				resulting_path_buf.push(component.as_os_str());
			},
			Component::CurDir => {},
			Component::ParentDir => {
				resulting_path_buf.pop();
			},
			Component::Normal(inner_component) => {
				resulting_path_buf.push(inner_component);
			},
		}
	}
	resulting_path_buf
}
