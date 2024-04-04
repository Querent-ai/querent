use crate::{
	agent::{AgentExecutor, ConversationalAgent, ConversationalAgentBuilder},
	chain::options::ChainCallOptions,
	discovery::chain::chain_trait::Chain,
	llm::{OpenAI, OpenAIModel},
	memory::WindowBufferMemory,
	prompt_args,
	tools::CommandExecutor,
};
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
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
		let current_agent = self.discover_agent.as_ref().unwrap();
		let input_variables = prompt_args! {
			"input" => message.query.clone(),
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
