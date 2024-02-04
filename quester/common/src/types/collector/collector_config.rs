use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct AzureCollectorConfig {
	pub connection_string: String,
	pub account_url: String,
	pub credentials: String,
	pub container: String,
	pub chunk_size: Option<u64>,
	pub prefix: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct S3CollectorConfig {
	pub access_key: String,
	pub secret_key: String,
	pub region: String,
	pub bucket: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct GCSCollectorConfig {
	pub credentials: String,
	pub bucket: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct SlackCollectorConfig {
	pub access_token: String,
	pub channel_name: String,
	pub cursor: Option<String>,
	pub include_all_metadata: Option<bool>,
	pub includive: Option<bool>,
	pub limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct DropBoxCollectorConfig {
	pub dropbox_app_key: String,
	pub dropbox_app_secret: String,
	pub folder_path: String,
	pub dropbox_refresh_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct GithubCollectorConfig {
	pub github_username: String,
	pub github_access_token: String,
	pub repository: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct GoogleDriveCollectorConfig {
	pub drive_refresh_token: String,
	pub drive_token: String,
	pub drive_scopes: String,
	pub drive_client_id: String,
	pub drive_client_secret: String,
	pub folder_to_crawl: Option<String>,
	pub specific_file_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct EmailCollectorConfig {
	pub imap_server: String,
	pub imap_port: u16,
	pub imap_username: String,
	pub imap_password: String,
	pub imap_folder: String,
	pub imap_keyfile: Option<String>,
	pub imap_certfile: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema, PartialEq)]
#[pyclass]
pub struct JiraCollectorConfig {
	pub jira_server: String,
	pub jira_username: String,
	pub jira_project: String,
	pub jira_query: String,
	pub jira_password: Option<String>,
	pub jira_api_token: Option<String>,
	pub jira_start_at: Option<u16>,
	pub jira_max_results: Option<u16>,
	pub jira_keyfile: Option<String>,
	pub jira_certfile: Option<String>,
	pub jira_verify: Option<bool>,
}
