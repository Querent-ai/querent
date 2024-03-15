/// NamedWorkflows is a message to hold named workflows for semantic pipelines.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NamedWorkflows {
    /// Knowledge graph using Llama2 v1 workflow.
    #[prost(string, tag = "1")]
    pub knowledge_graph_using_llama2_v1: ::prost::alloc::string::String,
    /// Knowledge graph using OpenAI workflow.
    #[prost(message, optional, tag = "2")]
    pub knowledge_graph_using_openai: ::core::option::Option<OpenAiConfig>,
}
/// OpenAIConfig is a message to hold configuration for an OpenAI workflow.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpenAiConfig {
    /// API key of the OpenAI workflow.
    ///
    /// Corrected field name
    #[prost(string, tag = "1")]
    pub openai_api_key: ::prost::alloc::string::String,
}
