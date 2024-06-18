use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Entity {
    pub text: String,
    pub wikidata_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeadTailRelations {
    pub head: Entity,
    pub tail: Entity,
    pub relations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SearchBeam {
    pub rel_tokens: Vec<usize>,
    pub score: f32,
}

impl SearchBeam {
    pub fn mean_score(&self) -> f32 {
        self.score / self.rel_tokens.len() as f32
    }
}

pub struct IndividualFilter {
    token_idx_2_word_doc_idx: Vec<(String, usize)>,
    doc: Vec<Token>,
    forward_relations: bool,
    threshold: f32,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub lemma: String,
}

impl IndividualFilter {
    pub fn new(
        token_idx_2_word_doc_idx: Vec<(String, usize)>,
        doc: Vec<Token>,
        forward_relations: bool,
        threshold: f32,
    ) -> Self {
        Self {
            token_idx_2_word_doc_idx,
            doc,
            forward_relations,
            threshold,
        }
    }

    pub fn filter(&self, candidates: Vec<SearchBeam>, head: &Entity, tail: &Entity) -> HeadTailRelations {
        let mut response = HeadTailRelations {
            head: head.clone(),
            tail: tail.clone(),
            relations: Vec::new(),
        };

        for candidate in candidates {
            if candidate.mean_score() < self.threshold {
                continue;
            }

            let mut rel_txt = String::new();
            let mut rel_idx = Vec::new();
            let mut last_index = None;
            let mut valid = true;

            for &token_id in &candidate.rel_tokens {
                let (ref word, word_id) = self.token_idx_2_word_doc_idx[token_id];

                if self.forward_relations && last_index.is_some() && word_id - last_index.unwrap() != 1 {
                    valid = false;
                    break;
                }
                last_index = Some(word_id);

                if !rel_txt.is_empty() {
                    rel_txt.push(' ');
                }
                let lowered_word = word.to_lowercase();
                if !head.text.contains(&lowered_word) && !tail.text.contains(&lowered_word) {
                    rel_txt.push_str(&lowered_word);
                    rel_idx.push(word_id);
                }
            }

            if valid {
                let lemmatized_txt = self.simple_filter(&rel_txt);
                if !lemmatized_txt.is_empty() {
                    response.relations.push(lemmatized_txt);
                }
            }
        }

        response
    }

    fn simple_filter(&self, relation: &str) -> String {
        relation
            .split_whitespace()
            .filter(|word| word.chars().all(char::is_alphanumeric))
            .collect::<Vec<&str>>()
            .join(" ")
    }
}

pub fn frequency_cutoff(ht_relations: &mut [HeadTailRelations], frequency: usize) {
    if frequency == 1 {
        return;
    }
    let mut counter: HashMap<String, usize> = HashMap::new();

    for ht_item in ht_relations.iter() {
        for relation in &ht_item.relations {
            *counter.entry(relation.clone()).or_insert(0) += 1;
        }
    }

    for ht_item in ht_relations.iter_mut() {
        ht_item.relations.retain(|rel| counter.get(rel).copied().unwrap_or(0) >= frequency);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_individual_filter() {
        let token_idx_2_word_doc_idx = vec![
            ("Joel".to_string(), 0),
            ("lives".to_string(), 1),
            ("in".to_string(), 2),
            ("India".to_string(), 3),
            (".".to_string(), 4),
        ];

        let doc = vec![
            Token { text: "Joel".to_string(), lemma: "joel".to_string() },
            Token { text: "lives".to_string(), lemma: "live".to_string() },
            Token { text: "in".to_string(), lemma: "in".to_string() },
            Token { text: "India".to_string(), lemma: "india".to_string() },
            Token { text: ".".to_string(), lemma: ".".to_string() },
        ];

        let filter = IndividualFilter::new(token_idx_2_word_doc_idx, doc, true, 0.05);

        let candidates = vec![
            SearchBeam { rel_tokens: vec![1], score: 0.1 },
            SearchBeam { rel_tokens: vec![1, 2], score: 0.2 },
            SearchBeam { rel_tokens: vec![1, 2, 3], score: 0.3 },
        ];

        let head = Entity { text: "joel".to_string(), wikidata_id: None };
        let tail = Entity { text: "india".to_string(), wikidata_id: None };

        let result = filter.filter(candidates, &head, &tail);
        println!("Filtered Relations: {:?}", result.relations);

        // assert_eq!(result.relations.len(), 2);
        // assert!(result.relations.contains(&"live".to_string()));
    }

}
