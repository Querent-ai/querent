pub fn get_suggestions_prompt(suggestion_texts: &[String]) -> String {
	println!("This is the suggestions ---------------{:?}", suggestion_texts);
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
        Based on your understanding of the data above, generate 4 possible natural language questions that a user might ask to gain deeper insights and effectively traverse the semantic data fabric. \
        The questions should be focused on extracting meaningful insights and understanding patterns within the data, rather than simple count-based queries.
        Output Format:\n\
        1. [First question]\n\
        2. [Second question]\n\
        3. [Third question]\n\
        4. [Fourth question]\n\
        #######\n\
        Output:",
        suggestion_texts.get(0).unwrap_or(&"Data not available".to_string()),
        suggestion_texts.get(1).unwrap_or(&"Data not available".to_string()),
    )
}

pub fn get_final_prompt(query: &str, context: &str) -> String {
	format!(
        "You are a knowledgeable assistant responsible for generating a comprehensive summary of the data provided below. \
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

pub fn get_analysis_prompt(query: &str, combined_summaries: &str) -> String {
	format!(
        "You are a knowledgeable assistant tasked with generating a one-page report based on the provided summaries. \
        The report should consist of three key sections: a title, a keywords section, and a concise summary (approximately 100 words). \
        The summaries are listed in descending order of relevance, with Summary 1 being the most relevant to the query. As the relevance decreases with higher numbers, these less relevant summaries may still reveal indirect or previously unknown relationships that are valuable to the analysis. \
        Your goal is to synthesize these summaries into a coherent, third-person summary that effectively addresses the query.\n\n\
        #######\n\
        -Data-\n\
        Query: {query}\n\
        Summaries: {combined_summaries}\n\
        #######\n\
        Output Format:\n\
        Title: [Generate an appropriate title for the report]\n\
        Keywords: [List 3-5 key terms or phrases related to the report]\n\
        Summary: [Compose a concise summary of around 100 words]\n\
        #######\n\
        Output:"
    )
}
