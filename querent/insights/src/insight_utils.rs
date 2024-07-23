use std::collections::HashMap;

pub fn unique_sentences(
    discovered_knowledge: &[(String, String, String, String, String, String, String, f32)],
) -> Vec<String> {
    let mut unique_sentences_map = HashMap::new();

    for (_, _, entity1, entity2, _, sentence, _, _) in discovered_knowledge {
        // Insert into HashMap only if the sentence is not already present
        unique_sentences_map.entry(sentence).or_insert_with(|| format!("Entities: {} and {}, Sentence: {}", entity1, entity2, sentence));
    }

    // Collect only the values from the map, which are the formatted strings
    unique_sentences_map.values().cloned().collect()
}
