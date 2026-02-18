# ADR-004: PostgreSQL for Message Persistence

## Status

Accepted

## Context

Nexis needs persistent storage for:
- Messages (chat history)
- Rooms (conversation containers)
- Members (user/ai identities)
- Future: Files, reactions, threads

Requirements:
- ACID compliance (reliability)
- JSON support (flexible message content)
- Full-text search (message search)
- Scalability (future growth)

Options considered:
- MongoDB: Flexible schema, weaker consistency
- PostgreSQL: Strong consistency, mature ecosystem
- ScyllaDB: High write throughput, complex operations

## Decision

Use PostgreSQL as the primary data store.

## Consequences

### Positive
- ACID compliance for reliability
- JSONB for flexible message content
- Full-text search built-in
- Mature Rust ecosystem (SQLx)
- Easy to find expertise

### Negative
- Horizontal scaling more complex than NoSQL
- Schema migrations require planning

### Mitigation
- Use connection pooling (SQLx)
- Design for future sharding if needed
- Use SQLx migrations for schema management
