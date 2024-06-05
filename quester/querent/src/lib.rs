pub mod qsource;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::PubSubBroker;
use proto::{semantics::SemanticPipelineRequest, NodeConfig};
pub use qsource::*;
pub mod events;
pub use events::*;
pub mod source;
use querent_synapse::{
	config::{
		config::{CollectorConfig, EngineConfig, WorkflowConfig},
		Config,
	},
	cross::{CLRepr, StringType},
	querent::Workflow,
};
pub use source::*;
pub mod storage;
use ::storage::Storage;
pub use storage::*;
pub mod indexer;
pub mod pipeline;
pub use pipeline::*;
pub mod discovery;
pub use discovery::*;
pub mod agent;
pub use agent::*;
pub mod chain;
pub mod document_loaders;
pub mod embedding;
pub mod ingest;
pub mod language_models;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod schemas;
pub mod semantic_router;
pub mod text_splitter;
pub mod tools;
pub mod vectorstore;
use tracing::info;

#[allow(clippy::too_many_arguments)]
pub async fn start_semantic_service(
	node_config: &NodeConfig,
	quester: &Quester,
	cluster: &Cluster,
	pubsub_broker: &PubSubBroker,
) -> anyhow::Result<MessageBus<SemanticService>> {
	info!("Starting semantic service");

	let semantic_service =
		SemanticService::new(node_config.node_id.clone(), cluster.clone(), pubsub_broker.clone());

	let (semantic_service_mailbox, _) = quester.spawn_builder().spawn(semantic_service);
	info!("Starting semantic service started");
	Ok(semantic_service_mailbox)
}

const _CODE_CONFIG_EVENT_HANDLER: &str = r#"
import asyncio
import json

async def print_querent(config, text: str):
    print(text + ": Engine Bot 🤖") 
    querent_started = False

    try:
        import querent
        print("✨ Querent imported successfully for ai engines✨")
        querent_started = True
        await querent.workflow.start_workflow(config)
        return
    except Exception as e:
        querent_started = False
        print("❌ Failed to import querent:  " + str(e))

"#;

pub async fn create_dynamic_sources(
	request: &SemanticPipelineRequest,
) -> Result<Vec<Arc<dyn sources::Source>>, PipelineErrors> {
	let collectors_configs = request.collectors.clone();
	let mut sources: Vec<Arc<dyn sources::Source>> = vec![];
	for collector in collectors_configs {
		match &collector.backend {
			Some(proto::semantics::Backend::Files(config)) => {
				let file_source = sources::filesystem::files::LocalFolderSource::new(
					PathBuf::from(config.root_path.clone()),
					None,
				);
				sources.push(Arc::new(file_source));
			},
			Some(proto::semantics::Backend::Gcs(config)) => {
				let gcs_source = sources::gcs::get_gcs_storage(config.clone()).map_err(|e| {
					PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Failed to create GCS source: {}",
						e
					))
				})?;
				sources.push(Arc::new(gcs_source));
			},
			Some(proto::semantics::Backend::S3(config)) => {
				let s3_source = sources::s3::S3Source::new(config.clone());
				sources.push(Arc::new(s3_source));
			},
			_ =>
				return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Invalid source type: {}",
					collector.name.clone(),
				))),
		};
	}
	Ok(sources)
}

