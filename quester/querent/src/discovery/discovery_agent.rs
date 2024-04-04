use crate::{
	agent::{AgentExecutor, ConversationalAgent, ConversationalAgentBuilder},
	chain::options::ChainCallOptions,
	discovery::chain::chain_trait::Chain,
	llm::{OpenAI, OpenAIModel},
	memory::WindowBufferMemory,
	prompt::{PromptTemplate, TemplateFormat},
	prompt_args,
	tools::CommandExecutor,
};
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tokio::runtime::Handle;

pub struct DiscoveryAgent {
	agent_id: String,
	timestamp: u64,
	event_storages: HashMap<EventType, Arc<dyn Storage>>,
	index_storages: Vec<Arc<dyn Storage>>,
	discovery_agent_params: DiscoverySessionRequest,
	discover_agent: Option<AgentExecutor<ConversationalAgent>>,
	template: Option<PromptTemplate>,
	embedding_model: Option<TextEmbedding>,
}

impl DiscoveryAgent {
	pub fn new(
		agent_id: String,
		timestamp: u64,
		event_storages: HashMap<EventType, Arc<dyn Storage>>,
		index_storages: Vec<Arc<dyn Storage>>,
		discovery_agent_params: DiscoverySessionRequest,
	) -> Self {
		Self {
			agent_id,
			timestamp,
			event_storages,
			index_storages,
			discovery_agent_params,
			discover_agent: None,
			template: None,
			embedding_model: None,
		}
	}

	pub fn get_timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn set_timestamp(&mut self, timestamp: u64) {
		self.timestamp = timestamp;
	}

	pub fn get_agent_id(&self) -> String {
		self.agent_id.clone()
	}
}

#[async_trait]
impl Actor for DiscoveryAgent {
	type ObservableState = ();

	async fn initialize(&mut self, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		let template = PromptTemplate::new(
            "Given the query: {{query}}
        
            You are given the following graph data: {{graph_data}}.
        
            Please summarize the key insights and deductions that can be made from the graph data in relation to the query. Your summary should cover:
        
            1. The most relevant nodes and their relationships to the query.
            2. The overall structure and connectivity of the graph as it pertains to the query.
            3. Any significant patterns, clusters, or communities within the graph that are relevant to the query.
            4. Potential use cases or applications based on the graph structure in the context of the query.
            5. Any other important insights or observations related to the query.
        
            Provide a concise yet comprehensive summary that captures the essential information and insights that can be derived from the graph data in the context of the given query.".to_string(),
            vec!["query".to_string(), "graph_data".to_string()],
            TemplateFormat::Jinja2,
        );

		let llm = OpenAI::default().with_model(OpenAIModel::Gpt35);
		let memory =
			WindowBufferMemory::new(self.discovery_agent_params.max_message_memory_size as usize);
		let command_executor = CommandExecutor::default();
		let agent = ConversationalAgentBuilder::new()
			.tools(&[Arc::new(command_executor)])
			.options(ChainCallOptions::new().with_max_tokens(1000))
			.build(llm)
			.map_err(|e| {
				log::error!("Failed to build discovery agent: {}", e);
				ActorExitStatus::Failure(Arc::new(anyhow::anyhow!(
					"Failed to build discovery agent"
				)))
			})?;

		let executor = AgentExecutor::from_agent(agent).with_memory(memory.into());
		self.discover_agent = Some(executor);
		self.template = Some(template);
		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		})?;

		self.embedding_model = Some(embedding_model);
		Ok(())
	}

	fn observable_state(&self) -> Self::ObservableState {}

	fn name(&self) -> String {
		format!("DiscoveryAgent-{}", self.agent_id)
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(self.discovery_agent_params.max_message_memory_size as usize)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::NonBlocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed |
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Panicked => return Ok(()),
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				log::info!("Discovery agent {} exiting with success", self.agent_id);
			},
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<DiscoveryRequest> for DiscoveryAgent {
	type Reply = Result<DiscoveryResponse, DiscoveryError>;

	async fn handle(
		&mut self,
		message: DiscoveryRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		if self.discover_agent.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent is not initialized".to_string(),
			)));
		}
		if self.template.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent template is not initialized".to_string(),
			)));
		}

		if self.embedding_model.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent embedding model is not initialized".to_string(),
			)));
		}
		let current_agent = self.discover_agent.as_ref().unwrap();
		let template = self.template.as_ref().unwrap();
		let embedder = self.embedding_model.as_ref().unwrap();
		let embeddings = embedder.embed(vec![message.query.clone()], None)?;
		let current_query_embedding = embeddings[0].clone();
		use serde_json::{json, Value};

		let dummy_graph_data_geologic_deposition: Value = json!({
			"nodes": [
				{
					"id": 1,
					"label": "Alluvial Fan",
					"type": "depositional_feature"
				},
				{
					"id": 2,
					"label": "Delta",
					"type": "depositional_feature"
				},
				{
					"id": 3,
					"label": "Fluvial Deposit",
					"type": "depositional_feature"
				},
				{
					"id": 4,
					"label": "Glacial Deposit",
					"type": "depositional_feature"
				},
				{
					"id": 5,
					"label": "Aeolian Deposit",
					"type": "depositional_feature"
				},
				{
					"id": 6,
					"label": "Lacustrine Deposit",
					"type": "depositional_feature"
				},
				{
					"id": 7,
					"label": "Arid Climate",
					"type": "climate"
				},
				{
					"id": 8,
					"label": "Humid Climate",
					"type": "climate"
				},
				{
					"id": 9,
					"label": "Glacial Climate",
					"type": "climate"
				}
			],
			"edges": [
				{
					"source": 1,
					"target": 7,
					"type": "forms_in"
				},
				{
					"source": 2,
					"target": 8,
					"type": "forms_in"
				},
				{
					"source": 3,
					"target": 8,
					"type": "forms_in"
				},
				{
					"source": 4,
					"target": 9,
					"type": "forms_in"
				},
				{
					"source": 5,
					"target": 7,
					"type": "forms_in"
				},
				{
					"source": 6,
					"target": 8,
					"type": "forms_in"
				}
			]
		});
		let input_variables = prompt_args! {
			"query" => message.query.clone(),
			"graph_data" => dummy_graph_data_geologic_deposition,
		};

		match current_agent.invoke(input_variables).await {
			Ok(result) => {
				println!("Result: {:?}", result);
			},
			Err(e) => panic!("Error invoking LLMChain: {:?}", e),
		}

		Ok(Ok(DiscoveryResponse::default()))
	}
}
