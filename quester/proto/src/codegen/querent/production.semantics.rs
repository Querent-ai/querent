#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineRequestInfoList {
	#[prost(message, repeated, tag = "1")]
	pub requests: ::prost::alloc::vec::Vec<PipelineRequestInfo>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineRequestInfo {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
	#[prost(message, optional, tag = "2")]
	pub request: ::core::option::Option<SemanticPipelineRequest>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmptyObserve {}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmptyList {}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmptyGetPipelinesMetadata {}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopPipelineRequest {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DescribePipelineRequest {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RestartPipelineRequest {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CollectorConfigResponse {
	#[prost(string, tag = "1")]
	pub id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteCollectorRequest {
	#[prost(string, repeated, tag = "1")]
	pub id: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteCollectorResponse {
	#[prost(string, repeated, tag = "1")]
	pub id: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListCollectorRequest {}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListCollectorConfig {
	#[prost(message, repeated, tag = "1")]
	pub config: ::prost::alloc::vec::Vec<CollectorConfig>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SemanticPipelineRequest {
    #[prost(string, repeated, tag = "1")]
    pub collectors: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "2")]
    pub fixed_entities: ::core::option::Option<FixedEntities>,
    #[prost(message, optional, tag = "3")]
    pub sample_entities: ::core::option::Option<SampleEntities>,
    #[prost(enumeration = "Model", optional, tag = "4")]
    pub model: ::core::option::Option<i32>,
    #[prost(float, optional, tag = "5")]
    pub attention_threshold: ::core::option::Option<f32>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FixedEntities {
	#[prost(string, repeated, tag = "1")]
	pub entities: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SampleEntities {
	#[prost(string, repeated, tag = "1")]
	pub entities: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SemanticPipelineResponse {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SemanticServiceCounters {
	#[prost(int32, tag = "1")]
	pub num_running_pipelines: i32,
	#[prost(int32, tag = "2")]
	pub num_successful_pipelines: i32,
	#[prost(int32, tag = "3")]
	pub num_failed_pipelines: i32,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IngestedTokens {
	#[prost(string, tag = "1")]
	pub file: ::prost::alloc::string::String,
	#[prost(string, repeated, tag = "2")]
	pub data: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
	#[prost(bool, tag = "3")]
	pub is_token_stream: bool,
	#[prost(string, tag = "4")]
	pub doc_source: ::prost::alloc::string::String,
	#[prost(string, tag = "5")]
	pub source_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendIngestedTokens {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
	#[prost(message, repeated, tag = "2")]
	pub tokens: ::prost::alloc::vec::Vec<IngestedTokens>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IndexingStatistics {
	#[prost(uint64, tag = "1")]
	pub total_docs: u64,
	#[prost(uint64, tag = "2")]
	pub total_events: u64,
	#[prost(uint64, tag = "3")]
	pub total_events_processed: u64,
	#[prost(uint64, tag = "4")]
	pub total_events_received: u64,
	#[prost(uint64, tag = "5")]
	pub total_events_sent: u64,
	#[prost(uint64, tag = "6")]
	pub total_batches: u64,
	#[prost(uint64, tag = "7")]
	pub total_sentences: u64,
	#[prost(uint64, tag = "8")]
	pub total_subjects: u64,
	#[prost(uint64, tag = "9")]
	pub total_predicates: u64,
	#[prost(uint64, tag = "10")]
	pub total_objects: u64,
	#[prost(uint64, tag = "11")]
	pub total_graph_events: u64,
	#[prost(uint64, tag = "12")]
	pub total_vector_events: u64,
	#[prost(uint64, tag = "13")]
	pub total_data_processed_size: u64,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineMetadata {
	#[prost(string, tag = "1")]
	pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelinesMetadata {
	#[prost(message, repeated, tag = "1")]
	pub pipelines: ::prost::alloc::vec::Vec<PipelineMetadata>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BooleanResponse {
	#[prost(bool, tag = "1")]
	pub response: bool,
}
/// CollectorConfig is a message to hold configuration for a collector.
/// Defines a collector with a specific configuration.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CollectorConfig {
	#[prost(string, tag = "1")]
	pub name: ::prost::alloc::string::String,
	#[prost(oneof = "collector_config::Backend", tags = "2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12")]
	pub backend: ::core::option::Option<Backend>,
}
/// Nested message and enum types in `CollectorConfig`.
pub mod collector_config {
	use super::*;
	#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
	#[serde(rename_all = "snake_case")]
	#[allow(clippy::derive_partial_eq_without_eq)]
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Backend {
		#[prost(message, tag = "2")]
		Azure(AzureCollectorConfig),
		#[prost(message, tag = "3")]
		Gcs(GcsCollectorConfig),
		#[prost(message, tag = "4")]
		S3(S3CollectorConfig),
		#[prost(message, tag = "5")]
		Jira(JiraCollectorConfig),
		#[prost(message, tag = "6")]
		Drive(GoogleDriveCollectorConfig),
		#[prost(message, tag = "7")]
		Email(EmailCollectorConfig),
		#[prost(message, tag = "8")]
		Dropbox(DropBoxCollectorConfig),
		#[prost(message, tag = "9")]
		Github(GithubCollectorConfig),
		#[prost(message, tag = "10")]
		Slack(SlackCollectorConfig),
		#[prost(message, tag = "11")]
		News(NewsCollectorConfig),
		#[prost(message, tag = "12")]
		Files(FileCollectorConfig),
	}
}
/// FileCollectorConfig is a message to hold configuration for a file collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FileCollectorConfig {
	#[prost(string, tag = "1")]
	pub root_path: ::prost::alloc::string::String,
	/// Id for the collector
	#[prost(string, tag = "2")]
	pub id: ::prost::alloc::string::String,
}
/// AzureCollectorConfig is a message to hold configuration for an Azure collector.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AzureCollectorConfig {
	/// Connection string of the Azure collector.
	#[prost(string, tag = "1")]
	pub connection_string: ::prost::alloc::string::String,
	/// Container of the Azure collector.
	#[prost(string, tag = "2")]
	pub container: ::prost::alloc::string::String,
	/// Credentials of the Azure collector.
	#[prost(string, tag = "3")]
	pub credentials: ::prost::alloc::string::String,
	/// Prefix of the Azure collector.
	#[prost(string, tag = "4")]
	pub prefix: ::prost::alloc::string::String,
	/// Chunk size of the Azure collector.
	#[prost(int64, tag = "5")]
	pub chunk_size: i64,
	/// Id for the collector
	#[prost(string, tag = "7")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "3")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "5")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "12")]
	pub id: ::prost::alloc::string::String,
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
    /// Folder to crawl of the Google Drive collector.
    #[prost(string, tag = "4")]
    pub folder_to_crawl: ::prost::alloc::string::String,
    /// Specific file type of the Google Drive collector.
    #[prost(string, tag = "5")]
    pub specific_file_type: ::prost::alloc::string::String,
    /// Id for the collector
    #[prost(string, tag = "6")]
    pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "6")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "5")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "4")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "7")]
	pub id: ::prost::alloc::string::String,
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
	/// Id for the collector
	#[prost(string, tag = "9")]
	pub id: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OneDriveConfig {
	/// Client ID of the app
	#[prost(string, tag = "1")]
	pub client_id: ::prost::alloc::string::String,
	/// Client secret of the app
	#[prost(string, tag = "2")]
	pub client_secret: ::prost::alloc::string::String,
	/// Redirect URI
	#[prost(string, tag = "3")]
	pub redirect_uri: ::prost::alloc::string::String,
	/// Auth code of the app
	#[prost(string, tag = "4")]
	pub auth_code: ::prost::alloc::string::String,
	/// Folder path of the app
	#[prost(string, tag = "5")]
	pub folder_path: ::prost::alloc::string::String,
	/// / Id for the collector
	#[prost(string, tag = "6")]
	pub id: ::prost::alloc::string::String,
}
/// StorageConfig is a message to hold configuration for a storage.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StorageConfig {
	/// Postgres configuration.
	#[prost(message, optional, tag = "1")]
	pub postgres: ::core::option::Option<PostgresConfig>,
	/// Milvus configuration.
	#[prost(message, optional, tag = "2")]
	pub milvus: ::core::option::Option<MilvusConfig>,
	/// Neo4j configuration.
	#[prost(message, optional, tag = "3")]
	pub neo4j: ::core::option::Option<Neo4jConfig>,
}
/// PostgresConfig is a message to hold configuration for a Postgres storage.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PostgresConfig {
	/// Name of the Postgres storage.
	#[prost(string, tag = "1")]
	pub name: ::prost::alloc::string::String,
	/// Type of the storage.
	#[prost(message, optional, tag = "2")]
	pub storage_type: ::core::option::Option<StorageType>,
	/// URL of the Postgres storage.
	#[prost(string, tag = "3")]
	pub url: ::prost::alloc::string::String,
}
/// MilvusConfig is a message to hold configuration for a Milvus storage.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MilvusConfig {
	/// Name of the Milvus storage.
	#[prost(string, tag = "1")]
	pub name: ::prost::alloc::string::String,
	/// Type of the storage.
	#[prost(message, optional, tag = "2")]
	pub storage_type: ::core::option::Option<StorageType>,
	/// URL of the Milvus storage.
	#[prost(string, tag = "3")]
	pub url: ::prost::alloc::string::String,
	/// Username for the Milvus storage.
	#[prost(string, tag = "4")]
	pub username: ::prost::alloc::string::String,
	/// Password for the Milvus storage.
	#[prost(string, tag = "5")]
	pub password: ::prost::alloc::string::String,
}
/// Neo4jConfig is a message to hold configuration for a Neo4j storage.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Neo4jConfig {
	/// Name of the Neo4j storage.
	#[prost(string, tag = "1")]
	pub name: ::prost::alloc::string::String,
	/// Type of the storage.
	#[prost(message, optional, tag = "2")]
	pub storage_type: ::core::option::Option<StorageType>,
	/// URL of the Neo4j storage.
	#[prost(string, tag = "3")]
	pub url: ::prost::alloc::string::String,
	/// Username for the Neo4j storage.
	#[prost(string, tag = "4")]
	pub username: ::prost::alloc::string::String,
	/// Password for the Neo4j storage.
	#[prost(string, tag = "5")]
	pub password: ::prost::alloc::string::String,
	/// Name of the database in the Neo4j storage.
	#[prost(string, tag = "6")]
	pub db_name: ::prost::alloc::string::String,
	/// Fetch size for the Neo4j storage.
	#[prost(int32, tag = "7")]
	pub fetch_size: i32,
	/// Maximum connection pool size for the Neo4j storage.
	#[prost(int32, tag = "8")]
	pub max_connection_pool_size: i32,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SurrealDbConfig {
    /// path of the database
    #[prost(string, tag = "1")]
    pub path: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Model {
	Bert = 0,
	GeoBert = 1,
	PubMedBert = 2,
}
impl Model {
	/// String value of the enum field names used in the ProtoBuf definition.
	///
	/// The values are not transformed in any way and thus are considered stable
	/// (if the ProtoBuf definition does not change) and safe for programmatic use.
	pub fn as_str_name(&self) -> &'static str {
		match self {
			Model::Bert => "BERT",
			Model::GeoBert => "GeoBert",
			Model::PubMedBert => "PubMedBert",
		}
	}
	/// Creates an enum from field names used in the ProtoBuf definition.
	pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
		match value {
			"BERT" => Some(Self::Bert),
			"GeoBert" => Some(Self::GeoBert),
			"PubMedBert" => Some(Self::PubMedBert),
			_ => None,
		}
	}
}
use common::tower::RpcName;
/// BEGIN
#[allow(unused_imports)]
use std::str::FromStr;
use tower::{Layer, Service, ServiceExt};
impl RpcName for SemanticPipelineRequest {
	fn rpc_name() -> &'static str {
		"start_pipeline"
	}
}
impl RpcName for EmptyObserve {
	fn rpc_name() -> &'static str {
		"observe_pipeline"
	}
}
impl RpcName for EmptyGetPipelinesMetadata {
	fn rpc_name() -> &'static str {
		"get_pipelines_metadata"
	}
}
impl RpcName for StopPipelineRequest {
	fn rpc_name() -> &'static str {
		"stop_pipeline"
	}
}
impl RpcName for DescribePipelineRequest {
	fn rpc_name() -> &'static str {
		"describe_pipeline"
	}
}
impl RpcName for SendIngestedTokens {
	fn rpc_name() -> &'static str {
		"ingest_tokens"
	}
}
impl RpcName for RestartPipelineRequest {
	fn rpc_name() -> &'static str {
		"restart_pipeline"
	}
}
impl RpcName for CollectorConfig {
	fn rpc_name() -> &'static str {
		"post_collectors"
	}
}
impl RpcName for DeleteCollectorRequest {
	fn rpc_name() -> &'static str {
		"delete_collectors"
	}
}
impl RpcName for ListCollectorRequest {
	fn rpc_name() -> &'static str {
		"list_collectors"
	}
}
impl RpcName for EmptyList {
	fn rpc_name() -> &'static str {
		"list_pipeline_info"
	}
}
#[cfg_attr(any(test, feature = "testsuite"), mockall::automock)]
#[async_trait::async_trait]
pub trait SemanticsService: std::fmt::Debug + dyn_clone::DynClone + Send + Sync + 'static {
	async fn start_pipeline(
		&mut self,
		request: SemanticPipelineRequest,
	) -> crate::semantics::SemanticsResult<SemanticPipelineResponse>;
	async fn observe_pipeline(
		&mut self,
		request: EmptyObserve,
	) -> crate::semantics::SemanticsResult<SemanticServiceCounters>;
	async fn get_pipelines_metadata(
		&mut self,
		request: EmptyGetPipelinesMetadata,
	) -> crate::semantics::SemanticsResult<PipelinesMetadata>;
	async fn stop_pipeline(
		&mut self,
		request: StopPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse>;
	async fn describe_pipeline(
		&mut self,
		request: DescribePipelineRequest,
	) -> crate::semantics::SemanticsResult<IndexingStatistics>;
	async fn ingest_tokens(
		&mut self,
		request: SendIngestedTokens,
	) -> crate::semantics::SemanticsResult<BooleanResponse>;
	async fn restart_pipeline(
		&mut self,
		request: RestartPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse>;
	async fn post_collectors(
		&mut self,
		request: CollectorConfig,
	) -> crate::semantics::SemanticsResult<CollectorConfigResponse>;
	async fn delete_collectors(
		&mut self,
		request: DeleteCollectorRequest,
	) -> crate::semantics::SemanticsResult<DeleteCollectorResponse>;
	async fn list_collectors(
		&mut self,
		request: ListCollectorRequest,
	) -> crate::semantics::SemanticsResult<ListCollectorConfig>;
	async fn list_pipeline_info(
		&mut self,
		request: EmptyList,
	) -> crate::semantics::SemanticsResult<PipelineRequestInfoList>;
}
dyn_clone::clone_trait_object!(SemanticsService);
#[cfg(any(test, feature = "testsuite"))]
impl Clone for MockSemanticsService {
	fn clone(&self) -> Self {
		MockSemanticsService::new()
	}
}
#[derive(Debug, Clone)]
pub struct SemanticsServiceClient {
	inner: Box<dyn SemanticsService>,
}
impl SemanticsServiceClient {
	pub fn new<T>(instance: T) -> Self
	where
		T: SemanticsService,
	{
		#[cfg(any(test, feature = "testsuite"))]
		assert!(
            std::any::TypeId::of:: < T > () != std::any::TypeId::of:: <
            MockSemanticsService > (),
            "`MockSemanticsService` must be wrapped in a `MockSemanticsServiceWrapper`. Use `MockSemanticsService::from(mock)` to instantiate the client."
        );
		Self { inner: Box::new(instance) }
	}
	pub fn as_grpc_service(
		&self,
		max_message_size: bytesize::ByteSize,
	) -> semantics_service_grpc_server::SemanticsServiceGrpcServer<SemanticsServiceGrpcServerAdapter>
	{
		let adapter = SemanticsServiceGrpcServerAdapter::new(self.clone());
		semantics_service_grpc_server::SemanticsServiceGrpcServer::new(adapter)
			.max_decoding_message_size(max_message_size.0 as usize)
			.max_encoding_message_size(max_message_size.0 as usize)
	}
	pub fn from_channel(
		addr: std::net::SocketAddr,
		channel: tonic::transport::Channel,
		max_message_size: bytesize::ByteSize,
	) -> Self {
		let (_, connection_keys_watcher) =
			tokio::sync::watch::channel(std::collections::HashSet::from_iter([addr]));
		let client = semantics_service_grpc_client::SemanticsServiceGrpcClient::new(channel)
			.max_decoding_message_size(max_message_size.0 as usize)
			.max_encoding_message_size(max_message_size.0 as usize);
		let adapter = SemanticsServiceGrpcClientAdapter::new(client, connection_keys_watcher);
		Self::new(adapter)
	}
	pub fn from_balance_channel(
		balance_channel: common::tower::BalanceChannel<std::net::SocketAddr>,
		max_message_size: bytesize::ByteSize,
	) -> SemanticsServiceClient {
		let connection_keys_watcher = balance_channel.connection_keys_watcher();
		let client =
			semantics_service_grpc_client::SemanticsServiceGrpcClient::new(balance_channel)
				.max_decoding_message_size(max_message_size.0 as usize)
				.max_encoding_message_size(max_message_size.0 as usize);
		let adapter = SemanticsServiceGrpcClientAdapter::new(client, connection_keys_watcher);
		Self::new(adapter)
	}
	pub fn from_mailbox<A>(mailbox: actors::MessageBus<A>) -> Self
	where
		A: actors::Actor + std::fmt::Debug + Send + 'static,
		SemanticsServiceMessageBus<A>: SemanticsService,
	{
		SemanticsServiceClient::new(SemanticsServiceMessageBus::new(mailbox))
	}
	pub fn tower() -> SemanticsServiceTowerLayerStack {
		SemanticsServiceTowerLayerStack::default()
	}
	#[cfg(any(test, feature = "testsuite"))]
	pub fn mock() -> MockSemanticsService {
		MockSemanticsService::new()
	}
}
#[async_trait::async_trait]
impl SemanticsService for SemanticsServiceClient {
	async fn start_pipeline(
		&mut self,
		request: SemanticPipelineRequest,
	) -> crate::semantics::SemanticsResult<SemanticPipelineResponse> {
		self.inner.start_pipeline(request).await
	}
	async fn observe_pipeline(
		&mut self,
		request: EmptyObserve,
	) -> crate::semantics::SemanticsResult<SemanticServiceCounters> {
		self.inner.observe_pipeline(request).await
	}
	async fn get_pipelines_metadata(
		&mut self,
		request: EmptyGetPipelinesMetadata,
	) -> crate::semantics::SemanticsResult<PipelinesMetadata> {
		self.inner.get_pipelines_metadata(request).await
	}
	async fn stop_pipeline(
		&mut self,
		request: StopPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner.stop_pipeline(request).await
	}
	async fn describe_pipeline(
		&mut self,
		request: DescribePipelineRequest,
	) -> crate::semantics::SemanticsResult<IndexingStatistics> {
		self.inner.describe_pipeline(request).await
	}
	async fn ingest_tokens(
		&mut self,
		request: SendIngestedTokens,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner.ingest_tokens(request).await
	}
	async fn restart_pipeline(
		&mut self,
		request: RestartPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner.restart_pipeline(request).await
	}
	async fn post_collectors(
		&mut self,
		request: CollectorConfig,
	) -> crate::semantics::SemanticsResult<CollectorConfigResponse> {
		self.inner.post_collectors(request).await
	}
	async fn delete_collectors(
		&mut self,
		request: DeleteCollectorRequest,
	) -> crate::semantics::SemanticsResult<DeleteCollectorResponse> {
		self.inner.delete_collectors(request).await
	}
	async fn list_collectors(
		&mut self,
		request: ListCollectorRequest,
	) -> crate::semantics::SemanticsResult<ListCollectorConfig> {
		self.inner.list_collectors(request).await
	}
	async fn list_pipeline_info(
		&mut self,
		request: EmptyList,
	) -> crate::semantics::SemanticsResult<PipelineRequestInfoList> {
		self.inner.list_pipeline_info(request).await
	}
}
#[cfg(any(test, feature = "testsuite"))]
pub mod semantics_service_mock {
	use super::*;
	#[derive(Debug, Clone)]
	struct MockSemanticsServiceWrapper {
		inner: std::sync::Arc<tokio::sync::Mutex<MockSemanticsService>>,
	}
	#[async_trait::async_trait]
	impl SemanticsService for MockSemanticsServiceWrapper {
		async fn start_pipeline(
			&mut self,
			request: super::SemanticPipelineRequest,
		) -> crate::semantics::SemanticsResult<super::SemanticPipelineResponse> {
			self.inner.lock().await.start_pipeline(request).await
		}
		async fn observe_pipeline(
			&mut self,
			request: super::EmptyObserve,
		) -> crate::semantics::SemanticsResult<super::SemanticServiceCounters> {
			self.inner.lock().await.observe_pipeline(request).await
		}
		async fn get_pipelines_metadata(
			&mut self,
			request: super::EmptyGetPipelinesMetadata,
		) -> crate::semantics::SemanticsResult<super::PipelinesMetadata> {
			self.inner.lock().await.get_pipelines_metadata(request).await
		}
		async fn stop_pipeline(
			&mut self,
			request: super::StopPipelineRequest,
		) -> crate::semantics::SemanticsResult<super::BooleanResponse> {
			self.inner.lock().await.stop_pipeline(request).await
		}
		async fn describe_pipeline(
			&mut self,
			request: super::DescribePipelineRequest,
		) -> crate::semantics::SemanticsResult<super::IndexingStatistics> {
			self.inner.lock().await.describe_pipeline(request).await
		}
		async fn ingest_tokens(
			&mut self,
			request: super::SendIngestedTokens,
		) -> crate::semantics::SemanticsResult<super::BooleanResponse> {
			self.inner.lock().await.ingest_tokens(request).await
		}
		async fn restart_pipeline(
			&mut self,
			request: super::RestartPipelineRequest,
		) -> crate::semantics::SemanticsResult<super::BooleanResponse> {
			self.inner.lock().await.restart_pipeline(request).await
		}
		async fn post_collectors(
			&mut self,
			request: super::CollectorConfig,
		) -> crate::semantics::SemanticsResult<super::CollectorConfigResponse> {
			self.inner.lock().await.post_collectors(request).await
		}
		async fn delete_collectors(
			&mut self,
			request: super::DeleteCollectorRequest,
		) -> crate::semantics::SemanticsResult<super::DeleteCollectorResponse> {
			self.inner.lock().await.delete_collectors(request).await
		}
		async fn list_collectors(
			&mut self,
			request: super::ListCollectorRequest,
		) -> crate::semantics::SemanticsResult<super::ListCollectorConfig> {
			self.inner.lock().await.list_collectors(request).await
		}
		async fn list_pipeline_info(
			&mut self,
			request: super::EmptyList,
		) -> crate::semantics::SemanticsResult<super::PipelineRequestInfoList> {
			self.inner.lock().await.list_pipeline_info(request).await
		}
	}
	impl From<MockSemanticsService> for SemanticsServiceClient {
		fn from(mock: MockSemanticsService) -> Self {
			let mock_wrapper = MockSemanticsServiceWrapper {
				inner: std::sync::Arc::new(tokio::sync::Mutex::new(mock)),
			};
			SemanticsServiceClient::new(mock_wrapper)
		}
	}
}
pub type BoxFuture<T, E> =
	std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'static>>;
impl tower::Service<SemanticPipelineRequest> for Box<dyn SemanticsService> {
	type Response = SemanticPipelineResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: SemanticPipelineRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.start_pipeline(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<EmptyObserve> for Box<dyn SemanticsService> {
	type Response = SemanticServiceCounters;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: EmptyObserve) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.observe_pipeline(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<EmptyGetPipelinesMetadata> for Box<dyn SemanticsService> {
	type Response = PipelinesMetadata;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: EmptyGetPipelinesMetadata) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.get_pipelines_metadata(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<StopPipelineRequest> for Box<dyn SemanticsService> {
	type Response = BooleanResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: StopPipelineRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.stop_pipeline(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<DescribePipelineRequest> for Box<dyn SemanticsService> {
	type Response = IndexingStatistics;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: DescribePipelineRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.describe_pipeline(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<SendIngestedTokens> for Box<dyn SemanticsService> {
	type Response = BooleanResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: SendIngestedTokens) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.ingest_tokens(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<RestartPipelineRequest> for Box<dyn SemanticsService> {
	type Response = BooleanResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: RestartPipelineRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.restart_pipeline(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<CollectorConfig> for Box<dyn SemanticsService> {
	type Response = CollectorConfigResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: CollectorConfig) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.post_collectors(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<DeleteCollectorRequest> for Box<dyn SemanticsService> {
	type Response = DeleteCollectorResponse;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: DeleteCollectorRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.delete_collectors(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<ListCollectorRequest> for Box<dyn SemanticsService> {
	type Response = ListCollectorConfig;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: ListCollectorRequest) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.list_collectors(request).await };
		Box::pin(fut)
	}
}
impl tower::Service<EmptyList> for Box<dyn SemanticsService> {
	type Response = PipelineRequestInfoList;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, request: EmptyList) -> Self::Future {
		let mut svc = self.clone();
		let fut = async move { svc.list_pipeline_info(request).await };
		Box::pin(fut)
	}
}
/// A tower service stack is a set of tower services.
#[derive(Debug)]
struct SemanticsServiceTowerServiceStack {
	inner: Box<dyn SemanticsService>,
	start_pipeline_svc: common::tower::BoxService<
		SemanticPipelineRequest,
		SemanticPipelineResponse,
		crate::semantics::SemanticsError,
	>,
	observe_pipeline_svc: common::tower::BoxService<
		EmptyObserve,
		SemanticServiceCounters,
		crate::semantics::SemanticsError,
	>,
	get_pipelines_metadata_svc: common::tower::BoxService<
		EmptyGetPipelinesMetadata,
		PipelinesMetadata,
		crate::semantics::SemanticsError,
	>,
	stop_pipeline_svc: common::tower::BoxService<
		StopPipelineRequest,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	describe_pipeline_svc: common::tower::BoxService<
		DescribePipelineRequest,
		IndexingStatistics,
		crate::semantics::SemanticsError,
	>,
	ingest_tokens_svc: common::tower::BoxService<
		SendIngestedTokens,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	restart_pipeline_svc: common::tower::BoxService<
		RestartPipelineRequest,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	post_collectors_svc: common::tower::BoxService<
		CollectorConfig,
		CollectorConfigResponse,
		crate::semantics::SemanticsError,
	>,
	delete_collectors_svc: common::tower::BoxService<
		DeleteCollectorRequest,
		DeleteCollectorResponse,
		crate::semantics::SemanticsError,
	>,
	list_collectors_svc: common::tower::BoxService<
		ListCollectorRequest,
		ListCollectorConfig,
		crate::semantics::SemanticsError,
	>,
	list_pipeline_info_svc: common::tower::BoxService<
		EmptyList,
		PipelineRequestInfoList,
		crate::semantics::SemanticsError,
	>,
}
impl Clone for SemanticsServiceTowerServiceStack {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			start_pipeline_svc: self.start_pipeline_svc.clone(),
			observe_pipeline_svc: self.observe_pipeline_svc.clone(),
			get_pipelines_metadata_svc: self.get_pipelines_metadata_svc.clone(),
			stop_pipeline_svc: self.stop_pipeline_svc.clone(),
			describe_pipeline_svc: self.describe_pipeline_svc.clone(),
			ingest_tokens_svc: self.ingest_tokens_svc.clone(),
			restart_pipeline_svc: self.restart_pipeline_svc.clone(),
			post_collectors_svc: self.post_collectors_svc.clone(),
			delete_collectors_svc: self.delete_collectors_svc.clone(),
			list_collectors_svc: self.list_collectors_svc.clone(),
			list_pipeline_info_svc: self.list_pipeline_info_svc.clone(),
		}
	}
}
#[async_trait::async_trait]
impl SemanticsService for SemanticsServiceTowerServiceStack {
	async fn start_pipeline(
		&mut self,
		request: SemanticPipelineRequest,
	) -> crate::semantics::SemanticsResult<SemanticPipelineResponse> {
		self.start_pipeline_svc.ready().await?.call(request).await
	}
	async fn observe_pipeline(
		&mut self,
		request: EmptyObserve,
	) -> crate::semantics::SemanticsResult<SemanticServiceCounters> {
		self.observe_pipeline_svc.ready().await?.call(request).await
	}
	async fn get_pipelines_metadata(
		&mut self,
		request: EmptyGetPipelinesMetadata,
	) -> crate::semantics::SemanticsResult<PipelinesMetadata> {
		self.get_pipelines_metadata_svc.ready().await?.call(request).await
	}
	async fn stop_pipeline(
		&mut self,
		request: StopPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.stop_pipeline_svc.ready().await?.call(request).await
	}
	async fn describe_pipeline(
		&mut self,
		request: DescribePipelineRequest,
	) -> crate::semantics::SemanticsResult<IndexingStatistics> {
		self.describe_pipeline_svc.ready().await?.call(request).await
	}
	async fn ingest_tokens(
		&mut self,
		request: SendIngestedTokens,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.ingest_tokens_svc.ready().await?.call(request).await
	}
	async fn restart_pipeline(
		&mut self,
		request: RestartPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.restart_pipeline_svc.ready().await?.call(request).await
	}
	async fn post_collectors(
		&mut self,
		request: CollectorConfig,
	) -> crate::semantics::SemanticsResult<CollectorConfigResponse> {
		self.post_collectors_svc.ready().await?.call(request).await
	}
	async fn delete_collectors(
		&mut self,
		request: DeleteCollectorRequest,
	) -> crate::semantics::SemanticsResult<DeleteCollectorResponse> {
		self.delete_collectors_svc.ready().await?.call(request).await
	}
	async fn list_collectors(
		&mut self,
		request: ListCollectorRequest,
	) -> crate::semantics::SemanticsResult<ListCollectorConfig> {
		self.list_collectors_svc.ready().await?.call(request).await
	}
	async fn list_pipeline_info(
		&mut self,
		request: EmptyList,
	) -> crate::semantics::SemanticsResult<PipelineRequestInfoList> {
		self.list_pipeline_info_svc.ready().await?.call(request).await
	}
}
type StartPipelineLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		SemanticPipelineRequest,
		SemanticPipelineResponse,
		crate::semantics::SemanticsError,
	>,
	SemanticPipelineRequest,
	SemanticPipelineResponse,
	crate::semantics::SemanticsError,
>;
type ObservePipelineLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		EmptyObserve,
		SemanticServiceCounters,
		crate::semantics::SemanticsError,
	>,
	EmptyObserve,
	SemanticServiceCounters,
	crate::semantics::SemanticsError,
>;
type GetPipelinesMetadataLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		EmptyGetPipelinesMetadata,
		PipelinesMetadata,
		crate::semantics::SemanticsError,
	>,
	EmptyGetPipelinesMetadata,
	PipelinesMetadata,
	crate::semantics::SemanticsError,
>;
type StopPipelineLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		StopPipelineRequest,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	StopPipelineRequest,
	BooleanResponse,
	crate::semantics::SemanticsError,
>;
type DescribePipelineLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		DescribePipelineRequest,
		IndexingStatistics,
		crate::semantics::SemanticsError,
	>,
	DescribePipelineRequest,
	IndexingStatistics,
	crate::semantics::SemanticsError,
>;
type IngestTokensLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		SendIngestedTokens,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	SendIngestedTokens,
	BooleanResponse,
	crate::semantics::SemanticsError,
>;
type RestartPipelineLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		RestartPipelineRequest,
		BooleanResponse,
		crate::semantics::SemanticsError,
	>,
	RestartPipelineRequest,
	BooleanResponse,
	crate::semantics::SemanticsError,
>;
type PostCollectorsLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		CollectorConfig,
		CollectorConfigResponse,
		crate::semantics::SemanticsError,
	>,
	CollectorConfig,
	CollectorConfigResponse,
	crate::semantics::SemanticsError,
>;
type DeleteCollectorsLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		DeleteCollectorRequest,
		DeleteCollectorResponse,
		crate::semantics::SemanticsError,
	>,
	DeleteCollectorRequest,
	DeleteCollectorResponse,
	crate::semantics::SemanticsError,
>;
type ListCollectorsLayer = common::tower::BoxLayer<
	common::tower::BoxService<
		ListCollectorRequest,
		ListCollectorConfig,
		crate::semantics::SemanticsError,
	>,
	ListCollectorRequest,
	ListCollectorConfig,
	crate::semantics::SemanticsError,
>;
type ListPipelineInfoLayer = common::tower::BoxLayer<
	common::tower::BoxService<EmptyList, PipelineRequestInfoList, crate::semantics::SemanticsError>,
	EmptyList,
	PipelineRequestInfoList,
	crate::semantics::SemanticsError,
>;
#[derive(Debug, Default)]
pub struct SemanticsServiceTowerLayerStack {
	start_pipeline_layers: Vec<StartPipelineLayer>,
	observe_pipeline_layers: Vec<ObservePipelineLayer>,
	get_pipelines_metadata_layers: Vec<GetPipelinesMetadataLayer>,
	stop_pipeline_layers: Vec<StopPipelineLayer>,
	describe_pipeline_layers: Vec<DescribePipelineLayer>,
	ingest_tokens_layers: Vec<IngestTokensLayer>,
	restart_pipeline_layers: Vec<RestartPipelineLayer>,
	post_collectors_layers: Vec<PostCollectorsLayer>,
	delete_collectors_layers: Vec<DeleteCollectorsLayer>,
	list_collectors_layers: Vec<ListCollectorsLayer>,
	list_pipeline_info_layers: Vec<ListPipelineInfoLayer>,
}
impl SemanticsServiceTowerLayerStack {
	pub fn stack_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					SemanticPipelineRequest,
					SemanticPipelineResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				SemanticPipelineRequest,
				SemanticPipelineResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				SemanticPipelineRequest,
				Response = SemanticPipelineResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				SemanticPipelineRequest,
				SemanticPipelineResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<SemanticPipelineRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					EmptyObserve,
					SemanticServiceCounters,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				EmptyObserve,
				SemanticServiceCounters,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				EmptyObserve,
				Response = SemanticServiceCounters,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				EmptyObserve,
				SemanticServiceCounters,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<EmptyObserve>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					EmptyGetPipelinesMetadata,
					PipelinesMetadata,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				EmptyGetPipelinesMetadata,
				PipelinesMetadata,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				EmptyGetPipelinesMetadata,
				Response = PipelinesMetadata,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				EmptyGetPipelinesMetadata,
				PipelinesMetadata,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<EmptyGetPipelinesMetadata>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					StopPipelineRequest,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				StopPipelineRequest,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				StopPipelineRequest,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				StopPipelineRequest,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<StopPipelineRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					DescribePipelineRequest,
					IndexingStatistics,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				DescribePipelineRequest,
				IndexingStatistics,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				DescribePipelineRequest,
				Response = IndexingStatistics,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				DescribePipelineRequest,
				IndexingStatistics,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<DescribePipelineRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					SendIngestedTokens,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				SendIngestedTokens,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				SendIngestedTokens,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				SendIngestedTokens,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<SendIngestedTokens>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					RestartPipelineRequest,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				RestartPipelineRequest,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				RestartPipelineRequest,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				RestartPipelineRequest,
				BooleanResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<RestartPipelineRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					CollectorConfig,
					CollectorConfigResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				CollectorConfig,
				CollectorConfigResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				CollectorConfig,
				Response = CollectorConfigResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				CollectorConfig,
				CollectorConfigResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<CollectorConfig>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					DeleteCollectorRequest,
					DeleteCollectorResponse,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				DeleteCollectorRequest,
				DeleteCollectorResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				DeleteCollectorRequest,
				Response = DeleteCollectorResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				DeleteCollectorRequest,
				DeleteCollectorResponse,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<DeleteCollectorRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					ListCollectorRequest,
					ListCollectorConfig,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				ListCollectorRequest,
				ListCollectorConfig,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				ListCollectorRequest,
				Response = ListCollectorConfig,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				ListCollectorRequest,
				ListCollectorConfig,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<ListCollectorRequest>>::Future: Send + 'static,
		L: tower::Layer<
				common::tower::BoxService<
					EmptyList,
					PipelineRequestInfoList,
					crate::semantics::SemanticsError,
				>,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L as tower::Layer<
			common::tower::BoxService<
				EmptyList,
				PipelineRequestInfoList,
				crate::semantics::SemanticsError,
			>,
		>>::Service: tower::Service<
				EmptyList,
				Response = PipelineRequestInfoList,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<<L as tower::Layer<
			common::tower::BoxService<
				EmptyList,
				PipelineRequestInfoList,
				crate::semantics::SemanticsError,
			>,
		>>::Service as tower::Service<EmptyList>>::Future: Send + 'static,
	{
		self.start_pipeline_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.observe_pipeline_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.get_pipelines_metadata_layers
			.push(common::tower::BoxLayer::new(layer.clone()));
		self.stop_pipeline_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.describe_pipeline_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.ingest_tokens_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.restart_pipeline_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.post_collectors_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.delete_collectors_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.list_collectors_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self.list_pipeline_info_layers.push(common::tower::BoxLayer::new(layer.clone()));
		self
	}
	pub fn stack_start_pipeline_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					SemanticPipelineRequest,
					SemanticPipelineResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				SemanticPipelineRequest,
				Response = SemanticPipelineResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<SemanticPipelineRequest>>::Future: Send + 'static,
	{
		self.start_pipeline_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_observe_pipeline_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					EmptyObserve,
					SemanticServiceCounters,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				EmptyObserve,
				Response = SemanticServiceCounters,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<EmptyObserve>>::Future: Send + 'static,
	{
		self.observe_pipeline_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_get_pipelines_metadata_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					EmptyGetPipelinesMetadata,
					PipelinesMetadata,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				EmptyGetPipelinesMetadata,
				Response = PipelinesMetadata,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<EmptyGetPipelinesMetadata>>::Future: Send + 'static,
	{
		self.get_pipelines_metadata_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_stop_pipeline_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					StopPipelineRequest,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				StopPipelineRequest,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<StopPipelineRequest>>::Future: Send + 'static,
	{
		self.stop_pipeline_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_describe_pipeline_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					DescribePipelineRequest,
					IndexingStatistics,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				DescribePipelineRequest,
				Response = IndexingStatistics,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<DescribePipelineRequest>>::Future: Send + 'static,
	{
		self.describe_pipeline_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_ingest_tokens_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					SendIngestedTokens,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				SendIngestedTokens,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<SendIngestedTokens>>::Future: Send + 'static,
	{
		self.ingest_tokens_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_restart_pipeline_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					RestartPipelineRequest,
					BooleanResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				RestartPipelineRequest,
				Response = BooleanResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<RestartPipelineRequest>>::Future: Send + 'static,
	{
		self.restart_pipeline_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_post_collectors_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					CollectorConfig,
					CollectorConfigResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				CollectorConfig,
				Response = CollectorConfigResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<CollectorConfig>>::Future: Send + 'static,
	{
		self.post_collectors_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_delete_collectors_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					DeleteCollectorRequest,
					DeleteCollectorResponse,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				DeleteCollectorRequest,
				Response = DeleteCollectorResponse,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<DeleteCollectorRequest>>::Future: Send + 'static,
	{
		self.delete_collectors_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_list_collectors_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					ListCollectorRequest,
					ListCollectorConfig,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				ListCollectorRequest,
				Response = ListCollectorConfig,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<ListCollectorRequest>>::Future: Send + 'static,
	{
		self.list_collectors_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn stack_list_pipeline_info_layer<L>(mut self, layer: L) -> Self
	where
		L: tower::Layer<
				common::tower::BoxService<
					EmptyList,
					PipelineRequestInfoList,
					crate::semantics::SemanticsError,
				>,
			> + Send
			+ Sync
			+ 'static,
		L::Service: tower::Service<
				EmptyList,
				Response = PipelineRequestInfoList,
				Error = crate::semantics::SemanticsError,
			> + Clone
			+ Send
			+ Sync
			+ 'static,
		<L::Service as tower::Service<EmptyList>>::Future: Send + 'static,
	{
		self.list_pipeline_info_layers.push(common::tower::BoxLayer::new(layer));
		self
	}
	pub fn build<T>(self, instance: T) -> SemanticsServiceClient
	where
		T: SemanticsService,
	{
		self.build_from_boxed(Box::new(instance))
	}
	pub fn build_from_channel(
		self,
		addr: std::net::SocketAddr,
		channel: tonic::transport::Channel,
		max_message_size: bytesize::ByteSize,
	) -> SemanticsServiceClient {
		self.build_from_boxed(Box::new(SemanticsServiceClient::from_channel(
			addr,
			channel,
			max_message_size,
		)))
	}
	pub fn build_from_balance_channel(
		self,
		balance_channel: common::tower::BalanceChannel<std::net::SocketAddr>,
		max_message_size: bytesize::ByteSize,
	) -> SemanticsServiceClient {
		self.build_from_boxed(Box::new(SemanticsServiceClient::from_balance_channel(
			balance_channel,
			max_message_size,
		)))
	}
	pub fn build_from_mailbox<A>(self, mailbox: actors::MessageBus<A>) -> SemanticsServiceClient
	where
		A: actors::Actor + std::fmt::Debug + Send + 'static,
		SemanticsServiceMessageBus<A>: SemanticsService,
	{
		self.build_from_boxed(Box::new(SemanticsServiceMessageBus::new(mailbox)))
	}
	fn build_from_boxed(self, boxed_instance: Box<dyn SemanticsService>) -> SemanticsServiceClient {
		let start_pipeline_svc = self
			.start_pipeline_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let observe_pipeline_svc = self
			.observe_pipeline_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let get_pipelines_metadata_svc = self
			.get_pipelines_metadata_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let stop_pipeline_svc = self
			.stop_pipeline_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let describe_pipeline_svc = self
			.describe_pipeline_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let ingest_tokens_svc = self
			.ingest_tokens_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let restart_pipeline_svc = self
			.restart_pipeline_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let post_collectors_svc = self
			.post_collectors_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let delete_collectors_svc = self
			.delete_collectors_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let list_collectors_svc = self
			.list_collectors_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let list_pipeline_info_svc = self
			.list_pipeline_info_layers
			.into_iter()
			.rev()
			.fold(common::tower::BoxService::new(boxed_instance.clone()), |svc, layer| {
				layer.layer(svc)
			});
		let tower_svc_stack = SemanticsServiceTowerServiceStack {
			inner: boxed_instance.clone(),
			start_pipeline_svc,
			observe_pipeline_svc,
			get_pipelines_metadata_svc,
			stop_pipeline_svc,
			describe_pipeline_svc,
			ingest_tokens_svc,
			restart_pipeline_svc,
			post_collectors_svc,
			delete_collectors_svc,
			list_collectors_svc,
			list_pipeline_info_svc,
		};
		SemanticsServiceClient::new(tower_svc_stack)
	}
}
#[derive(Debug, Clone)]
struct MailboxAdapter<A: actors::Actor, E> {
	inner: actors::MessageBus<A>,
	phantom: std::marker::PhantomData<E>,
}
impl<A, E> std::ops::Deref for MailboxAdapter<A, E>
where
	A: actors::Actor,
{
	type Target = actors::MessageBus<A>;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
#[derive(Debug)]
pub struct SemanticsServiceMessageBus<A: actors::Actor> {
	inner: MailboxAdapter<A, crate::semantics::SemanticsError>,
}
impl<A: actors::Actor> SemanticsServiceMessageBus<A> {
	pub fn new(instance: actors::MessageBus<A>) -> Self {
		let inner = MailboxAdapter { inner: instance, phantom: std::marker::PhantomData };
		Self { inner }
	}
}
impl<A: actors::Actor> Clone for SemanticsServiceMessageBus<A> {
	fn clone(&self) -> Self {
		let inner = MailboxAdapter { inner: self.inner.clone(), phantom: std::marker::PhantomData };
		Self { inner }
	}
}
impl<A, M, T, E> tower::Service<M> for SemanticsServiceMessageBus<A>
where
	A: actors::Actor + actors::DeferableReplyHandler<M, Reply = Result<T, E>> + Send + 'static,
	M: std::fmt::Debug + Send + 'static,
	T: Send + 'static,
	E: std::fmt::Debug + Send + 'static,
	crate::semantics::SemanticsError: From<actors::AskError<E>>,
{
	type Response = T;
	type Error = crate::semantics::SemanticsError;
	type Future = BoxFuture<Self::Response, Self::Error>;
	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		//! This does not work with balance middlewares such as `tower::balance::pool::Pool` because
		//! this always returns `Poll::Ready`. The fix is to acquire a permit from the
		//! mailbox in `poll_ready` and consume it in `call`.
		std::task::Poll::Ready(Ok(()))
	}
	fn call(&mut self, message: M) -> Self::Future {
		let mailbox = self.inner.clone();
		let fut = async move { mailbox.ask_for_res(message).await.map_err(|error| error.into()) };
		Box::pin(fut)
	}
}
#[async_trait::async_trait]
impl<A> SemanticsService for SemanticsServiceMessageBus<A>
where
	A: actors::Actor + std::fmt::Debug,
	SemanticsServiceMessageBus<A>: tower::Service<
			SemanticPipelineRequest,
			Response = SemanticPipelineResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<SemanticPipelineResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			EmptyObserve,
			Response = SemanticServiceCounters,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<SemanticServiceCounters, crate::semantics::SemanticsError>,
		> + tower::Service<
			EmptyGetPipelinesMetadata,
			Response = PipelinesMetadata,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<PipelinesMetadata, crate::semantics::SemanticsError>,
		> + tower::Service<
			StopPipelineRequest,
			Response = BooleanResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<BooleanResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			DescribePipelineRequest,
			Response = IndexingStatistics,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<IndexingStatistics, crate::semantics::SemanticsError>,
		> + tower::Service<
			SendIngestedTokens,
			Response = BooleanResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<BooleanResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			RestartPipelineRequest,
			Response = BooleanResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<BooleanResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			CollectorConfig,
			Response = CollectorConfigResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<CollectorConfigResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			DeleteCollectorRequest,
			Response = DeleteCollectorResponse,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<DeleteCollectorResponse, crate::semantics::SemanticsError>,
		> + tower::Service<
			ListCollectorRequest,
			Response = ListCollectorConfig,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<ListCollectorConfig, crate::semantics::SemanticsError>,
		> + tower::Service<
			EmptyList,
			Response = PipelineRequestInfoList,
			Error = crate::semantics::SemanticsError,
			Future = BoxFuture<PipelineRequestInfoList, crate::semantics::SemanticsError>,
		>,
{
	async fn start_pipeline(
		&mut self,
		request: SemanticPipelineRequest,
	) -> crate::semantics::SemanticsResult<SemanticPipelineResponse> {
		self.call(request).await
	}
	async fn observe_pipeline(
		&mut self,
		request: EmptyObserve,
	) -> crate::semantics::SemanticsResult<SemanticServiceCounters> {
		self.call(request).await
	}
	async fn get_pipelines_metadata(
		&mut self,
		request: EmptyGetPipelinesMetadata,
	) -> crate::semantics::SemanticsResult<PipelinesMetadata> {
		self.call(request).await
	}
	async fn stop_pipeline(
		&mut self,
		request: StopPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.call(request).await
	}
	async fn describe_pipeline(
		&mut self,
		request: DescribePipelineRequest,
	) -> crate::semantics::SemanticsResult<IndexingStatistics> {
		self.call(request).await
	}
	async fn ingest_tokens(
		&mut self,
		request: SendIngestedTokens,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.call(request).await
	}
	async fn restart_pipeline(
		&mut self,
		request: RestartPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.call(request).await
	}
	async fn post_collectors(
		&mut self,
		request: CollectorConfig,
	) -> crate::semantics::SemanticsResult<CollectorConfigResponse> {
		self.call(request).await
	}
	async fn delete_collectors(
		&mut self,
		request: DeleteCollectorRequest,
	) -> crate::semantics::SemanticsResult<DeleteCollectorResponse> {
		self.call(request).await
	}
	async fn list_collectors(
		&mut self,
		request: ListCollectorRequest,
	) -> crate::semantics::SemanticsResult<ListCollectorConfig> {
		self.call(request).await
	}
	async fn list_pipeline_info(
        &mut self,
        request: EmptyList,
    ) -> crate::semantics::SemanticsResult<PipelineRequestInfoList> {
        self.call(request).await
    }
}
#[derive(Debug, Clone)]
pub struct SemanticsServiceGrpcClientAdapter<T> {
	inner: T,
	#[allow(dead_code)]
	connection_addrs_rx:
		tokio::sync::watch::Receiver<std::collections::HashSet<std::net::SocketAddr>>,
}
impl<T> SemanticsServiceGrpcClientAdapter<T> {
	pub fn new(
		instance: T,
		connection_addrs_rx: tokio::sync::watch::Receiver<
			std::collections::HashSet<std::net::SocketAddr>,
		>,
	) -> Self {
		Self { inner: instance, connection_addrs_rx }
	}
}
#[async_trait::async_trait]
impl<T> SemanticsService
	for SemanticsServiceGrpcClientAdapter<semantics_service_grpc_client::SemanticsServiceGrpcClient<T>>
where
	T: tonic::client::GrpcService<tonic::body::BoxBody>
		+ std::fmt::Debug
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
	<T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
	T::Future: Send,
{
	async fn start_pipeline(
		&mut self,
		request: SemanticPipelineRequest,
	) -> crate::semantics::SemanticsResult<SemanticPipelineResponse> {
		self.inner
			.start_pipeline(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn observe_pipeline(
		&mut self,
		request: EmptyObserve,
	) -> crate::semantics::SemanticsResult<SemanticServiceCounters> {
		self.inner
			.observe_pipeline(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn get_pipelines_metadata(
		&mut self,
		request: EmptyGetPipelinesMetadata,
	) -> crate::semantics::SemanticsResult<PipelinesMetadata> {
		self.inner
			.get_pipelines_metadata(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn stop_pipeline(
		&mut self,
		request: StopPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner
			.stop_pipeline(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn describe_pipeline(
		&mut self,
		request: DescribePipelineRequest,
	) -> crate::semantics::SemanticsResult<IndexingStatistics> {
		self.inner
			.describe_pipeline(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn ingest_tokens(
		&mut self,
		request: SendIngestedTokens,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner
			.ingest_tokens(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn restart_pipeline(
		&mut self,
		request: RestartPipelineRequest,
	) -> crate::semantics::SemanticsResult<BooleanResponse> {
		self.inner
			.restart_pipeline(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn post_collectors(
		&mut self,
		request: CollectorConfig,
	) -> crate::semantics::SemanticsResult<CollectorConfigResponse> {
		self.inner
			.post_collectors(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn delete_collectors(
		&mut self,
		request: DeleteCollectorRequest,
	) -> crate::semantics::SemanticsResult<DeleteCollectorResponse> {
		self.inner
			.delete_collectors(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn list_collectors(
		&mut self,
		request: ListCollectorRequest,
	) -> crate::semantics::SemanticsResult<ListCollectorConfig> {
		self.inner
			.list_collectors(request)
			.await
			.map(|response| response.into_inner())
			.map_err(crate::error::grpc_status_to_service_error)
	}
	async fn list_pipeline_info(
        &mut self,
        request: EmptyList,
    ) -> crate::semantics::SemanticsResult<PipelineRequestInfoList> {
        self.inner
            .list_pipeline_info(request)
            .await
            .map(|response| response.into_inner())
            .map_err(crate::error::grpc_status_to_service_error)
    }
}
#[derive(Debug)]
pub struct SemanticsServiceGrpcServerAdapter {
	inner: Box<dyn SemanticsService>,
}
impl SemanticsServiceGrpcServerAdapter {
	pub fn new<T>(instance: T) -> Self
	where
		T: SemanticsService,
	{
		Self { inner: Box::new(instance) }
	}
}
#[async_trait::async_trait]
impl semantics_service_grpc_server::SemanticsServiceGrpc for SemanticsServiceGrpcServerAdapter {
	async fn start_pipeline(
		&self,
		request: tonic::Request<SemanticPipelineRequest>,
	) -> Result<tonic::Response<SemanticPipelineResponse>, tonic::Status> {
		self.inner
			.clone()
			.start_pipeline(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn observe_pipeline(
		&self,
		request: tonic::Request<EmptyObserve>,
	) -> Result<tonic::Response<SemanticServiceCounters>, tonic::Status> {
		self.inner
			.clone()
			.observe_pipeline(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn get_pipelines_metadata(
		&self,
		request: tonic::Request<EmptyGetPipelinesMetadata>,
	) -> Result<tonic::Response<PipelinesMetadata>, tonic::Status> {
		self.inner
			.clone()
			.get_pipelines_metadata(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn stop_pipeline(
		&self,
		request: tonic::Request<StopPipelineRequest>,
	) -> Result<tonic::Response<BooleanResponse>, tonic::Status> {
		self.inner
			.clone()
			.stop_pipeline(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn describe_pipeline(
		&self,
		request: tonic::Request<DescribePipelineRequest>,
	) -> Result<tonic::Response<IndexingStatistics>, tonic::Status> {
		self.inner
			.clone()
			.describe_pipeline(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn ingest_tokens(
		&self,
		request: tonic::Request<SendIngestedTokens>,
	) -> Result<tonic::Response<BooleanResponse>, tonic::Status> {
		self.inner
			.clone()
			.ingest_tokens(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn restart_pipeline(
		&self,
		request: tonic::Request<RestartPipelineRequest>,
	) -> Result<tonic::Response<BooleanResponse>, tonic::Status> {
		self.inner
			.clone()
			.restart_pipeline(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn post_collectors(
		&self,
		request: tonic::Request<CollectorConfig>,
	) -> Result<tonic::Response<CollectorConfigResponse>, tonic::Status> {
		self.inner
			.clone()
			.post_collectors(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn delete_collectors(
		&self,
		request: tonic::Request<DeleteCollectorRequest>,
	) -> Result<tonic::Response<DeleteCollectorResponse>, tonic::Status> {
		self.inner
			.clone()
			.delete_collectors(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn list_collectors(
		&self,
		request: tonic::Request<ListCollectorRequest>,
	) -> Result<tonic::Response<ListCollectorConfig>, tonic::Status> {
		self.inner
			.clone()
			.list_collectors(request.into_inner())
			.await
			.map(tonic::Response::new)
			.map_err(crate::error::grpc_error_to_grpc_status)
	}
	async fn list_pipeline_info(
        &self,
        request: tonic::Request<EmptyList>,
    ) -> Result<tonic::Response<PipelineRequestInfoList>, tonic::Status> {
        self.inner
            .clone()
            .list_pipeline_info(request.into_inner())
            .await
            .map(tonic::Response::new)
            .map_err(crate::error::grpc_error_to_grpc_status)
    }
}
/// Generated client implementations.
pub mod semantics_service_grpc_client {
	#![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
	use tonic::codegen::http::Uri;
	use tonic::codegen::*;
	#[derive(Debug, Clone)]
	pub struct SemanticsServiceGrpcClient<T> {
		inner: tonic::client::Grpc<T>,
	}
	impl SemanticsServiceGrpcClient<tonic::transport::Channel> {
		/// Attempt to create a new client by connecting to a given endpoint.
		pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
		where
			D: TryInto<tonic::transport::Endpoint>,
			D::Error: Into<StdError>,
		{
			let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
			Ok(Self::new(conn))
		}
	}
	impl<T> SemanticsServiceGrpcClient<T>
	where
		T: tonic::client::GrpcService<tonic::body::BoxBody>,
		T::Error: Into<StdError>,
		T::ResponseBody: Body<Data = Bytes> + Send + 'static,
		<T::ResponseBody as Body>::Error: Into<StdError> + Send,
	{
		pub fn new(inner: T) -> Self {
			let inner = tonic::client::Grpc::new(inner);
			Self { inner }
		}
		pub fn with_origin(inner: T, origin: Uri) -> Self {
			let inner = tonic::client::Grpc::with_origin(inner, origin);
			Self { inner }
		}
		pub fn with_interceptor<F>(
			inner: T,
			interceptor: F,
		) -> SemanticsServiceGrpcClient<InterceptedService<T, F>>
		where
			F: tonic::service::Interceptor,
			T::ResponseBody: Default,
			T: tonic::codegen::Service<
				http::Request<tonic::body::BoxBody>,
				Response = http::Response<
					<T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
				>,
			>,
			<T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
				Into<StdError> + Send + Sync,
		{
			SemanticsServiceGrpcClient::new(InterceptedService::new(inner, interceptor))
		}
		/// Compress requests with the given encoding.
		///
		/// This requires the server to support it otherwise it might respond with an
		/// error.
		#[must_use]
		pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
			self.inner = self.inner.send_compressed(encoding);
			self
		}
		/// Enable decompressing responses.
		#[must_use]
		pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
			self.inner = self.inner.accept_compressed(encoding);
			self
		}
		/// Limits the maximum size of a decoded message.
		///
		/// Default: `4MB`
		#[must_use]
		pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
			self.inner = self.inner.max_decoding_message_size(limit);
			self
		}
		/// Limits the maximum size of an encoded message.
		///
		/// Default: `usize::MAX`
		#[must_use]
		pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
			self.inner = self.inner.max_encoding_message_size(limit);
			self
		}
		pub async fn start_pipeline(
			&mut self,
			request: impl tonic::IntoRequest<super::SemanticPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::SemanticPipelineResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/StartPipeline",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "StartPipeline"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn observe_pipeline(
			&mut self,
			request: impl tonic::IntoRequest<super::EmptyObserve>,
		) -> std::result::Result<tonic::Response<super::SemanticServiceCounters>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/ObservePipeline",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "ObservePipeline"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn get_pipelines_metadata(
			&mut self,
			request: impl tonic::IntoRequest<super::EmptyGetPipelinesMetadata>,
		) -> std::result::Result<tonic::Response<super::PipelinesMetadata>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/GetPipelinesMetadata",
			);
			let mut req = request.into_request();
			req.extensions_mut().insert(GrpcMethod::new(
				"querent.semantics.SemanticsService",
				"GetPipelinesMetadata",
			));
			self.inner.unary(req, path, codec).await
		}
		pub async fn stop_pipeline(
			&mut self,
			request: impl tonic::IntoRequest<super::StopPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/StopPipeline",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "StopPipeline"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn describe_pipeline(
			&mut self,
			request: impl tonic::IntoRequest<super::DescribePipelineRequest>,
		) -> std::result::Result<tonic::Response<super::IndexingStatistics>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/DescribePipeline",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "DescribePipeline"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn ingest_tokens(
			&mut self,
			request: impl tonic::IntoRequest<super::SendIngestedTokens>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/IngestTokens",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "IngestTokens"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn restart_pipeline(
			&mut self,
			request: impl tonic::IntoRequest<super::RestartPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/RestartPipeline",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "RestartPipeline"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn post_collectors(
			&mut self,
			request: impl tonic::IntoRequest<super::CollectorConfig>,
		) -> std::result::Result<tonic::Response<super::CollectorConfigResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/PostCollectors",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "PostCollectors"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn delete_collectors(
			&mut self,
			request: impl tonic::IntoRequest<super::DeleteCollectorRequest>,
		) -> std::result::Result<tonic::Response<super::DeleteCollectorResponse>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/DeleteCollectors",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "DeleteCollectors"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn list_collectors(
			&mut self,
			request: impl tonic::IntoRequest<super::ListCollectorRequest>,
		) -> std::result::Result<tonic::Response<super::ListCollectorConfig>, tonic::Status> {
			self.inner.ready().await.map_err(|e| {
				tonic::Status::new(
					tonic::Code::Unknown,
					format!("Service was not ready: {}", e.into()),
				)
			})?;
			let codec = tonic::codec::ProstCodec::default();
			let path = http::uri::PathAndQuery::from_static(
				"/querent.semantics.SemanticsService/ListCollectors",
			);
			let mut req = request.into_request();
			req.extensions_mut()
				.insert(GrpcMethod::new("querent.semantics.SemanticsService", "ListCollectors"));
			self.inner.unary(req, path, codec).await
		}
		pub async fn list_pipeline_info(
            &mut self,
            request: impl tonic::IntoRequest<super::EmptyList>,
        ) -> std::result::Result<
            tonic::Response<super::PipelineRequestInfoList>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/querent.semantics.SemanticsService/ListPipelineInfo",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.semantics.SemanticsService",
                        "ListPipelineInfo",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
	}
}
/// Generated server implementations.
pub mod semantics_service_grpc_server {
	#![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
	use tonic::codegen::*;
	/// Generated trait containing gRPC methods that should be implemented for use with SemanticsServiceGrpcServer.
	#[async_trait]
	pub trait SemanticsServiceGrpc: Send + Sync + 'static {
		async fn start_pipeline(
			&self,
			request: tonic::Request<super::SemanticPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::SemanticPipelineResponse>, tonic::Status>;
		async fn observe_pipeline(
			&self,
			request: tonic::Request<super::EmptyObserve>,
		) -> std::result::Result<tonic::Response<super::SemanticServiceCounters>, tonic::Status>;
		async fn get_pipelines_metadata(
			&self,
			request: tonic::Request<super::EmptyGetPipelinesMetadata>,
		) -> std::result::Result<tonic::Response<super::PipelinesMetadata>, tonic::Status>;
		async fn stop_pipeline(
			&self,
			request: tonic::Request<super::StopPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status>;
		async fn describe_pipeline(
			&self,
			request: tonic::Request<super::DescribePipelineRequest>,
		) -> std::result::Result<tonic::Response<super::IndexingStatistics>, tonic::Status>;
		async fn ingest_tokens(
			&self,
			request: tonic::Request<super::SendIngestedTokens>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status>;
		async fn restart_pipeline(
			&self,
			request: tonic::Request<super::RestartPipelineRequest>,
		) -> std::result::Result<tonic::Response<super::BooleanResponse>, tonic::Status>;
		async fn post_collectors(
			&self,
			request: tonic::Request<super::CollectorConfig>,
		) -> std::result::Result<tonic::Response<super::CollectorConfigResponse>, tonic::Status>;
		async fn delete_collectors(
			&self,
			request: tonic::Request<super::DeleteCollectorRequest>,
		) -> std::result::Result<tonic::Response<super::DeleteCollectorResponse>, tonic::Status>;
		async fn list_collectors(
			&self,
			request: tonic::Request<super::ListCollectorRequest>,
		) -> std::result::Result<tonic::Response<super::ListCollectorConfig>, tonic::Status>;
		async fn list_pipeline_info(
            &self,
            request: tonic::Request<super::EmptyList>,
        ) -> std::result::Result<
            tonic::Response<super::PipelineRequestInfoList>,
            tonic::Status,
        >;
	}
	#[derive(Debug)]
	pub struct SemanticsServiceGrpcServer<T: SemanticsServiceGrpc> {
		inner: _Inner<T>,
		accept_compression_encodings: EnabledCompressionEncodings,
		send_compression_encodings: EnabledCompressionEncodings,
		max_decoding_message_size: Option<usize>,
		max_encoding_message_size: Option<usize>,
	}
	struct _Inner<T>(Arc<T>);
	impl<T: SemanticsServiceGrpc> SemanticsServiceGrpcServer<T> {
		pub fn new(inner: T) -> Self {
			Self::from_arc(Arc::new(inner))
		}
		pub fn from_arc(inner: Arc<T>) -> Self {
			let inner = _Inner(inner);
			Self {
				inner,
				accept_compression_encodings: Default::default(),
				send_compression_encodings: Default::default(),
				max_decoding_message_size: None,
				max_encoding_message_size: None,
			}
		}
		pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
		where
			F: tonic::service::Interceptor,
		{
			InterceptedService::new(Self::new(inner), interceptor)
		}
		/// Enable decompressing requests with the given encoding.
		#[must_use]
		pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
			self.accept_compression_encodings.enable(encoding);
			self
		}
		/// Compress responses with the given encoding, if the client supports it.
		#[must_use]
		pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
			self.send_compression_encodings.enable(encoding);
			self
		}
		/// Limits the maximum size of a decoded message.
		///
		/// Default: `4MB`
		#[must_use]
		pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
			self.max_decoding_message_size = Some(limit);
			self
		}
		/// Limits the maximum size of an encoded message.
		///
		/// Default: `usize::MAX`
		#[must_use]
		pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
			self.max_encoding_message_size = Some(limit);
			self
		}
	}
	impl<T, B> tonic::codegen::Service<http::Request<B>> for SemanticsServiceGrpcServer<T>
	where
		T: SemanticsServiceGrpc,
		B: Body + Send + 'static,
		B::Error: Into<StdError> + Send + 'static,
	{
		type Response = http::Response<tonic::body::BoxBody>;
		type Error = std::convert::Infallible;
		type Future = BoxFuture<Self::Response, Self::Error>;
		fn poll_ready(
			&mut self,
			_cx: &mut Context<'_>,
		) -> Poll<std::result::Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}
		fn call(&mut self, req: http::Request<B>) -> Self::Future {
			let inner = self.inner.clone();
			match req.uri().path() {
				"/querent.semantics.SemanticsService/StartPipeline" => {
					#[allow(non_camel_case_types)]
					struct StartPipelineSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::SemanticPipelineRequest> for StartPipelineSvc<T>
					{
						type Response = super::SemanticPipelineResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::SemanticPipelineRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).start_pipeline(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = StartPipelineSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/ObservePipeline" => {
					#[allow(non_camel_case_types)]
					struct ObservePipelineSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc> tonic::server::UnaryService<super::EmptyObserve>
						for ObservePipelineSvc<T>
					{
						type Response = super::SemanticServiceCounters;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::EmptyObserve>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).observe_pipeline(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = ObservePipelineSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/GetPipelinesMetadata" => {
					#[allow(non_camel_case_types)]
					struct GetPipelinesMetadataSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::EmptyGetPipelinesMetadata> for GetPipelinesMetadataSvc<T>
					{
						type Response = super::PipelinesMetadata;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::EmptyGetPipelinesMetadata>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).get_pipelines_metadata(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = GetPipelinesMetadataSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/StopPipeline" => {
					#[allow(non_camel_case_types)]
					struct StopPipelineSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::StopPipelineRequest> for StopPipelineSvc<T>
					{
						type Response = super::BooleanResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::StopPipelineRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).stop_pipeline(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = StopPipelineSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/DescribePipeline" => {
					#[allow(non_camel_case_types)]
					struct DescribePipelineSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::DescribePipelineRequest> for DescribePipelineSvc<T>
					{
						type Response = super::IndexingStatistics;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::DescribePipelineRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).describe_pipeline(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = DescribePipelineSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/IngestTokens" => {
					#[allow(non_camel_case_types)]
					struct IngestTokensSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::SendIngestedTokens> for IngestTokensSvc<T>
					{
						type Response = super::BooleanResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::SendIngestedTokens>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).ingest_tokens(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = IngestTokensSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/RestartPipeline" => {
					#[allow(non_camel_case_types)]
					struct RestartPipelineSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::RestartPipelineRequest> for RestartPipelineSvc<T>
					{
						type Response = super::BooleanResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::RestartPipelineRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).restart_pipeline(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = RestartPipelineSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/PostCollectors" => {
					#[allow(non_camel_case_types)]
					struct PostCollectorsSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::CollectorConfig> for PostCollectorsSvc<T>
					{
						type Response = super::CollectorConfigResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::CollectorConfig>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).post_collectors(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = PostCollectorsSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/DeleteCollectors" => {
					#[allow(non_camel_case_types)]
					struct DeleteCollectorsSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::DeleteCollectorRequest> for DeleteCollectorsSvc<T>
					{
						type Response = super::DeleteCollectorResponse;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::DeleteCollectorRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).delete_collectors(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = DeleteCollectorsSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/ListCollectors" => {
					#[allow(non_camel_case_types)]
					struct ListCollectorsSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
					impl<T: SemanticsServiceGrpc>
						tonic::server::UnaryService<super::ListCollectorRequest> for ListCollectorsSvc<T>
					{
						type Response = super::ListCollectorConfig;
						type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
						fn call(
							&mut self,
							request: tonic::Request<super::ListCollectorRequest>,
						) -> Self::Future {
							let inner = Arc::clone(&self.0);
							let fut = async move { (*inner).list_collectors(request).await };
							Box::pin(fut)
						}
					}
					let accept_compression_encodings = self.accept_compression_encodings;
					let send_compression_encodings = self.send_compression_encodings;
					let max_decoding_message_size = self.max_decoding_message_size;
					let max_encoding_message_size = self.max_encoding_message_size;
					let inner = self.inner.clone();
					let fut = async move {
						let inner = inner.0;
						let method = ListCollectorsSvc(inner);
						let codec = tonic::codec::ProstCodec::default();
						let mut grpc = tonic::server::Grpc::new(codec)
							.apply_compression_config(
								accept_compression_encodings,
								send_compression_encodings,
							)
							.apply_max_message_size_config(
								max_decoding_message_size,
								max_encoding_message_size,
							);
						let res = grpc.unary(method, req).await;
						Ok(res)
					};
					Box::pin(fut)
				},
				"/querent.semantics.SemanticsService/ListPipelineInfo" => {
                    #[allow(non_camel_case_types)]
                    struct ListPipelineInfoSvc<T: SemanticsServiceGrpc>(pub Arc<T>);
                    impl<
                        T: SemanticsServiceGrpc,
                    > tonic::server::UnaryService<super::EmptyList>
                    for ListPipelineInfoSvc<T> {
                        type Response = super::PipelineRequestInfoList;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EmptyList>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).list_pipeline_info(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListPipelineInfoSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
				},
				_ => Box::pin(async move {
					Ok(http::Response::builder()
						.status(200)
						.header("grpc-status", "12")
						.header("content-type", "application/grpc")
						.body(empty_body())
						.unwrap())
				}),
			}
		}
	}
	impl<T: SemanticsServiceGrpc> Clone for SemanticsServiceGrpcServer<T> {
		fn clone(&self) -> Self {
			let inner = self.inner.clone();
			Self {
				inner,
				accept_compression_encodings: self.accept_compression_encodings,
				send_compression_encodings: self.send_compression_encodings,
				max_decoding_message_size: self.max_decoding_message_size,
				max_encoding_message_size: self.max_encoding_message_size,
			}
		}
	}
	impl<T: SemanticsServiceGrpc> Clone for _Inner<T> {
		fn clone(&self) -> Self {
			Self(Arc::clone(&self.0))
		}
	}
	impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{:?}", self.0)
		}
	}
	impl<T: SemanticsServiceGrpc> tonic::server::NamedService for SemanticsServiceGrpcServer<T> {
		const NAME: &'static str = "querent.semantics.SemanticsService";
	}
}
