CREATE TABLE discovered_knowledge(
    id SERIAL PRIMARY KEY,
    doc_id VARCHAR,
    doc_source VARCHAR,
    sentence TEXT,
    subject TEXT,
    object TEXT,
    cosine_distance FLOAT8,
    query_embedding vector(384),
    query VARCHAR,
    session_id VARCHAR,
    score FLOAT8, 
    collection_id VARCHAR
);

CREATE TABLE embedded_knowledge(
    id SERIAL PRIMARY KEY,
    embeddings vector(384),
    score FLOAT4,
    event_id VARCHAR
);

CREATE TABLE insight_knowledge (
    id SERIAL PRIMARY KEY,
    query VARCHAR,
    session_id VARCHAR,
    response VARCHAR

);

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

