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

use std::collections::HashMap;
use storage::DiscoveredKnowledge;

/// Function to get unique contexts
pub fn unique_sentences(
	discovered_knowledge: &[(String, String, String, String, String, String, String, f32)],
) -> (Vec<String>, usize) {
	let mut unique_sentences_map = HashMap::new();

	for (_, _, entity1, entity2, _, sentence, _, _) in discovered_knowledge {
		unique_sentences_map.entry(sentence).or_insert_with(|| {
			format!("Entities: {} and {}, Sentence: {}", entity1, entity2, sentence)
		});
	}
	let unique_sentences = unique_sentences_map.values().cloned().collect();
	let count = unique_sentences_map.len();
	(unique_sentences, count)
}

/// Function to extract sentences from documents
pub fn extract_sentences(documents: &Vec<DiscoveredKnowledge>) -> Vec<&str> {
	documents.iter().map(|doc| doc.sentence.as_str()).collect()
}

/// Function to split a group of sentences into sets of 10 sentences each
pub fn split_sentences(sentences: &[String]) -> Vec<Vec<String>> {
	sentences.chunks(10).map(|chunk| chunk.to_vec()).collect()
}

/// Function for computing cosine similarity.
pub fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
	let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
	let magnitude1: f32 = vec1.iter().map(|v| v * v).sum::<f32>().sqrt();
	let magnitude2: f32 = vec2.iter().map(|v| v * v).sum::<f32>().sqrt();
	dot_product / (magnitude1 * magnitude2)
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

		let split = split_sentences(&sentences);

		assert_eq!(split.len(), 2);
		assert_eq!(split[0].len(), 10);
		assert_eq!(split[1].len(), 1);
		assert_eq!(split[0][0], "Sentence 1".to_string());
		assert_eq!(split[0][9], "Sentence 10".to_string());
		assert_eq!(split[1][0], "Sentence 11".to_string());
	}
}