pub async fn create_querent_synapose_workflow(
	id: String,
	request: &SemanticPipelineRequest,
	secret_store: Arc<dyn Storage>,
) -> Result<Workflow, PipelineErrors> {
	let request_string = serde_json::to_string(request).unwrap();
	secret_store.store_kv(&id, &request_string).await.map_err(|e| {
		PipelineErrors::InvalidParams(anyhow::anyhow!(
			"Failed to store pipeline request in secret store: {}",
			e
		))
	})?;
	let mut sources_credential_map = HashMap::new();
	let _collector_configs: Vec<CollectorConfig> = request
		.collectors
		.iter()
		.map(|c| {
			let mut backend = "".to_string();
			let config = match &c.backend {
				Some(proto::semantics::Backend::Gcs(config)) => {
					backend = "gcs".to_string();
					let source = format!("gcs://{}", config.bucket.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					let mut map = HashMap::new();
					map.insert("bucket".to_string(), config.bucket.clone());
					map.insert("credentials".to_string(), config.credentials.clone());
					map
				},
				Some(proto::semantics::Backend::S3(config)) => {
					backend = "s3".to_string();
					let mut map = HashMap::new();
					let source =
						format!("s3://{}/{}", config.bucket.clone(), config.region.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("bucket".to_string(), config.bucket.clone());
					map.insert("region".to_string(), config.region.clone());
					map.insert("access_key".to_string(), config.access_key.clone());
					map.insert("secret_key".to_string(), config.secret_key.clone());
					map
				},
				Some(proto::semantics::Backend::Azure(config)) => {
					backend = "azure".to_string();
					let mut map = HashMap::new();
					let source = format!(
						"azure://{}/{}",
						config.container.clone(),
						config.account_url.clone()
					);
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("container".to_string(), config.container.clone());
					map.insert("connection_string".to_string(), config.connection_string.clone());
					map.insert("account_url".to_string(), config.account_url.clone());
					map.insert("credentials".to_string(), config.credentials.clone());
					map.insert("prefix".to_string(), config.prefix.clone());
					map
				},
				Some(proto::semantics::Backend::Drive(config)) => {
					backend = "drive".to_string();
					let mut map = HashMap::new();
					let source = format!("drive://{}", config.folder_to_crawl.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert(
						"drive_refresh_token".to_string(),
						config.drive_refresh_token.to_string(),
					);
					map.insert("drive_client_id".to_string(), config.drive_client_id.to_string());
					map.insert(
						"drive_client_secret".to_string(),
						config.drive_client_secret.to_string(),
					);
					map.insert("drive_scopes".to_string(), config.drive_scopes.to_string());
					map.insert("drive_token".to_string(), config.drive_token.to_string());
					map.insert("folder_to_crawl".to_string(), config.folder_to_crawl.to_string());
					map.insert(
						"specific_file_type".to_string(),
						config.specific_file_type.to_string(),
					);
					map
				},
				Some(proto::semantics::Backend::Dropbox(config)) => {
					backend = "dropbox".to_string();
					let mut map = HashMap::new();
					let source = format!("dropbox://{}", config.folder_path.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("dropbox_app_key".to_string(), config.dropbox_app_key.to_string());
					map.insert(
						"dropbox_app_secret".to_string(),
						config.dropbox_app_secret.to_string(),
					);
					map.insert("folder_path".to_string(), config.folder_path.to_string());
					map.insert(
						"dropbox_refresh_token".to_string(),
						config.dropbox_refresh_token.to_string(),
					);
					map
				},
				Some(proto::semantics::Backend::Github(config)) => {
					backend = "github".to_string();
					let mut map = HashMap::new();
					let source = format!("github://{}", config.repository.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("github_username".to_string(), config.github_username.to_string());
					map.insert(
						"github_access_token".to_string(),
						config.github_access_token.to_string(),
					);
					map.insert("repository".to_string(), config.repository.to_string());
					map
				},
				Some(proto::semantics::Backend::Slack(config)) => {
					backend = "slack".to_string();
					let mut map = HashMap::new();
					let source = format!("slack://{}", config.channel_name.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("access_token".to_string(), config.access_token.to_string());
					map.insert("channel_name".to_string(), config.channel_name.to_string());
					map.insert("cursor".to_string(), config.cursor.to_string());
					map.insert(
						"include_all_metadata".to_string(),
						config.include_all_metadata.to_string(),
					);
					if config.limit == 0 {
						map.insert("limit".to_string(), "100".to_string());
					} else {
						map.insert("limit".to_string(), config.limit.to_string());
					}
					map
				},
				Some(proto::semantics::Backend::Jira(config)) => {
					backend = "jira".to_string();
					let mut map = HashMap::new();
					let source = format!(
						"jira://{}/{}",
						config.jira_server.clone(),
						config.jira_project.clone()
					);
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("jira_server".to_string(), config.jira_server.to_string());
					map.insert("jira_username".to_string(), config.jira_username.to_string());
					map.insert("jira_password".to_string(), config.jira_password.to_string());
					map.insert("jira_project".to_string(), config.jira_project.to_string());
					map.insert("jira_query".to_string(), config.jira_query.to_string());
					if config.jira_max_results == 0 {
						map.insert("jira_max_results".to_string(), "100".to_string());
					} else {
						map.insert(
							"jira_max_results".to_string(),
							config.jira_max_results.to_string(),
						);
					}
					map.insert("jira_api_token".to_string(), config.jira_api_token.to_string());
					map.insert("jira_start_at".to_string(), config.jira_start_at.to_string());
					map.insert("jira_keyfile".to_string(), config.jira_keyfile.to_string());
					map.insert("jira_certfile".to_string(), config.jira_certfile.to_string());
					map.insert("jira_verify".to_string(), config.jira_verify.to_string());
					map
				},
				Some(proto::semantics::Backend::Email(config)) => {
					backend = "email".to_string();
					let mut map = HashMap::new();
					let source = format!(
						"email://{}/{}",
						config.imap_server.clone(),
						config.imap_folder.clone()
					);
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("imap_server".to_string(), config.imap_server.to_string());
					map.insert("imap_port".to_string(), config.imap_port.to_string());
					map.insert("imap_username".to_string(), config.imap_username.to_string());
					map.insert("imap_password".to_string(), config.imap_password.to_string());
					map.insert("imap_folder".to_string(), config.imap_folder.to_string());
					map.insert("imap_keyfile".to_string(), config.imap_keyfile.to_string());
					map.insert("imap_certfile".to_string(), config.imap_certfile.to_string());
					map
				},
				Some(proto::semantics::Backend::News(config)) => {
					backend = "news".to_string();
					let mut map = HashMap::new();
					let source = format!("news://{}", config.query.clone());
					sources_credential_map.insert(source.clone(), serde_json::to_string(config));
					map.insert("query".to_string(), config.query.to_string());
					map.insert("api_key".to_string(), config.api_key.to_string());
					if !config.language.is_empty() {
						map.insert("language".to_string(), config.language.to_string());
					}
					if !config.sources.is_empty() {
						map.insert("sources".to_string(), config.sources.to_string());
					}
					if !config.domains.is_empty() {
						map.insert("domains".to_string(), config.domains.to_string());
					}
					if !config.exclude_domains.is_empty() {
						map.insert(
							"exclude_domains".to_string(),
							config.exclude_domains.to_string(),
						);
					}
					map.insert("from_date".to_string(), config.from_date.to_string());
					map.insert("to_date".to_string(), config.to_date.to_string());
					map
				},
				Some(proto::semantics::Backend::Files(config)) => {
					backend = "localfile".to_string();
					let mut map = HashMap::new();
					map.insert("root_path".to_string(), config.root_path.to_string());
					map
				},
				_ => {
					let map = HashMap::new();
					map
				},
			};
			CollectorConfig {
				id: id.clone(),
				name: c.name.clone(),
				backend,
				config,
				inner_channel: None,
				channel: None,
			}
		})
		.collect();
	if request.name.clone().is_none() {
		return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
			"Invalid number of workflows: one workflow is supported in current version."
		)));
	}

	// Store source credentials in secret store
	for (source, credential) in sources_credential_map {
		if credential.is_err() {
			log::error!("Failed to serialize credential: {}", credential.err().unwrap());
			continue;
		}
		secret_store.store_kv(&source, &credential.unwrap()).await.map_err(|e| {
			return PipelineErrors::InvalidParams(anyhow::anyhow!(
				"Failed to store source credentials in secret store: {}",
				e
			));
		})?;
	}

	let current_workflow = request.name.clone().unwrap();
	let mut engine_name = "knowledge_graph_using_llama2_v1".to_string();
	let mut engine_additional_config = HashMap::new();
	match current_workflow.clone() {
		proto::semantics::Name::KnowledgeGraphUsingOpenai(openai_config) => {
			engine_name = "knowledge_graph_using_openai".to_string();
			engine_additional_config
				.insert("openai_api_key".to_string(), openai_config.openai_api_key.clone());
		},
		_ => (),
	}

	let mut params_map = HashMap::new();

	if let Some(workflow_contract) = &request.config {
		// Convert scalar fields to string and insert them into the hashmap
		if let Some(ner_model_name) = &workflow_contract.ner_model_name {
			params_map.insert("ner_model_name".to_owned(), ner_model_name.clone());
		}
		if workflow_contract.enable_filtering.is_some() {
			params_map.insert(
				"enable_filtering".to_owned(),
				workflow_contract.enable_filtering.unwrap().to_string(),
			);
		}
		if workflow_contract.is_confined_search.is_some() {
			params_map.insert(
				"is_confined_search".to_owned(),
				workflow_contract.is_confined_search.unwrap().to_string(),
			);
		}
		if workflow_contract.user_context.is_some() {
			params_map.insert(
				"user_context".to_owned(),
				<std::option::Option<std::string::String> as Clone>::clone(
					&workflow_contract.user_context,
				)
				.unwrap()
				.clone(),
			);
		}

		if workflow_contract.score_threshold.is_some() {
			params_map.insert(
				"score_threshold".to_owned(),
				workflow_contract.score_threshold.unwrap().to_string(),
			);
		}

		if workflow_contract.attention_score_threshold.is_some() {
			params_map.insert(
				"attention_score_threshold".to_owned(),
				workflow_contract.attention_score_threshold.unwrap().to_string(),
			);
		}

		if workflow_contract.similarity_threshold.is_some() {
			params_map.insert(
				"similarity_threshold".to_owned(),
				workflow_contract.similarity_threshold.unwrap().to_string(),
			);
		}

		if workflow_contract.min_cluster_size.is_some() {
			params_map.insert(
				"min_cluster_size".to_owned(),
				workflow_contract.min_cluster_size.unwrap().to_string(),
			);
		}

		if workflow_contract.min_samples.is_some() {
			params_map.insert(
				"min_samples".to_owned(),
				workflow_contract.min_samples.unwrap().to_string(),
			);
		}

		if workflow_contract.cluster_persistence_threshold.is_some() {
			params_map.insert(
				"cluster_persistence_threshold".to_owned(),
				workflow_contract.cluster_persistence_threshold.unwrap().to_string(),
			);
		}

		if let Some(fixed_entities) = &workflow_contract.fixed_entities {
			if !fixed_entities.entities.is_empty() {
				let fixed_entities_str = fixed_entities.entities.join(",");
				params_map.insert("fixed_entities".to_owned(), fixed_entities_str);
			}
		}

		if let Some(sample_entities) = &workflow_contract.sample_entities {
			if !sample_entities.entities.is_empty() {
				let sample_entities_str = sample_entities.entities.join(",");
				params_map.insert("sample_entities".to_owned(), sample_entities_str);
			}
		}

		if let Some(sample_relationships) = &workflow_contract.sample_relationships {
			if !sample_relationships.relationships.is_empty() {
				let sample_relationships_str = sample_relationships.relationships.join(",");
				params_map.insert("sample_relationships".to_owned(), sample_relationships_str);
			}
		}

		if let Some(fixed_relationship) = &workflow_contract.fixed_relationships {
			if !fixed_relationship.relationships.is_empty() {
				let fixed_relationship_str = fixed_relationship.relationships.join(",");
				params_map.insert("fixed_relationships".to_owned(), fixed_relationship_str);
			}
		}
	}

	let engine_configs: Vec<EngineConfig> = vec![EngineConfig {
		id: id.clone(),
		name: engine_name.clone(),
		inner_channel: None,
		channel: None,
		config: engine_additional_config,
	}];
	let config = Config {
		version: request.version.clone(),
		querent_id: id.to_string(),
		querent_name: engine_name.clone(),
		workflow: WorkflowConfig {
			name: engine_name.clone(),
			id: id.to_string(),
			config: params_map.clone(),
			channel: None,
			inner_channel: None,
			inner_event_handler: None,
			event_handler: None,
			inner_tokens_feader: None,
			tokens_feader: None,
		},
		collectors: Vec::new(),
		engines: engine_configs,
		resource: None,
	};
	let workflow = Workflow {
		name: engine_name.clone(),
		id: id.to_string(),
		import: "".to_string(),
		attr: "print_querent".to_string(),
		code: Some(_CODE_CONFIG_EVENT_HANDLER.to_string()),
		arguments: vec![CLRepr::String("Starting Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};
	Ok(workflow)
}
