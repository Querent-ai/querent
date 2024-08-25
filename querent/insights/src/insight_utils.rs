use common::DocumentPayload;
use std::collections::HashMap;

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

/// Function to extract sentences from documents
pub fn extract_sentences(documents: &Vec<DocumentPayload>) -> Vec<&str> {
	documents.iter().map(|doc| doc.sentence.as_str()).collect()
}


/// Function to split a group of sentences into sets of 10 sentences each
pub fn split_sentences(sentences: &[String]) -> Vec<Vec<String>> {
	sentences.chunks(10).map(|chunk| chunk.to_vec()).collect()
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
