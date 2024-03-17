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
    #[prost(enumeration = "StorageType", tag = "2")]
    pub storage_type: i32,
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
    #[prost(enumeration = "StorageType", tag = "2")]
    pub storage_type: i32,
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
    #[prost(enumeration = "StorageType", tag = "2")]
    pub storage_type: i32,
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
/// Enum for defining different types of storage.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum StorageType {
    /// Default value, representing an unknown storage type.
    Unknown = 0,
    Index = 1,
    Vector = 2,
    Graph = 3,
}
impl StorageType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            StorageType::Unknown => "UNKNOWN",
            StorageType::Index => "INDEX",
            StorageType::Vector => "VECTOR",
            StorageType::Graph => "GRAPH",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNKNOWN" => Some(Self::Unknown),
            "INDEX" => Some(Self::Index),
            "VECTOR" => Some(Self::Vector),
            "GRAPH" => Some(Self::Graph),
            _ => None,
        }
    }
}
