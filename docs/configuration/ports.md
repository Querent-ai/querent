---
title: Ports
sidebar_position: 6
---

Internally, Querent will, in fact, use three sockets. The ports of these three sockets
cannot be configured independently at the moment.
The ports used are computed relative to the `rest.listen_port` port, as follows.

| Service                       | Port used                 | Protocol |  Default  |
|-------------------------------|---------------------------|----------|-----------|
| Http server with the rest api | `${rest.listen_port}`     |   TCP    | 7280      |
| Cluster membership            | `${rest.listen_port}`     |   UDP    | 7280      |
| GRPC service                  | `${rest.listen_port} + 1` |   TCP    | 7281      |

It is not possible for the moment to configure these ports independently.

In order to form a cluster, you will also need to define a `peer_seeds` parameter.
The following addresses are valid peer seed addresses:

| Type | Example without port | Example with port         |
|--------------|--------------|---------------------------|
| IPv4         | 172.1.0.12   | 172.1.0.12:7180           |
| IPv6         | 2001:0db8:85a3:0000:0000:8a2e:0370:7334  | [2001:0db8:85a3:0000:0000:8a2e:0370:7334:7180]:7280 |
| hostname     | node3        | node3:7180                |

If no port is specified in a peer node address, a Querent node will assume the peer is using the same port as itself.
