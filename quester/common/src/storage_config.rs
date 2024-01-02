use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MilvusConfig {
	pub url: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Neo4jConfig {
	pub url: String,
	pub username: String,
	pub password: String,
	pub db_name: String,
	pub max_connections: usize,
	pub fetch_size: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PostgresConfig {
	pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageConfig {
	/// postgresql://user:password@host:port/database
	Postgres(PostgresConfig),
	/// Milvus storage
	Milvus(MilvusConfig),
	/// Neo4j storage
	Neo4j(Neo4jConfig),
}

impl StorageConfig {
	pub fn redacted(&self) -> Self {
		match self {
			Self::Postgres(url) => Self::Postgres(PostgresConfig { url: url.url.clone() }),
			Self::Milvus(config) => Self::Milvus(MilvusConfig {
				url: config.url.clone(),
				username: "<redacted>".to_owned(),
				password: "<redacted>".to_owned(),
			}),
			Self::Neo4j(config) => Self::Neo4j(Neo4jConfig {
				url: config.url.clone(),
				username: "<redacted>".to_owned(),
				password: "<redacted>".to_owned(),
				db_name: config.db_name.clone(),
				max_connections: config.max_connections,
				fetch_size: config.fetch_size,
			}),
		}
	}

	pub fn postgres(url: &str) -> Self {
		Self::Postgres(PostgresConfig { url: url.to_owned() })
	}

	pub fn milvus(url: &str, username: &str, password: &str) -> Self {
		Self::Milvus(MilvusConfig {
			url: url.to_owned(),
			username: username.to_owned(),
			password: password.to_owned(),
		})
	}

	pub fn neo4j(
		url: &str,
		username: &str,
		password: &str,
		db_name: &str,
		max_connections: usize,
		fetch_size: usize,
	) -> Self {
		Self::Neo4j(Neo4jConfig {
			url: url.to_owned(),
			username: username.to_owned(),
			password: password.to_owned(),
			db_name: db_name.to_owned(),
			max_connections,
			fetch_size,
		})
	}

	pub fn as_postgres(&self) -> Option<&PostgresConfig> {
		match self {
			Self::Postgres(config) => Some(config),
			_ => None,
		}
	}

	pub fn as_milvus(&self) -> Option<&MilvusConfig> {
		match self {
			Self::Milvus(config) => Some(config),
			_ => None,
		}
	}

	pub fn as_neo4j(&self) -> Option<&Neo4jConfig> {
		match self {
			Self::Neo4j(config) => Some(config),
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_deserialize_milvus_config() {
		let json = r#"
            {
                "url": "http://milvus-url",
                "username": "milvus-user",
                "password": "milvus-pass"
            }
        "#;

		let config: MilvusConfig = serde_json::from_str(json).unwrap();
		assert_eq!(config.url, "http://milvus-url");
		assert_eq!(config.username, "milvus-user");
		assert_eq!(config.password, "milvus-pass");
	}

	#[test]
	fn test_deserialize_neo4j_config() {
		let json = r#"
            {
                "url": "http://neo4j-url",
                "username": "neo4j-user",
                "password": "neo4j-pass",
                "db_name": "neo4j-db",
                "max_connections": 10,
                "fetch_size": 100
            }
        "#;

		let config: Neo4jConfig = serde_json::from_str(json).unwrap();
		assert_eq!(config.url, "http://neo4j-url");
		assert_eq!(config.username, "neo4j-user");
		assert_eq!(config.password, "neo4j-pass");
		assert_eq!(config.db_name, "neo4j-db");
		assert_eq!(config.max_connections, 10);
		assert_eq!(config.fetch_size, 100);
	}

	#[test]
	fn test_deserialize_postgres_config() {
		let json = r#"
            {
                "url": "postgresql://user:password@host:5432/database"
            }
        "#;

		let config: PostgresConfig = serde_json::from_str(json).unwrap();
		assert_eq!(config.url, "postgresql://user:password@host:5432/database");
	}

	#[test]
	fn test_deserialize_storage_config() {
		let json = r#"
            {
                "Postgres": {
                    "url": "postgresql://user:password@host:5432/database"
                }
            }
        "#;

		let config: StorageConfig = serde_json::from_str(json).unwrap();
		assert_eq!(
			config,
			StorageConfig::Postgres(PostgresConfig {
				url: "postgresql://user:password@host:5432/database".to_owned()
			})
		);
	}

	#[test]
	fn test_redacted_storage_config() {
		let original = StorageConfig::milvus("http://milvus-url", "milvus-user", "milvus-pass");
		let redacted = original.redacted();

		assert_eq!(
			redacted,
			StorageConfig::Milvus(MilvusConfig {
				url: "http://milvus-url".to_owned(),
				username: "<redacted>".to_owned(),
				password: "<redacted>".to_owned()
			})
		);
	}

	#[test]
	fn test_serialize_deserialize_storage_config() {
		let original = StorageConfig::neo4j(
			"http://neo4j-url",
			"neo4j-user",
			"neo4j-pass",
			"neo4j-db",
			10,
			100,
		);
		let json = serde_json::to_string(&original).unwrap();
		let deserialized: StorageConfig = serde_json::from_str(&json).unwrap();

		assert_eq!(original, deserialized);
	}
}
