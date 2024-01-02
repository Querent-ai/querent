# Quester

Quester is an enterprise-grade Rust engine built on top of Querent, designed to provide powerful and flexible querying capabilities for various data sources. It leverages the Querent library to facilitate efficient and expressive searches across structured and unstructured data.

## Features

- **Semantic Querying:** Quester enables semantic querying by utilizing a SemanticPayload data structure. This allows users to express complex queries based on semantic relationships between entities.

- **Support for Multiple Index Databases:** Quester is designed to support various index databases, including Milvus and Neo4j. This flexibility allows users to choose the indexing solution that best fits their requirements.

- **Modular Architecture:** The architecture of Quester is modular, making it easy to extend and integrate with different components. This modularity promotes scalability and maintainability.

- **Indexing Actor:** Quester includes an indexing actor responsible for efficiently processing and indexing SemanticPayload data. The actor model ensures concurrent and asynchronous handling of indexing tasks.

- **Detailed Indexing Metrics:** The indexing actor tracks various metrics during the indexing process, such as the number of documents indexed, memory usage, indexing throughput, and more. These metrics provide insights into the performance and health of the indexing process.

- **Querying Capabilities:** Quester supports expressive querying capabilities, allowing users to perform complex searches based on semantic relationships, predicates, and other criteria.

- **Resource Utilization Monitoring:** The engine monitors resource utilization during both indexing and querying, helping users optimize performance and allocate resources effectively.

## Sequence Diagram

```mermaid
    sequenceDiagram
        participant Qflow as Q
        participant SourceActor as SA
        participant StorageMapper as SM
        participant Indexer as I
        participant EventStreamer as ES

        Q ->> SA: Initialization
        SA ->> SM: Initialization
        SM ->> I: Initialization
        SM ->> ES: Initialization

        loop Health Check
            Q ->> SA: Check Health
            SA ->> SM: Check Health
            SM ->> I: Check Health
            SM ->> ES: Check Health
        end

        alt Pipeline Healthy
            SA ->> SM: Start Processing
            SM ->> I: Start Processing
            SM ->> ES: Start Processing
        else Pipeline Unhealthy
            SA ->> SM: Terminate
            SM ->> I: Terminate
            SM ->> ES: Terminate
            SA ->> Q: Retry Initialization
        end
```

## Architecture

![Architecure](docs/archv1.png?raw=true "Quester Architecture")

## Contributing

We welcome contributions to Quester! If you have ideas, bug reports, or feature requests, please [open an issue](https://github.com/querent-ai/quester/issues) or submit a pull request.

## License

Quester is licensed under the [BSL License](LICENSE).
