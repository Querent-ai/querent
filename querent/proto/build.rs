// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use codegen::ProtoGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Cluster proto code generation
	ProtoGenerator::builder()
		.with_protos(&["protos/querent/cluster.proto"])
		.with_output_dir("src/codegen/querent")
		.with_result_type_path("crate::cluster::ClusterResult")
		.with_error_type_path("crate::cluster::ClusterError")
		.generate_rpc_name_impls()
		.run()
		.unwrap();

	// Semantic service proto code generation
	let mut prost_config: prost_build::Config = prost_build::Config::default();
	prost_config
		.type_attribute(".", "#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]");
	prost_config.extern_path(".querent.semantics.StorageType", "StorageType");
	ProtoGenerator::builder()
		.with_prost_config(prost_config)
		.with_protos(&["protos/querent/semantics.proto"])
		.with_includes(&["protos"])
		.with_output_dir("src/codegen/querent")
		.with_result_type_path("crate::semantics::SemanticsResult")
		.with_error_type_path("crate::semantics::SemanticsError")
		.generate_rpc_name_impls()
		.run()
		.unwrap();

	// Discovery service proto code generation
	let mut prost_config = prost_build::Config::default();
	prost_config.protoc_arg("--experimental_allow_proto3_optional");
	prost_config.extern_path(".querent.discovery.StorageType", "StorageType");
	prost_config.extern_path(".querent.discovery.StorageConfig", "StorageConfig");
	prost_config.extern_path(".querent.discovery.PostgresConfig", "PostgresConfig");
	prost_config.extern_path(".querent.discovery.Neo4jConfig", "Neo4jConfig");
	prost_config.extern_path(".querent.discovery.DiscoveryAgentType", "DiscoveryAgentType");

	tonic_build::configure()
		.enum_attribute(".", "#[serde(rename_all=\"snake_case\")]")
		.type_attribute(".", "#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]")
		.type_attribute("DiscoveryRequest", "#[derive(Eq, Hash)]")
		.type_attribute("SortFld", "#[derive(Eq, Hash)]")
		.out_dir("src/codegen/querent")
		.compile_with_config(prost_config, &["protos/querent/discovery.proto"], &["protos"])?;

	// Insight service proto code generation
	let mut prost_config = prost_build::Config::default();
	prost_config.protoc_arg("--experimental_allow_proto3_optional");
	tonic_build::configure()
		.enum_attribute(".", "#[serde(rename_all=\"snake_case\")]")
		.type_attribute(".", "#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]")
		.type_attribute("SortFld", "#[derive(Eq, Hash)]")
		.out_dir("src/codegen/querent")
		.compile_with_config(prost_config, &["protos/querent/insights.proto"], &["protos"])?;

	Ok(())
}
