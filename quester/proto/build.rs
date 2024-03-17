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
	ProtoGenerator::builder()
		.with_protos(&["protos/querent/semantics.proto"])
		.with_includes(&["protos"])
		.with_output_dir("src/codegen/querent")
		.with_result_type_path("crate::semantics::SemanticsResult")
		.with_error_type_path("crate::semantics::SemanticsError")
		.generate_rpc_name_impls()
		.run()
		.unwrap();
	Ok(())
}
