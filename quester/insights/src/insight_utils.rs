use std::collections::HashSet;
use storage::DiscoveredKnowledge;

pub fn unique_sentences(discovered_knowledge: &[DiscoveredKnowledge]) -> String {
    let mut unique_sentences_set = HashSet::new();

    for dk in discovered_knowledge {
        unique_sentences_set.insert(dk.sentence.clone());
    }

    unique_sentences_set.into_iter().collect::<Vec<String>>().join(" ")
}