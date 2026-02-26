# Architecture Overview

Nexis follows a layered architecture for protocol, runtime, and gateway responsibilities.

## Core Layers

- `nexis-protocol`: shared protocol and identity primitives
- `nexis-core`: core domain abstractions
- `nexis-runtime`: provider and tool execution runtime
- `nexis-gateway`: HTTP/WebSocket gateway and service orchestration

## Multi-Tenant Model

Entity hierarchy:

- Tenant
- Workspace
- Member
- Room
- Message

Tenant-aware boundaries are designed for strict data isolation and future enterprise controls.

## Data and Search

- Structured entities via Rust domain models
- Search services with vector integration points
- Extensible provider registry for AI backends

## Related Docs

- Existing model detail: [tenant-model.md](https://github.com/schorsch888/Nexis/blob/main/docs/en/architecture/tenant-model.md)
- ADRs: [docs/adr](https://github.com/schorsch888/Nexis/tree/main/docs/adr)
