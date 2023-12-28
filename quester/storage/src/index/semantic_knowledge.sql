CREATE TABLE semantic_knowledge (
    id SERIAL PRIMARY KEY,
    subject VARCHAR,
    subject_type VARCHAR,
    object VARCHAR,
    object_type VARCHAR,
    predicate VARCHAR,
    predicate_type VARCHAR,
    sentence TEXT,
    document_id VARCHAR,
);
