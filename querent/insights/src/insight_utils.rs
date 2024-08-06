use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::collections::HashMap;
use tracing::error;

pub fn unique_sentences(
	discovered_knowledge: &[(String, String, String, String, String, String, String, f32)],
) -> (Vec<String>, usize) {
	let mut unique_sentences_map = HashMap::new();

	for (_, _, entity1, entity2, _, sentence, _, _) in discovered_knowledge {
		// Insert into HashMap only if the sentence is not already present
		unique_sentences_map.entry(sentence).or_insert_with(|| {
			format!("Entities: {} and {}, Sentence: {}", entity1, entity2, sentence)
		});
	}

	// Collect only the values from the map, which are the formatted strings
	let unique_sentences = unique_sentences_map.values().cloned().collect();
	let count = unique_sentences_map.len();
	(unique_sentences, count)
}

/// Function to rerank documents based on a query
pub fn rerank_documents(query: &str, documents: Vec<String>) -> Option<Vec<(String, f32)>> {
	let model = match TextRerank::try_new(RerankInitOptions {
		model_name: RerankerModel::BGERerankerBase,
		show_download_progress: true,
		..Default::default()
	}) {
		Ok(model) => model,
		Err(e) => {
			error!("Failed to initialize the reranker model: {:?}", e);
			return None;
		},
	};
	let sentences: Vec<&str> = documents
		.iter()
		.map(|doc| {
			let parts: Vec<&str> = doc.split(", Sentence: ").collect();
			parts[1]
		})
		.collect();

	let results = match model.rerank(query, sentences.clone(), true, None) {
		Ok(results) => results,
		Err(e) => {
			error!("Failed to rerank documents: {:?}", e);
			return None;
		},
	};
	Some(
		results
			.into_iter()
			.map(|result| {
				let doc = &documents[result.index];
				(doc.clone(), result.score)
			})
			.collect(),
	)
}
