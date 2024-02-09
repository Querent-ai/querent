---
title: REST API
sidebar_position: 10
---

## API Version

All endpoints in the Querent API start with the `api/v1/` prefix, indicating the usage of version 1 of the API.

## OpenAPI Specification

The OpenAPI specification of the REST API is available at `/openapi.json`, providing comprehensive documentation for all endpoints. Additionally, a Swagger UI version is accessible at `/swagger-ui` for interactive exploration and testing of the API.

## Parameters

Parameters passed in the URL must be properly URL-encoded, utilizing UTF-8 encoding for non-ASCII characters.

```bash

GET [..]/search?query=barack%20obama

```

## Error Handling

- Successful requests return a 2xx HTTP status code.
- Failed requests return a 4xx HTTP status code, with the response body containing a JSON object containing a `message` field describing the error.

```json
{
 "message": "Failed to parse query"
}
```

## Semantic Service API

### Observe Service Pipeline

**Endpoint:** `/api/v1/semantics`

**Method:** `GET`

**Description:** Retrieve observations of the semantic service pipeline.

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "num_running_pipelines": 3,
 "num_successful_pipelines": 10,
 "num_failed_pipelines": 2
}
```

### Start Semantic Pipeline

**Endpoint:** `/api/v1/semantics`

**Method:** `POST`

**Description:** Initiate a semantic pipeline by providing a `SemanticPipelineRequest`.

**Request Body:**

- Content Type: application/json

```json
{
  "name": "knowledge_graph_using_llama2_v1",
  "version": 1.0,
  "collectors": [
    {
      "name": "AzureCollector",
      "backend": "azure",
      "config": {
        "account_url": "your_account_url",
        "container": "your_container",
        "credentials": "your_credentials"
      }
    }
  ]
}
```

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "pipeline_id": "pipeline-12345"
}
```

### Stop Semantic Pipeline

**Endpoint:** `/api/v1/semantics/{pipeline_id}`

**Method:** `DELETE`

**Description:** Terminate a semantic pipeline by providing its unique identifier.

**Path Variable:**

- `{pipeline_id}`: The ID of the pipeline to stop.

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "pipeline_id": "pipeline-12345"
}
```

### Get Pipelines Metadata

**Endpoint:** `/api/v1/semantics/pipelines`

**Method:** `GET`

**Description:** Retrieve metadata for all pipelines.

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "pipelines": [
   {
     "pipeline_id": "pipeline-123",
     "name": "knowledge_graph_using_llama2_v1",
     "import": "semantic_index",
     "attr": "new_pipeline"
   },
   {
     "pipeline_id": "pipeline-456",
     "name": "knowledge_graph_using_openai",
     "import": "semantic_index",
     "attr": "old_pipeline"
   }
 ]
}
```

### Describe Semantic Pipeline

**Endpoint:** `/api/v1/semantics/{pipeline_id}/describe`

**Method:** `GET`

**Description:** Retrieve detailed statistics about a specific semantic pipeline.

**Path Variable:**

- `{pipeline_id}`: The ID of the pipeline to describe.

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "total_docs": 5000,
 "total_events": 10000,
 "total_batches": 50
}
```

### Ingest Tokens into Semantic Pipeline

**Endpoint:** `/api/v1/semantics/{pipeline_id}/ingest`

**Method:** `PUT`

**Description:** Send tokens to a semantic pipeline for processing.

**Path Variable:**

- `{pipeline_id}`: The ID of the pipeline to ingest tokens.

**Request Body:**

- Content Type: application/json

```json
[
  {
    "file": "document.txt",
    "data": ["token1", "token2", "token3"]
  }
]
```

**Response:**

- HTTP Status: 200 OK
- Content Type: text/plain

```json
true
```

### Restart Semantic Pipeline

**Endpoint:** `/api/v1/semantics/{pipeline_id}/restart`

**Method:** `POST`

**Description:** Restart a semantic pipeline by providing its unique identifier.

**Path Variable:**

- `{pipeline_id}`: The ID of the pipeline to restart.

**Response:**

- HTTP Status: 200 OK
- Content Type: application/json

```json
{
 "pipeline_id": "pipeline-12345"
}
```

## Node Health API

### Get Node Liveliness

**Endpoint:** `/health/livez`

**Method:** `GET`

**Description:** Check the liveliness status of the node.

**Response:**

- HTTP Status: 200 OK
- Content Type: text/plain

```json
true
```

### Get Node Readiness

**Endpoint:** `/health/readyz`

**Method:** `GET`

**Description:** Check the readiness status of the node.

**Response:**

- HTTP Status: 200 OK
- Content Type: text/plain

``` json
true
```
