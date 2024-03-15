/// CollectorConfig is a message to hold configuration for a collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CollectorConfig {
    /// Name of the collector.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Backend to be used by the collector.
    #[prost(message, optional, tag = "2")]
    pub backend: ::core::option::Option<SupportedSources>,
    /// Additional configuration for the collector.
    #[prost(map = "string, string", tag = "3")]
    pub config: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// SupportedSources is a message to hold supported sources for collectors.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SupportedSources {
    /// Azure configuration.
    #[prost(message, optional, tag = "1")]
    pub azure: ::core::option::Option<AzureCollectorConfig>,
    /// GCS configuration.
    #[prost(message, optional, tag = "2")]
    pub gcs: ::core::option::Option<GcsCollectorConfig>,
    /// S3 configuration.
    #[prost(message, optional, tag = "3")]
    pub s3: ::core::option::Option<S3CollectorConfig>,
    /// Jira configuration.
    #[prost(message, optional, tag = "4")]
    pub jira: ::core::option::Option<JiraCollectorConfig>,
    /// Google Drive configuration.
    #[prost(message, optional, tag = "5")]
    pub drive: ::core::option::Option<GoogleDriveCollectorConfig>,
    /// Email configuration.
    #[prost(message, optional, tag = "6")]
    pub email: ::core::option::Option<EmailCollectorConfig>,
    /// DropBox configuration.
    #[prost(message, optional, tag = "7")]
    pub dropbox: ::core::option::Option<DropBoxCollectorConfig>,
    /// Github configuration.
    #[prost(message, optional, tag = "8")]
    pub github: ::core::option::Option<GithubCollectorConfig>,
    /// Slack configuration.
    #[prost(message, optional, tag = "9")]
    pub slack: ::core::option::Option<SlackCollectorConfig>,
    /// News configuration.
    #[prost(message, optional, tag = "10")]
    pub news: ::core::option::Option<NewsCollectorConfig>,
}
/// AzureCollectorConfig is a message to hold configuration for an Azure collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AzureCollectorConfig {
    /// Account URL of the Azure collector.
    #[prost(string, tag = "1")]
    pub account_url: ::prost::alloc::string::String,
    /// Connection string of the Azure collector.
    #[prost(string, tag = "2")]
    pub connection_string: ::prost::alloc::string::String,
    /// Container of the Azure collector.
    #[prost(string, tag = "3")]
    pub container: ::prost::alloc::string::String,
    /// Credentials of the Azure collector.
    #[prost(string, tag = "4")]
    pub credentials: ::prost::alloc::string::String,
    /// Prefix of the Azure collector.
    #[prost(string, tag = "5")]
    pub prefix: ::prost::alloc::string::String,
    /// Chunk size of the Azure collector.
    #[prost(int64, tag = "6")]
    pub chunk_size: i64,
}
/// GCSCollectorConfig is a message to hold configuration for a GCS collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GcsCollectorConfig {
    /// Bucket of the GCS collector.
    #[prost(string, tag = "1")]
    pub bucket: ::prost::alloc::string::String,
    /// Credentials of the GCS collector.
    #[prost(string, tag = "2")]
    pub credentials: ::prost::alloc::string::String,
}
/// S3CollectorConfig is a message to hold configuration for an S3 collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S3CollectorConfig {
    /// Access key of the S3 collector.
    #[prost(string, tag = "1")]
    pub access_key: ::prost::alloc::string::String,
    /// Secret key of the S3 collector.
    #[prost(string, tag = "2")]
    pub secret_key: ::prost::alloc::string::String,
    /// Region of the S3 collector.
    #[prost(string, tag = "3")]
    pub region: ::prost::alloc::string::String,
    /// Bucket of the S3 collector.
    #[prost(string, tag = "4")]
    pub bucket: ::prost::alloc::string::String,
}
/// JiraCollectorConfig is a message to hold configuration for a Jira collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JiraCollectorConfig {
    /// Server of the Jira collector.
    #[prost(string, tag = "1")]
    pub jira_server: ::prost::alloc::string::String,
    /// Username of the Jira collector.
    #[prost(string, tag = "2")]
    pub jira_username: ::prost::alloc::string::String,
    /// Password of the Jira collector.
    #[prost(string, tag = "3")]
    pub jira_password: ::prost::alloc::string::String,
    /// API token of the Jira collector.
    #[prost(string, tag = "4")]
    pub jira_api_token: ::prost::alloc::string::String,
    /// Certificate file of the Jira collector.
    #[prost(string, tag = "5")]
    pub jira_certfile: ::prost::alloc::string::String,
    /// Key file of the Jira collector.
    #[prost(string, tag = "6")]
    pub jira_keyfile: ::prost::alloc::string::String,
    /// Verify of the Jira collector.
    #[prost(bool, tag = "7")]
    pub jira_verify: bool,
    /// Project of the Jira collector.
    #[prost(string, tag = "8")]
    pub jira_project: ::prost::alloc::string::String,
    /// Query of the Jira collector.
    #[prost(string, tag = "9")]
    pub jira_query: ::prost::alloc::string::String,
    /// Start at of the Jira collector.
    #[prost(int32, tag = "10")]
    pub jira_start_at: i32,
    /// Max results of the Jira collector.
    #[prost(int32, tag = "11")]
    pub jira_max_results: i32,
}
/// GoogleDriveCollectorConfig is a message to hold configuration for a Google Drive collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GoogleDriveCollectorConfig {
    /// Client ID of the Google Drive collector.
    #[prost(string, tag = "1")]
    pub drive_client_id: ::prost::alloc::string::String,
    /// Client secret of the Google Drive collector.
    #[prost(string, tag = "2")]
    pub drive_client_secret: ::prost::alloc::string::String,
    /// Refresh token of the Google Drive collector.
    #[prost(string, tag = "3")]
    pub drive_refresh_token: ::prost::alloc::string::String,
    /// Scopes of the Google Drive collector.
    #[prost(string, tag = "4")]
    pub drive_scopes: ::prost::alloc::string::String,
    /// Token of the Google Drive collector.
    #[prost(string, tag = "5")]
    pub drive_token: ::prost::alloc::string::String,
    /// Folder to crawl of the Google Drive collector.
    #[prost(string, tag = "6")]
    pub folder_to_crawl: ::prost::alloc::string::String,
    /// Specific file type of the Google Drive collector.
    #[prost(string, tag = "7")]
    pub specific_file_type: ::prost::alloc::string::String,
}
/// EmailCollectorConfig is a message to hold configuration for an Email collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmailCollectorConfig {
    /// Server of the Email collector.
    #[prost(string, tag = "1")]
    pub imap_server: ::prost::alloc::string::String,
    /// Port of the Email collector.
    #[prost(int32, tag = "2")]
    pub imap_port: i32,
    /// Username of the Email collector.
    #[prost(string, tag = "3")]
    pub imap_username: ::prost::alloc::string::String,
    /// Password of the Email collector.
    #[prost(string, tag = "4")]
    pub imap_password: ::prost::alloc::string::String,
    /// Folder of the Email collector.
    #[prost(string, tag = "5")]
    pub imap_folder: ::prost::alloc::string::String,
    /// Key file of the Email collector.
    #[prost(string, tag = "6")]
    pub imap_keyfile: ::prost::alloc::string::String,
    /// Cert file of the Email collector.
    #[prost(string, tag = "7")]
    pub imap_certfile: ::prost::alloc::string::String,
}
/// DropBoxCollectorConfig is a message to hold configuration for a DropBox collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DropBoxCollectorConfig {
    /// App key of the DropBox collector.
    #[prost(string, tag = "1")]
    pub dropbox_app_key: ::prost::alloc::string::String,
    /// App secret of the DropBox collector.
    #[prost(string, tag = "2")]
    pub dropbox_app_secret: ::prost::alloc::string::String,
    /// Refresh token of the DropBox collector.
    #[prost(string, tag = "3")]
    pub dropbox_refresh_token: ::prost::alloc::string::String,
    /// Folder path of the DropBox collector.
    #[prost(string, tag = "4")]
    pub folder_path: ::prost::alloc::string::String,
}
/// GithubCollectorConfig is a message to hold configuration for a Github collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GithubCollectorConfig {
    /// Username of the Github collector.
    #[prost(string, tag = "1")]
    pub github_username: ::prost::alloc::string::String,
    /// Access token of the Github collector.
    #[prost(string, tag = "2")]
    pub github_access_token: ::prost::alloc::string::String,
    /// Repository of the Github collector.
    #[prost(string, tag = "3")]
    pub repository: ::prost::alloc::string::String,
}
/// SlackCollectorConfig is a message to hold configuration for a Slack collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SlackCollectorConfig {
    /// Access token of the Slack collector.
    #[prost(string, tag = "1")]
    pub access_token: ::prost::alloc::string::String,
    /// Channel name of the Slack collector.
    #[prost(string, tag = "2")]
    pub channel_name: ::prost::alloc::string::String,
    /// Cursor of the Slack collector.
    #[prost(string, tag = "3")]
    pub cursor: ::prost::alloc::string::String,
    /// Include all metadata of the Slack collector.
    #[prost(bool, tag = "4")]
    pub include_all_metadata: bool,
    /// Includive of the Slack collector.
    #[prost(bool, tag = "5")]
    pub includive: bool,
    /// Limit of the Slack collector.
    #[prost(int64, tag = "6")]
    pub limit: i64,
}
/// NewsCollectorConfig is a message to hold configuration for a News collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NewsCollectorConfig {
    /// API key of the News collector.
    #[prost(string, tag = "1")]
    pub api_key: ::prost::alloc::string::String,
    /// Query of the News collector.
    #[prost(string, tag = "2")]
    pub query: ::prost::alloc::string::String,
    /// From date of the News collector.
    #[prost(string, tag = "3")]
    pub from_date: ::prost::alloc::string::String,
    /// To date of the News collector.
    #[prost(string, tag = "4")]
    pub to_date: ::prost::alloc::string::String,
    /// Language of the News collector.
    #[prost(string, tag = "5")]
    pub language: ::prost::alloc::string::String,
    /// Domains of the News collector.
    #[prost(string, tag = "6")]
    pub domains: ::prost::alloc::string::String,
    /// Exclude domains of the News collector.
    #[prost(string, tag = "7")]
    pub exclude_domains: ::prost::alloc::string::String,
    /// Sources of the News collector.
    #[prost(string, tag = "8")]
    pub sources: ::prost::alloc::string::String,
}
