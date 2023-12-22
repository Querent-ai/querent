use std::sync::Arc;

use common::{SemanticKnowledgePayload, VectorPayload};
use milvus::client::Client as MilvusClient;

use crate::storage::Storage;
use async_trait::async_trait;

pub struct MilvusStorage {
	pub client: Arc<MilvusClient>,
	pub url: String,
}

impl MilvusStorage {
	pub async fn new(url: String) -> Self {
		let client_res = MilvusClient::new(url.clone()).await;
		match client_res {
			Ok(client) => {
				log::info!("Milvus client created");
				MilvusStorage { client: Arc::new(client), url }
			},
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
		let _ = self.client.clone().list_collections().await?;
		Ok(())
	}

	async fn insert_vector(&self, _payload: Vec<(String, VectorPayload)>) -> anyhow::Result<()> {
		Ok(())
	}

	async fn insert_graph(
		&self,
		_payload: Vec<(String, SemanticKnowledgePayload)>,
	) -> anyhow::Result<()> {
		Err(anyhow::anyhow!("Not implemented"))
	}
}
