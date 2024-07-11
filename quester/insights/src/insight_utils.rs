use std::collections::HashSet;

pub fn unique_sentences(
	discovered_knowledge: &[(i32, String, String, String, String, String, String, f32)],
) -> Vec<String> {
	let mut unique_sentences_set = HashSet::new();

	for (_, _, _, _, _, sentence, _, _) in discovered_knowledge {
		unique_sentences_set.insert(sentence.clone());
	}

	unique_sentences_set.into_iter().collect::<Vec<String>>()
}
