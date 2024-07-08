use std::{collections::HashMap, sync::Arc, time::Duration};

use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use milvus::{
	client::{Client as MilvusClient, ConsistencyLevel},
	collection::SearchOption,
	index::{IndexParams, MetricType},
};
use proto::semantics::MilvusConfig;

use crate::{storage::Storage, DiscoveredKnowledge, StorageError, StorageErrorKind, StorageResult};
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
		collection_id: String,
		_payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		let collection_name = format!("pipeline_{}", collection_id);

		for (id, source, image_id, payload) in _payload {
			let result = self
				.insert_or_create_collection(
					collection_name.as_str(),
					id,
					source,
					image_id,
					payload,
				)
				.await;
			if let Err(err) = result {
				log::error!("Vector insertion failed: {:?}", err);
				return Err(err);
			}
		}
		Ok(())
	}

	/// Get discovered knowledge
	async fn get_discovered_data(
		&self,
		_session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>> {
		Ok(vec![])
	}

	async fn similarity_search_l2(
		&self,
		_session_id: String,
		_query: String,
		collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		_offset: i64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let collection_name = format!("pipeline_{}", collection_id);
		let collection = self.client.get_collection(collection_name.as_str()).await;
		let mut s_options = SearchOption::new();
		let search_options = s_options.set_consistency_level(ConsistencyLevel::Strong);
		match collection {
			Ok(collection) => {
				let mut params = HashMap::new();
				params.insert("nlist".to_string(), "128".to_string());
				let index_param = IndexParams::new(
					"example_index".to_string(),
					milvus::index::IndexType::IvfFlat,
					MetricType::L2,
					params,
				);
				match collection.create_index("embeddings", index_param).await {
					Ok(_) => {},
					Err(err) => {
						log::error!("Indexingfailed: {:?}", err);
						return Err(StorageError {
							kind: StorageErrorKind::CollectionBuilding,
							source: Arc::new(anyhow::Error::from(err)),
						});
					},
				}
				match collection.load(1).await {
					Ok(_) => {},
					Err(err) => {
						log::error!("Collection load failed: {:?}", err);
						return Err(StorageError {
							kind: StorageErrorKind::CollectionBuilding,
							source: Arc::new(anyhow::Error::from(err)),
						});
					},
				}
				let result = collection
					.search(
						vec![payload.clone().into()],
						"embeddings",
						max_results,
						MetricType::L2,
						vec!["id", "knowledge", "document", "sentence"],
						search_options,
					)
					.await;
				match result {
					Ok(result) => {
						let mut results = Vec::new();
						for search_res in result {
							let mut doc_payload = DocumentPayload::default();
							for field in search_res.field {
								match field.name.as_str() {
									// "knowledge" => {
									// 	let knowledge: Vec<String> =
									// 		field.value.try_into().unwrap_or_default();
									// 	doc_payload.knowledge = knowledge.join(" ");
									// 	// split at _ to get subject, predicate, object
									// 	let knowledge_parts: Vec<&str> =
									// 		// doc_payload.knowledge.split('_').collect();
									// 	if knowledge_parts.len() != 3 {
									// 		log::error!(
									// 			"Knowledge triple is not in correct format: {:?}",
									// 			// doc_payload.knowledge
									// 		);
									// 		continue;
									// 	}
									// 	doc_payload.subject = knowledge_parts[0].to_string();
									// 	// doc_payload.predicate = knowledge_parts[1].to_string();
									// 	doc_payload.object = knowledge_parts[2].to_string();
									// },
									"document" => {
										let document: Vec<String> =
											field.value.try_into().unwrap_or_default();
										doc_payload.doc_id = document.join(" ");
									},
									"sentence" => {
										let sentence: Vec<String> =
											field.value.try_into().unwrap_or_default();
										doc_payload.sentence = sentence.join(" ");
									},
									_ => {},
								}
							}
							results.push(doc_payload);
						}
						Ok(results)
					},
					Err(err) => {
						log::error!("Query failed: {:?}", err);
						Err(StorageError {
							kind: StorageErrorKind::Query,
							source: Arc::new(anyhow::Error::from(err)),
						})
					},
				}
			},
			Err(err) => {
				log::error!("Collection retrieval failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::CollectionRetrieval,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your insert_graph implementation here
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your index_triples implementation here
		Ok(())
	}

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		_payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		// Your insert_discovered_knowledge implementation here
		Ok(())
	}

	async fn traverse_metadata_table(
		&self,
		_filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>> {
		Ok(vec![])
	}

	/// Store key value pair
	async fn store_secret(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_secret(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	//Delete the key value pair
	async fn delete_secret(&self, _key: &String) -> StorageResult<()> {
		Ok(())
	}

	//Get all collectors key value pairs
	async fn get_all_secrets(&self) -> StorageResult<Vec<(String, String)>> {
		Ok(Vec::new())
	}

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(&self, _pipeline: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get semantic pipeline by id
	async fn get_pipeline(&self, _pipeline_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, _pipeline_id: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Discovery session ran by this node
	async fn set_discovery_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Discovery session by id
	async fn get_discovery_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(&self) -> StorageResult<Vec<String>> {
		Ok(Vec::new())
	}

	/// Set Insight session ran by this node
	async fn set_insight_session(&self, _session: &String) -> StorageResult<()> {
		Ok(())
	}

	/// Get Insight session by id
	async fn get_insight_session(&self, _session_id: &String) -> StorageResult<Option<String>> {
		Ok(None)
	}
}

impl MilvusStorage {
	async fn insert_or_create_collection(
		&self,
		collection_name: &str, // this is workflow id
		id: &str,              // this is document id
		source: &str,          // this is document source
		image_id: &Option<String>,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let collection = self.client.get_collection(collection_name).await;
		match collection {
			Ok(collection) => {
				log::debug!("Collection found: {:?}", collection);
				self.insert_into_collection(&collection, id, source, image_id, payload).await
			},
			Err(_err) => {
				log::error!("Error in milvus client: {:?}", _err);
				self.create_and_insert_collection(collection_name, id, source, image_id, payload)
					.await
			},
		}
	}

	async fn insert_into_collection(
		&self,
		_collection: &milvus::collection::Collection,
		_id: &str,
		_source: &str,
		_image_id: &Option<String>,
		_payload: &VectorPayload,
	) -> StorageResult<()> {
		Ok(())
		// let has_partition = collection.has_partition(&payload.namespace).await;
		// match has_partition {
		// 	Ok(has_partition) =>
		// 		if !has_partition {
		// 			let partition = collection.create_partition(&payload.namespace).await;
		// 			match partition {
		// 				Ok(partition) => {
		// 					log::debug!("Partition created: {:?}", partition);
		// 				},
		// 				Err(err) => {
		// 					log::error!("Partition creation failed: {:?}", err);
		// 					return Err(StorageError {
		// 						kind: StorageErrorKind::PartitionCreation,
		// 						source: Arc::new(anyhow::Error::from(err)),
		// 					});
		// 				},
		// 			}
		// 		},
		// 	Err(err) => {
		// 		log::error!("Partition check failed: {:?}", err);
		// 		return Err(StorageError {
		// 			kind: StorageErrorKind::PartitionCreation,
		// 			source: Arc::new(anyhow::Error::from(err)),
		// 		});
		// 	},
		// }

		// let knowledge_field = FieldColumn::new(
		// 	collection.schema().get_field("knowledge").unwrap(),
		// 	ValueVec::String(vec![payload.id.clone()]),
		// );
		// let relationship_field = FieldColumn::new(
		// 	collection.schema().get_field("relationship").unwrap(),
		// 	ValueVec::String(vec![payload.namespace.clone()]),
		// );
		// let document_field = FieldColumn::new(
		// 	collection.schema().get_field("document").unwrap(),
		// 	ValueVec::String(vec![id.to_string()]),
		// );
		// let document_source_field = FieldColumn::new(
		// 	collection.schema().get_field("document_source").unwrap(),
		// 	ValueVec::String(vec![source.to_string()]),
		// );
		// let embeddings_field = FieldColumn::new(
		// 	collection.schema().get_field("embeddings").unwrap(),
		// 	payload.embeddings.clone(),
		// );

		// let sentence_field = FieldColumn::new(
		// 	collection.schema().get_field("sentence").unwrap(),
		// 	ValueVec::String(vec![payload.sentence.clone().unwrap_or_default()]),
		// );
		// let image_id_field = FieldColumn::new(
		// 	collection.schema().get_field("image_id").unwrap(),
		// 	ValueVec::String(vec![image_id.clone().unwrap_or_default()]),
		// );

		// let records = vec![
		// 	knowledge_field,
		// 	relationship_field,
		// 	document_field,
		// 	document_source_field,
		// 	embeddings_field,
		// 	sentence_field,
		// 	image_id_field,
		// ];
		// let insert_result = collection.insert(records, Some(payload.namespace.as_str())).await;
		// match insert_result {
		// 	Ok(insert_result) => {
		// 		log::debug!("Insert result: {:?}", insert_result);
		// 		let flushed_collection = collection.flush().await;
		// 		match flushed_collection {
		// 			Ok(flushed_collection) => {
		// 				log::debug!("Collection flushed: {:?}", flushed_collection);
		// 				Ok(())
		// 			},
		// 			Err(err) => {
		// 				log::error!("Flush failed: {:?}", err);
		// 				Err(StorageError {
		// 					kind: StorageErrorKind::Insertion,
		// 					source: Arc::new(anyhow::Error::from(err)),
		// 				})
		// 			},
		// 		}
		// 	},
		// 	Err(err) => {
		// 		log::error!("Insert failed: {:?}", err);
		// 		Err(StorageError {
		// 			kind: StorageErrorKind::Insertion,
		// 			source: Arc::new(anyhow::Error::from(err)),
		// 		})
		// 	},
		// }
	}

	async fn create_and_insert_collection(
		&self,
		_collection_name: &str,
		_id: &str,
		_source: &str,
		_image_id: &Option<String>,
		_payload: &VectorPayload,
	) -> StorageResult<()> {
		Ok(())
		// let description = format!("Semantic collection adhering to s->p->o ={:?}", payload.id);
		// let new_coll = CollectionSchemaBuilder::new(collection_name, description.as_str())
		// 	.add_field(FieldSchema::new_primary_int64("id", "auto id for each vector", true))
		// 	.add_field(FieldSchema::new_varchar("knowledge", "subject, predicate, object", 280))
		// 	.add_field(FieldSchema::new_varchar(
		// 		"relationship",
		// 		"predicate associated with embedding",
		// 		280,
		// 	))
		// 	.add_field(FieldSchema::new_varchar(
		// 		"document",
		// 		"document associated with embedding",
		// 		280,
		// 	))
		// 	.add_field(FieldSchema::new_varchar("document_source", "source of the document", 280))
		// 	.add_field(FieldSchema::new_float_vector(
		// 		"embeddings",
		// 		"semantic vector embeddings",
		// 		payload.size as i64,
		// 	))
		// 	.add_field(FieldSchema::new_varchar(
		// 		"sentence",
		// 		"sentence associated with embedding",
		// 		5000,
		// 	))
		// 	.add_field(FieldSchema::new_varchar("image_id", "Unique id of each image", 20))
		// 	.build();

		// match new_coll {
		// 	Ok(new_coll) => {
		// 		let collection = self.client.create_collection(new_coll, None).await;
		// 		match collection {
		// 			Ok(collection) => {
		// 				log::debug!("Collection created: {:?}", collection);
		// 				self.insert_into_collection(&collection, id, source, image_id, payload)
		// 					.await
		// 			},
		// 			Err(err) => {
		// 				log::error!("Collection creation failed: {:?}", err);
		// 				Err(StorageError {
		// 					kind: StorageErrorKind::CollectionCreation,
		// 					source: Arc::new(anyhow::Error::from(err)),
		// 				})
		// 			},
		// 		}
		// 	},
		// 	Err(err) => {
		// 		log::error!("Collection builder failed: {:?}", err);
		// 		Err(StorageError {
		// 			kind: StorageErrorKind::CollectionBuilding,
		// 			source: Arc::new(anyhow::Error::from(err)),
		// 		})
		// 	},
		// }
	}
}

// #[cfg(test)]
// mod tests {
// 	use proto::semantics::StorageType;

// 	use super::*;

// 	#[tokio::test]
// 	async fn test_insert_vector() {
// 		// Create a MilvusStorage instance for testing with a local URL
// 		const URL: &str = "http://localhost:19530";
// 		let storage_config = MilvusConfig {
// 			name: "milvus".to_string(),
// 			storage_type: Some(StorageType::Vector),
// 			url: URL.to_string(),
// 			username: "".to_string(),
// 			password: "".to_string(),
// 		};
// 		let storage = MilvusStorage::new(storage_config).await;
// 		if let Err(err) = storage {
// 			log::error!("MilvusStorage creation failed: {:?}", err);
// 			return;
// 		}
// 		// Prepare test data
// 		let payload = VectorPayload {
// 			id: "test_id".to_string(),
// 			namespace: "test_namespace".to_string(),
// 			size: 10,
// 			embeddings: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
// 			sentence: Some("test_sentence".to_string()),
// 			document_source: Some("file://folder".to_string()),
// 			blob: Some("base64encodedimage".to_string()),
// 		};

// 		// Call the insert_vector function with the test data
// 		let _result = storage
// 			.unwrap()
// 			.insert_vector(
// 				"qflow_id".to_string(),
// 				&vec![(
// 					"test_id".to_string(),
// 					"test_source".to_string(),
// 					Some("123456".to_string()),
// 					payload,
// 				)],
// 			)
// 			.await;

// 		// Assert that the result is Ok indicating successful insertion
// 		// Uncomment to test when local Milvus is running
// 		//assert!(_result.is_ok(), "Insertion failed: {:?}", _result);
// 	}
// }
