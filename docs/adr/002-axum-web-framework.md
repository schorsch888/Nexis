# ADR-002: Axum Web Framework

## Status

Accepted

## Context

We need a web framework for:
- WebSocket handling
- REST API endpoints
- Middleware support (auth, logging, CORS)
- Integration with async Rust ecosystem

Options considered:
- Actix-web: Mature, but complex macro system
- Warp: Functional, steeper learning curve
- Axum: Tower-based, type-safe, simpler mental model

## Decision

Use Axum as the web framework.

## Consequences

### Positive
- Built on Tower, excellent middleware ecosystem
- Type-safe extractors reduce runtime errors
- Simple, predictable API
- First-class WebSocket support
- Excellent integration with Tokio

### Negative
- Newer than Actix, smaller community
- Some patterns still evolving

### Mitigation
- Follow official examples
- Contribute upstream if gaps found
