use std::collections::{HashSet, HashMap};
use common::{DocumentPayload};


// Function to get top k entries based on cosine distance and return unique pairs
pub fn get_top_k_pairs(payloads: Vec<DocumentPayload>, k: usize) -> Vec<(String, String)> {
    // Use a HashSet to store unique entries and filter them in one pass
    let mut unique_entries = HashSet::new();

    // Filter unique entries and sort by ascending order of cosine distance
    let mut unique_payloads: Vec<_> = payloads.into_iter()
        .filter(|p| unique_entries.insert((p.subject.clone(), p.object.clone())))
        .collect();

    unique_payloads.sort_by(|a, b| a.cosine_distance.partial_cmp(&b.cosine_distance).unwrap_or(std::cmp::Ordering::Equal));

    // Get top k unique entries
    unique_payloads.into_iter()
        .take(k)
        .map(|p| (p.subject, p.object))
        .collect()
}

pub fn process_traverser_results(
    data: Vec<(i32, String, String, String, String, String, String, f32)>,
) -> Vec<String> {
    // Create a map to store the subject-object pairs and their corresponding results
    let mut pairs_map: HashMap<(String, String), Vec<(i32, String, String, String, String, String, String, f32)>> = HashMap::new();
    
    // Populate the map with results
    for (id, doc_id, subject, object, doc_source, sentence, event_id, score) in data {
        pairs_map.entry((subject.clone(), object.clone()))
            .or_insert_with(Vec::new)
            .push((id, doc_id, subject, object, doc_source, sentence, event_id, score));
    }
    
    // Create a vector to store the formatted output
    let mut formatted_output: Vec<String> = Vec::new();
    
    // Process each pair
    for ((subject, object), mut results) in pairs_map {
        // Sort the results by score in descending order
        results.sort_by(|a, b| b.7.partial_cmp(&a.7).unwrap());
        
        // Ensure unique subject, object, and sentence combinations and collect top 3
        let mut unique_entries = HashSet::new();
        let mut top_3 = Vec::new();
        
        for result in results {
            if unique_entries.insert((result.2.clone(), result.3.clone(), result.5.clone())) { // Check if the combination is unique
                top_3.push(result);
                if top_3.len() == 3 {
                    break;
                }
            }
        }
        
        // Format the output for each of the top 3 results
        for (i, (id, _doc_id, _subject, _object, _doc_source, sentence, _event_id, _score)) in top_3.iter().enumerate() {
            formatted_output.push(format!(
                "{}. {} -> {}: {{ sentence: \"{}\", id: \"{}\" }}",
                i + 1, subject, object, sentence, id
            ));
        }
    }
    
    formatted_output
}
