use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::collections::HashMap;
use tracing::error;

/// Function to get unique contexts
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

/// Functions to split a group of sentences into sets of 10 sentences each
pub fn split_sentences(sentences: Vec<String>) -> Vec<Vec<String>> {
	let mut result = Vec::new();
	let mut chunk = Vec::new();

	for sentence in sentences {
		chunk.push(sentence);
		if chunk.len() == 10 {
			result.push(chunk);
			chunk = Vec::new();
		}
	}
	if !chunk.is_empty() {
		result.push(chunk);
	}

	result
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_unique_sentences() {
		let discovered_knowledge = vec![
			(
				"doc1".to_string(),
				"type1".to_string(),
				"Entity1".to_string(),
				"Entity2".to_string(),
				"relation1".to_string(),
				"This is a sentence.".to_string(),
				"additional".to_string(),
				0.9,
			),
			(
				"doc2".to_string(),
				"type2".to_string(),
				"Entity1".to_string(),
				"Entity3".to_string(),
				"relation2".to_string(),
				"This is another sentence.".to_string(),
				"additional".to_string(),
				0.8,
			),
			(
				"doc3".to_string(),
				"type3".to_string(),
				"Entity1".to_string(),
				"Entity2".to_string(),
				"relation3".to_string(),
				"This is a sentence.".to_string(),
				"additional".to_string(),
				0.85,
			),
		];

		let (unique_sentences, count) = unique_sentences(&discovered_knowledge);

		assert_eq!(unique_sentences.len(), 2);
		assert_eq!(count, 2);
		assert!(unique_sentences
			.contains(&"Entities: Entity1 and Entity2, Sentence: This is a sentence.".to_string()));
		assert!(unique_sentences.contains(
			&"Entities: Entity1 and Entity3, Sentence: This is another sentence.".to_string()
		));
	}

	#[test]
	fn test_split_sentences() {
		let sentences = vec![
			"Sentence 1".to_string(),
			"Sentence 2".to_string(),
			"Sentence 3".to_string(),
			"Sentence 4".to_string(),
			"Sentence 5".to_string(),
			"Sentence 6".to_string(),
			"Sentence 7".to_string(),
			"Sentence 8".to_string(),
			"Sentence 9".to_string(),
			"Sentence 10".to_string(),
			"Sentence 11".to_string(),
		];

		let split = split_sentences(sentences.clone());

		assert_eq!(split.len(), 2);
		assert_eq!(split[0].len(), 10);
		assert_eq!(split[1].len(), 1);
		assert_eq!(split[0][0], "Sentence 1".to_string());
		assert_eq!(split[0][9], "Sentence 10".to_string());
		assert_eq!(split[1][0], "Sentence 11".to_string());
	}
}
