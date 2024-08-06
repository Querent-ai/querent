pub fn get_suggestions_prompt(suggestion_texts: &[String]) -> String {
	format!(
        "The following data is based on a user's private domain data stored as a semantic data fabric in a SQL-based storage. \
        The semantic data fabric organizes data in the form of semantic triples (Subject, Predicate, Object), making it easier to connect information in a graph data structure for conducting traversal and finding unique patterns and linkages. \
        Please analyze and understand the data below to get a holistic view of the semantic data fabric:\n\n\
        1. **Most Frequently Occurring Entity Pairs**:\n\
        These identify the most common relationships in the data, highlighting the pairs of entities that appear together most often.\n\
        Data: {}\n\n\
        2. **Most Unique Entity Pairs**:\n\
        These focus on the rare relationships in the data, showing entity pairs that appear infrequently, indicating unique interactions.\n\
        Data: {}\n\n\
        3. **High-Impact Sentences**:\n\
        These identify sentences that appear most frequently in the data, which can be crucial for understanding key themes and recurring ideas.\n\
        Data: {}\n\n\
        Based on your understanding of the data above, generate 10 possible natural language questions that a user might ask to gain deeper insights and effectively traverse the semantic data fabric. \
        The questions should be focused on extracting meaningful insights and understanding patterns within the data, rather than simple count-based queries.",
        suggestion_texts.get(0).unwrap_or(&"Data not available".to_string()),
        suggestion_texts.get(1).unwrap_or(&"Data not available".to_string()),
        suggestion_texts.get(4).unwrap_or(&"Data not available".to_string())
    )
}

pub fn get_final_prompt(query: &str, context: &str) -> String {
	format!(
        "You are a helpful assistant responsible for generating a comprehensive summary of the data provided below. \
        Given below is a user query and its graph traversal results, which have sentences from various documents along with the entities identified in the sentence. \
        Please concatenate all of these into a single, comprehensive description that answers the user's query making sure to use information collected from all the sentences. \
        If the provided traversal results are contradictory, please resolve the contradictions and provide a single, coherent summary. \
        Make sure it is written in third person, and make sure we have the full context.\n\n\
        #######\n\
        -Data-\n\
        User Query: {query}\n\
        Graph Traversal Results: {context}\n\
        #######\n\
        Output:"
    )
}

pub fn get_analysis_prompt(query: &str, answer: &str) -> String {
	format!(
        "You are a helpful assistant responsible for analyzing the Question and Answer pair below for factual accuracy. \
        If the provided Question and Answer pair are contradictory, please resolve the contradictions and provide a single, coherent summary that accurately answers the user query without introducing any new information. \
        Make sure it is written in third person, and make sure we have the full context.\n\n\
        #######\n\
        -Data-\n\
        Question: {query}\n\
        Answer: {answer}\n\
        #######\n\
        Output:"
    )
}
