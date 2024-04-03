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
	prost_config.extern_path(".querent.discovery.StorageType", "crate::semantics::StorageType");

	tonic_build::configure()
		.enum_attribute(".", "#[serde(rename_all=\"snake_case\")]")
		.type_attribute(".", "#[derive(Serialize, Deserialize, utoipa::ToSchema)]")
		.type_attribute("DiscoveryRequest", "#[derive(Eq, Hash)]")
		.type_attribute("SortFld", "#[derive(Eq, Hash)]")
		.out_dir("src/codegen/querent")
		.compile_with_config(prost_config, &["protos/querent/discovery.proto"], &["protos"])?;

	Ok(())
}
