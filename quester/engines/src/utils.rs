use llms::LLM;
use regex::Regex;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Serialize)]
pub(crate) struct ClassifiedSentence {
    pub sentence: String,
    pub entities: Vec<(String, String, usize)>,
}

#[derive(Debug, Serialize)]
pub struct ClassifiedSentenceWithPairs {
    pub sentence: String,
    pub entities: Vec<(String, String, usize)>,
    pub pairs: Vec<(String, String)>,
}

/// Removes newline characters from the given text.
pub fn remove_newlines(text: &str) -> String {
    let re = Regex::new(r"\n+").unwrap();
    re.replace_all(text, " ").to_string()
}

/// Splits the provided text into a vector of sentences.
pub fn split_into_sentences(text: &str) -> Vec<String> {
    UnicodeSegmentation::split_sentence_bounds(text)
        .map(|sentence| sentence.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Splits the provided tokens into chunks based on the maximum token length.
pub fn split_into_chunks(max_tokens: usize, tokens: &str) -> Vec<String> {
    let sentences = split_into_sentences(tokens);
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_length = 0;

    for sentence in sentences {
        let sentence_length = sentence.chars().count();

        if current_length + sentence_length > max_tokens {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk.clear();
                current_length = 0;
            }

            if sentence_length > max_tokens {
                let mut start = 0;
                while start < sentence_length {
                    let end = std::cmp::min(start + max_tokens, sentence_length);
                    chunks.push(sentence[start..end].to_string());
                    start = end;
                }
            } else {
                current_chunk = sentence.clone();
                current_length = sentence_length;
            }
        } else {
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(&sentence);
            current_length += sentence_length;
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

pub fn label_entities_in_sentences(entities: &[String], sentences: &[String]) -> Vec<ClassifiedSentence> {
    let mut classified_sentences = Vec::new();
    for sentence in sentences {
        let mut found_entities = Vec::new();
        for entity in entities {
            if sentence.contains(entity) {
                found_entities.push((entity.clone(), "Unlabelled".to_string(), 0)); // Position will be updated later
            }
        }
        classified_sentences.push(ClassifiedSentence {
            sentence: sentence.clone(),
            entities: found_entities,
        });
    }
    classified_sentences
}

pub async fn tokens_to_words(llm: &dyn LLM, tokens: &[Vec<i32>]) -> Vec<Vec<String>> {
    let mut words_list = Vec::new();
    for token_seq in tokens {
        let words = llm.tokens_to_words(token_seq).await;
        println!("Words are --------{:?}", words);
        words_list.push(words);
    }
    words_list
}

pub fn match_entities_with_tokens(
    tokenized_sentences: &[Vec<String>],
    classified_sentences: &[ClassifiedSentence],
) -> Vec<ClassifiedSentence> {
    let mut results = Vec::new();

    for (sentence_index, classified_sentence) in classified_sentences.iter().enumerate() {
        let sentence_tokens = &tokenized_sentences[sentence_index];
        let mut matched_entities = Vec::new();

        for (entity, label, _) in &classified_sentence.entities {
            if let Some(position) = sentence_tokens.iter().position(|token| token == entity) {
                matched_entities.push((entity.clone(), label.clone(), position));
            }
        }

        results.push(ClassifiedSentence {
            sentence: classified_sentence.sentence.clone(),
            entities: matched_entities,
        });
    }

    results
}

pub fn create_binary_pairs(classified_sentences: &[ClassifiedSentence]) -> Vec<ClassifiedSentenceWithPairs> {
    let mut results = Vec::new();

    for classified_sentence in classified_sentences {
        let sentence = &classified_sentence.sentence;
        let entities = &classified_sentence.entities;

        if entities.len() > 1 {
            let mut pairs = Vec::new();

            for i in 0..entities.len() {
                for j in (i + 1)..entities.len() {
                    let (entity1, _, index1) = &entities[i];
                    let (entity2, _, index2) = &entities[j];

                    if (index1.clone() as isize - index2.clone() as isize).abs() < 10 {
                        pairs.push((entity1.clone(), entity2.clone()));
                    }
                }
            }

            results.push(ClassifiedSentenceWithPairs {
                sentence: sentence.clone(),
                entities: entities.clone(),
                pairs,
            });
        } else {
            results.push(ClassifiedSentenceWithPairs {
                sentence: sentence.clone(),
                entities: entities.clone(),
                pairs: Vec::new(),
            });
        }
    }

    results
}


