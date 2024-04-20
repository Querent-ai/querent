CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embedded_knowledge(
    id SERIAL PRIMARY KEY,
    document_id VARCHAR,
    document_source VARCHAR,
    knowledge TEXT,
    sentence TEXT,
    predicate TEXT,
    embeddings vector(384),
    collection_id VARCHAR
);
