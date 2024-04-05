CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embedded_knowledge(
    id SERIAL PRIMARY KEY,
    document_id VARCHAR,
    knowledge TEXT,
    predicate TEXT,
    embeddings vector(384)
);
