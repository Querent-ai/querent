CREATE TABLE semantic_knowledge (
    id SERIAL PRIMARY KEY,
    subject VARCHAR,
    subject_type VARCHAR,
    object VARCHAR,
    object_type VARCHAR,
    sentence TEXT,
    document_id VARCHAR,
    document_source VARCHAR,
    collection_id VARCHAR,
    image_id VARCHAR,
    event_id VARCHAR,
    source_id VARCHAR
);
