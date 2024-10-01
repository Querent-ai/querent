---
title: Deployment Types
sidebar_position: 1
---

As an application, Querent is built out of multiple services and is designed to run as a horizontally-scalable distributed system. Currently, Querent supports four core services (semantics, searcher, control plane) and one maintenance service (janitor):

- Semantics pipelines ingest documents from data sources and build semantic knowledge.
- Searchers execute search queries submitted via the REST API.
- The Control Plane distributes and coordinates indexing workloads on indexers.
- The Janitor performs periodic maintenance tasks.

Querent is distributed as a single binary or Docker image. The behavior of that executable file or image is controlled with the `serve` option of the `querent serve` command and defines which services run on a node. You may start one service, multiple, or all of them. Nodes always serve the REST API and the search and admin UI. In addition, they will redirect requests that they cannot satisfy to the appropriate nodes in the cluster. Finally, each service can run on one or several nodes depending on the expected load on the system.

## Standalone mode (single node)

This deployment mode is the simplest way to get started with Querent. Launch all the services with the `querent serve` [command](../reference/cli.md), and you are now ready to ingest data and search your indexes.

## Cluster mode (multi-node)

You can deploy Querent on multiple nodes. We provide a [Helm chart](kubernetes.md) to help you deploy Querent on Kubernetes. In cluster mode, you must store your index data on a shared storage backend such as Postgres, or Neo4j. The shared storage backend is used to store the index data and metadata, and it is accessed by all the nodes in the cluster.
