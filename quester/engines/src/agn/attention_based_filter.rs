use std::collections::HashSet;
use crate::agn::attention_based_search::Entity;


#[derive(Debug, Clone)]
pub struct HeadTailRelations {
    pub head: Entity,
    pub tail: Entity,
    pub relations: Vec<(String, f32)>,
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
        pub fn new(doc: Vec<Token>, forward_relations: bool, threshold: f32) -> Self {
            Self {
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
    
            let mut seen_relations = HashSet::new();
    
            for candidate in candidates {
                if candidate.mean_score() < self.threshold {
                    // println!("Skipping candidate with mean score {}", candidate.mean_score());
                    continue;
                }
    
                let mut rel_txt = String::new();
                let mut last_index = None;
                let mut valid = true;
    
                // println!("Processing candidate with rel_tokens: {:?}", candidate.rel_tokens);
    
                for &token_id in &candidate.rel_tokens {
                    let word_id = token_id;
                    let word = &self.doc[token_id].text;
                    // println!("Word ID is -----{:?}", word_id);
                    // println!("Word is --------{:?}", word);
                    // println!("last index --------{:?}", last_index);
                    if self.forward_relations && last_index.is_some() {
                        let last_idx = last_index.unwrap();
                        if word_id <= last_idx || word_id - last_idx != 1 {
                            valid = false;
                            // println!("Invalid token sequence: word_id {} is not contiguous with last_index {}", word_id, last_idx);
                            break;
                        }
                    }
                    last_index = Some(word_id);
    
                    if !rel_txt.is_empty() {
                        rel_txt.push(' ');
                    }
                    let lowered_word = word.to_lowercase();
    
                    // Only skip if the entire word is equal to head or tail entity text
                    if !head.name.eq_ignore_ascii_case(&lowered_word) && !tail.name.eq_ignore_ascii_case(&lowered_word) {
                        rel_txt.push_str(&lowered_word);
                        // println!("Adding word: {}", lowered_word);
                    } else {
                        // println!("Skipping word: {}", lowered_word);
                    }
    
                    // println!("Accumulated relation text: {}", rel_txt);
                }
    
                if valid {
                    let lemmatized_txt = self.simple_filter(&rel_txt);
                    // println!("Lemmatized relation text: {}", lemmatized_txt);
                    if !lemmatized_txt.is_empty() && seen_relations.insert(lemmatized_txt.clone()) {
                        // println!("Added relation: {}", lemmatized_txt);
                        response.relations.push((lemmatized_txt, candidate.mean_score()));
                    }
                }
            }
    
            response
        }
    
        fn simple_filter(&self, relation: &str) -> String {
            let filtered_relation = relation
                .split_whitespace()
                .filter(|word| word.chars().all(char::is_alphanumeric))
                .collect::<Vec<&str>>()
                .join(" ");
            // println!("Filtered relation: {}", filtered_relation);
            filtered_relation
        }
    }


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_individual_filter() {
//         let doc = vec![
//             Token { text: "Joel".to_string(), lemma: "joel".to_string() },
//             Token { text: "lives".to_string(), lemma: "live".to_string() },
//             Token { text: "in".to_string(), lemma: "in".to_string() },
//             Token { text: "India".to_string(), lemma: "india".to_string() },
//             Token { text: ".".to_string(), lemma: ".".to_string() },
//         ];

//         let filter = IndividualFilter::new(doc, true, 0.05);

//         let candidates = vec![
//             SearchBeam { rel_tokens: vec![1], score: 0.1 },
//             SearchBeam { rel_tokens: vec![1, 2], score: 0.2 },
//             SearchBeam { rel_tokens: vec![1, 2, 3], score: 0.3 },
//         ];

//         let head = Entity { name: "joel".to_string(), start_idx: 0, end_idx: 0};
//         let tail = Entity { name: "india".to_string(), start_idx:0, end_idx:0};

//         let result = filter.filter(candidates, &head, &tail);

//         // assert_eq!(result.relations.len(), 2);
//         // assert!(result.relations.contains(&"lives in".to_string()));
//         // assert!(result.relations.contains(&"lives in india".to_string()));
//     }
//     }