---
title: Command-line options
sidebar_position: 50
---

Querent command line tool lets you start a Querent server and manage indexes (create, delete, ingest), splits and sources (create, delete, toggle). To start a server, `querent` needs a [node config file path](../configuration/node_config.md) that you can specify with `QUERENT_NODE_CONFIG` environment variable: `export QUERENT_NODE_CONFIG=./config/querent.config.yaml`.

This page documents all the available commands, related options, and environment variables.

## Commands

[Command-line synopsis syntax](https://developers.google.com/style/code-syntax)

### Help

`querent` or `querent --help` displays the list of available commands.

`querent <command name> --help` displays the documentation for the command and a usage example.

### Version

`querent --version` displays the version. It is helpful for reporting bugs.

### Syntax

The CLI is structured into high-level commands with subcommands.
`querent [command] [subcommand] [args]`.

* `command`: `serve`, `search`

### Semantic service

The semantic service runs semantic pipelines assigned by the control plane.

### Searcher service

Starts a web server at `rest_listing_address:rest_list_port` that exposes the [Querent REST API](rest-api.md)
where `rest_listing_address` and `rest_list_port` are defined in Querent config file (querent.yaml).
The node can optionally join a cluster using the `peer_seeds` parameter.
This list of node addresses is used to discover the remaining peer nodes in the cluster through a gossip protocol, see [chitchat](https://github.com/quickwit-oss/chitchat).

### Janitor service

The Janitor service runs maintenance tasks on indexes: garbage collection, documents delete, and retention policy tasks.

:::note
Querent needs to open the following port for cluster formation and workload distribution:

    TCP port (default is 7280) for REST API
    TCP and UDP port (default is 7280) for cluster membership protocol
    TCP port + 1 (default is 7281) for gRPC address for the distributed search

If ports are already taken, the serve command will fail.
:::
  
`querent  serve [args]`

*Synopsis*

```bash
querent serve
    [--config <config>]
```

*Options*

| Option | Description | Default |
|-----------------|-------------|--------:|
| `--config` | Config file location | `config/querent.yaml` |

*Examples*

*Starts an indexer and a metastore services*

```bash
querent serve --config=./config/querent.yaml
```
