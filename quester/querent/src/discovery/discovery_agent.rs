use crate::{
	agent::{AgentExecutor, ConversationalAgent, ConversationalAgentBuilder},
	chain::{chain_trait::Chain, options::ChainCallOptions},
	llm::{OpenAI, OpenAIModel},
	memory::WindowBufferMemory,
	prompt::{PromptFromatter, PromptTemplate, TemplateFormat},
	prompt_args,
	tools::CommandExecutor,
};
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_openai::config::OpenAIConfig;
use async_trait::async_trait;
use common::RuntimeType;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryAgentType, DiscoveryError,
};
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tokio::runtime::Handle;

use super::insert_discovered_knowledge_async;

pub struct DiscoveryAgent {
	agent_id: String,
	timestamp: u64,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	discovery_agent_params: DiscoverySessionRequest,
	discover_agent: Option<AgentExecutor<ConversationalAgent>>,
	template: Option<PromptTemplate>,
	embedding_model: Option<TextEmbedding>,
}

impl DiscoveryAgent {
	pub fn new(
		agent_id: String,
		timestamp: u64,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		discovery_agent_params: DiscoverySessionRequest,
	) -> Self {
		Self {
			agent_id,
			timestamp,
			event_storages,
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
		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		})?;

		self.embedding_model = Some(embedding_model);

		let template = PromptTemplate::new(
            "Answer the user query: {{query}}

By summarising the following data where the format of each data item is  {\"doc_id\":\"the document from which sentence is extracted.\",\"sentence\":\"the context relevant to user's query.\"}.

Data: {{graph_data}}

In your analysis, focus on the following areas to construct a comprehensive and insightful summary:
1. Identification and examination of sentences directly relevant to the query. Examine the data provided, and emphasize any specific information such as numerical data, statistics, or quantitative values mentioned within the sentences.
2. Provide additional insights from a deep dive into the document analysis. Highlight any notable correlations, inconsistencies between sources, or emerging trends that have been documented. Point out documents that were particularly influential or informative in your analysis.
Your summary should distill the essential findings and insights from the dataset, offering a clear, detailed, and contextually relevant response to the user's query.".to_string(),
            vec!["query".to_string(), "graph_data".to_string()],
            TemplateFormat::Jinja2,
        );
		let open_ai_config =
			OpenAIConfig::new().with_api_key(self.discovery_agent_params.openai_api_key.clone());
		let llm = OpenAI::new(open_ai_config).with_model(OpenAIModel::Gpt35);
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
		let mut documents = Vec::new();
		for (event_type, storage) in self.event_storages.iter() {
			if event_type.clone() == EventType::Vector {
				for storage in storage.iter() {
					let search_results = storage
						.similarity_search_l2(
							message.session_id.clone(),
							self.discovery_agent_params.semantic_pipeline_id.clone(),
							&current_query_embedding.clone(),
							10,
						)
						.await;
					match search_results {
						Ok(results) => {
							let res = results.clone();
							for document in results {
								let tags = format!(
									"{}, {}, {}",
									document.subject.replace('_', " "),
									document.object.replace('_', " "),
									document.predicate.replace('_', " ")
								);
								let formatted_document = proto::discovery::Insight {
									document: document.doc_id,
									source: document.doc_source,
									knowledge: document.knowledge,
									sentence: document.sentence,
									tags,
								};

								documents.push(formatted_document);
							}
							tokio::spawn(insert_discovered_knowledge_async(storage.clone(), res));
						},
						Err(e) => {
							log::error!("Failed to search for similar documents: {}", e);
						},
					}
				}
			}
		}
		if let Some(DiscoveryAgentType::Retriever) = self.discovery_agent_params.session_type {
			return Ok(Ok(DiscoveryResponse {
				session_id: message.session_id,
				query: message.query.clone(),
				insights: documents.clone(),
			}));
		}
		let input_variables = prompt_args! {
			"query" => message.query.clone(),
			"graph_data" => documents.clone()
		};
		let result = template.format(input_variables).map_err(|e| {
			log::error!("Failed to format template: {}", e);
			ActorExitStatus::Failure(Arc::new(anyhow::anyhow!("Failed to format template")))
		})?;
		let input_query = prompt_args! {
			"input" => result,
		};
		match current_agent.invoke(input_query).await {
			Ok(_result) => {
				let mut response = DiscoveryResponse::default();
				response.session_id = message.session_id;
				response.insights = vec![];
				let insight = proto::discovery::Insight {
					document: message.query.clone(),
					source: "".to_string(),
					knowledge: "".to_string(),
					sentence: "".to_string(),
					tags: "".to_string(),
				};
				response.insights.push(insight);
				Ok(Ok(response))
			},
			Err(e) => panic!("Error invoking LLMChain: {:?}", e),
		}
	}
}
