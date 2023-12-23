use std::sync::Arc;

use common::{SemanticKnowledgePayload, VectorPayload};
use milvus::{
	client::Client as MilvusClient,
	data::FieldColumn,
	schema::{CollectionSchemaBuilder, FieldSchema},
	value::ValueVec,
};

use crate::{storage::Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;

pub struct MilvusStorage {
	pub client: Arc<MilvusClient>,
	pub url: String,
}

impl MilvusStorage {
	pub async fn new(url: String) -> Self {
		let client_res = MilvusClient::new(url.clone()).await;
		match client_res {
			Ok(client) => MilvusStorage { client: Arc::new(client), url },
			Err(err) => {
				log::error!("Milvus client creation failed: {:?}", err);
				panic!("Milvus client creation failed: {:?}", err);
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

	async fn insert_vector(&self, _payload: Vec<(String, VectorPayload)>) -> StorageResult<()> {
		for (id, payload) in _payload {
			let result = self.insert_or_create_collection(&payload.namespace, &id, &payload).await;
			if let Err(err) = result {
				log::error!("Vector insertion failed: {:?}", err);
				return Err(err);
			}
		}
		Ok(())
	}

	async fn insert_graph(
		&self,
		_payload: Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your insert_graph implementation here
		Ok(())
	}
}

impl MilvusStorage {
	async fn insert_or_create_collection(
		&self,
		collection_name: &str,
		id: &str,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let collection = self.client.get_collection(collection_name).await;
		match collection {
			Ok(collection) => {
				log::debug!("Collection found: {:?}", collection);
				self.insert_into_collection(&collection, id, payload).await
			},
			Err(_) => {
				log::debug!("Collection not found, creating: {:?}", collection_name);
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

		let records = vec![knowledge_field, relationship_field, document_field, embeddings_field];
		let insert_result = collection.insert(records, None).await;
		match insert_result {
			Ok(insert_result) => {
				log::debug!("Insert result: {:?}", insert_result);
				Ok(())
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
			.add_field(FieldSchema::new_string("knowledge", "subject, predicate, object"))
			.add_field(FieldSchema::new_string(
				"relationship",
				"predicate associated with embedding",
			))
			.add_field(FieldSchema::new_string("document", "document associated with embedding"))
			.add_field(FieldSchema::new_float_vector(
				"embeddings",
				"semantic vector embeddings",
				payload.size as i64,
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
