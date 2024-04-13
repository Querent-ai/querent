use std::{sync::Arc, time::Duration};

use common::{SemanticKnowledgePayload, VectorPayload};
use milvus::{
	client::Client as MilvusClient,
	data::FieldColumn,
	schema::{CollectionSchemaBuilder, FieldSchema},
	value::ValueVec,
};
use proto::semantics::MilvusConfig;

use crate::{storage::Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;

pub struct MilvusStorage {
	pub client: Arc<MilvusClient>,
	pub config: MilvusConfig,
}

impl MilvusStorage {
	pub async fn new(config: MilvusConfig) -> StorageResult<Self> {
		let client_res = MilvusClient::with_timeout(
			config.url.clone(),
			Duration::from_secs(5),
			Some(config.username.clone()),
			Some(config.password.clone()),
		)
		.await;
		match client_res {
			Ok(client) => Ok(MilvusStorage { client: Arc::new(client), config }),
			Err(err) => {
				log::error!("Milvus client creation failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}
}

#[async_trait]
impl Storage for MilvusStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.client.list_collections().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, VectorPayload)>,
	) -> StorageResult<()> {
		let collection_name = format!("pipeline_{}", _collection_id);

		for (id, payload) in _payload {
			let result =
				self.insert_or_create_collection(collection_name.as_str(), id, payload).await;
			if let Err(err) = result {
				log::error!("Vector insertion failed: {:?}", err);
				return Err(err);
			}
		}
		Ok(())
	}

	async fn insert_graph(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your insert_graph implementation here
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your index_triples implementation here
		Ok(())
	}

	/// Store key value pair
	async fn store_kv(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_kv(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}
}

impl MilvusStorage {
	async fn insert_or_create_collection(
		&self,
		collection_name: &str, // this is workflow id
		id: &str,              // this is document id
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let collection = self.client.get_collection(collection_name).await;
		match collection {
			Ok(collection) => {
				log::debug!("Collection found: {:?}", collection);
				self.insert_into_collection(&collection, id, payload).await
			},
			Err(_err) => {
				log::error!("Error in milvus client: {:?}", _err);
				self.create_and_insert_collection(collection_name, id, payload).await
			},
		}
	}

	async fn insert_into_collection(
		&self,
		collection: &milvus::collection::Collection,
		id: &str,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let has_partition = collection.has_partition(&payload.namespace).await;
		match has_partition {
			Ok(has_partition) =>
				if !has_partition {
					let partition = collection.create_partition(&payload.namespace).await;
					match partition {
						Ok(partition) => {
							log::debug!("Partition created: {:?}", partition);
						},
						Err(err) => {
							log::error!("Partition creation failed: {:?}", err);
							return Err(StorageError {
								kind: StorageErrorKind::PartitionCreation,
								source: Arc::new(anyhow::Error::from(err)),
							});
						},
					}
				},
			Err(err) => {
				log::error!("Partition check failed: {:?}", err);
				return Err(StorageError {
					kind: StorageErrorKind::PartitionCreation,
					source: Arc::new(anyhow::Error::from(err)),
				});
			},
		}

		let knowledge_field = FieldColumn::new(
			collection.schema().get_field("knowledge").unwrap(),
			ValueVec::String(vec![payload.id.clone()]),
		);
		let relationship_field = FieldColumn::new(
			collection.schema().get_field("relationship").unwrap(),
			ValueVec::String(vec![payload.namespace.clone()]),
		);
		let document_field = FieldColumn::new(
			collection.schema().get_field("document").unwrap(),
			ValueVec::String(vec![id.to_string()]),
		);
		let embeddings_field = FieldColumn::new(
			collection.schema().get_field("embeddings").unwrap(),
			payload.embeddings.clone(),
		);

		let sentence_field = FieldColumn::new(
			collection.schema().get_field("sentence").unwrap(),
			ValueVec::String(vec![payload.sentence.clone().unwrap_or_default()]),
		);

		let records = vec![
			knowledge_field,
			relationship_field,
			document_field,
			embeddings_field,
			sentence_field,
		];
		let insert_result = collection.insert(records, Some(payload.namespace.as_str())).await;
		match insert_result {
			Ok(insert_result) => {
				log::debug!("Insert result: {:?}", insert_result);
				let flushed_collection = collection.flush().await;
				match flushed_collection {
					Ok(flushed_collection) => {
						log::debug!("Collection flushed: {:?}", flushed_collection);
						Ok(())
					},
					Err(err) => {
						log::error!("Flush failed: {:?}", err);
						Err(StorageError {
							kind: StorageErrorKind::Insertion,
							source: Arc::new(anyhow::Error::from(err)),
						})
					},
				}
			},
			Err(err) => {
				log::error!("Insert failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Insertion,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}

	async fn create_and_insert_collection(
		&self,
		collection_name: &str,
		id: &str,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let description = format!("Semantic collection adhering to s->p->o ={:?}", payload.id);
		let new_coll = CollectionSchemaBuilder::new(collection_name, description.as_str())
			.add_field(FieldSchema::new_primary_int64("id", "auto id for each vector", true))
			.add_field(FieldSchema::new_varchar("knowledge", "subject, predicate, object", 280))
			.add_field(FieldSchema::new_varchar(
				"relationship",
				"predicate associated with embedding",
				280,
			))
			.add_field(FieldSchema::new_varchar(
				"document",
				"document associated with embedding",
				280,
			))
			.add_field(FieldSchema::new_float_vector(
				"embeddings",
				"semantic vector embeddings",
				payload.size as i64,
			))
			.add_field(FieldSchema::new_varchar(
				"sentence",
				"sentence associated with embedding",
				5000,
			))
			.build();

		match new_coll {
			Ok(new_coll) => {
				let collection = self.client.create_collection(new_coll, None).await;
				match collection {
					Ok(collection) => {
						log::debug!("Collection created: {:?}", collection);
						self.insert_into_collection(&collection, id, payload).await
					},
					Err(err) => {
						log::error!("Collection creation failed: {:?}", err);
						Err(StorageError {
							kind: StorageErrorKind::CollectionCreation,
							source: Arc::new(anyhow::Error::from(err)),
						})
					},
				}
			},
			Err(err) => {
				log::error!("Collection builder failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::CollectionBuilding,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use proto::semantics::StorageType;

	use super::*;

	#[tokio::test]
	async fn test_insert_vector() {
		// Create a MilvusStorage instance for testing with a local URL
		const URL: &str = "http://localhost:19530";
		let storage_config = MilvusConfig {
			name: "milvus".to_string(),
			storage_type: Some(StorageType::Vector),
			url: URL.to_string(),
			username: "".to_string(),
			password: "".to_string(),
		};
		let storage = MilvusStorage::new(storage_config).await;
		if let Err(err) = storage {
			log::error!("MilvusStorage creation failed: {:?}", err);
			return;
		}
		// Prepare test data
		let payload = VectorPayload {
			id: "test_id".to_string(),
			namespace: "test_namespace".to_string(),
			size: 10,
			embeddings: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
			sentence: Some("test_sentence".to_string()),
		};

		// Call the insert_vector function with the test data
		let _result = storage
			.unwrap()
			.insert_vector("qflow_id".to_string(), &vec![("test_id".to_string(), payload)])
			.await;

		// Assert that the result is Ok indicating successful insertion
		// Uncomment to test when local Milvus is running
		//assert!(_result.is_ok(), "Insertion failed: {:?}", _result);
	}
}
