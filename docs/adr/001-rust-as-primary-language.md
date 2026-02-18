# ADR-001: Rust as Primary Language

## Status

Accepted

## Context

Nexis is an AI-Native team communication platform that requires:
- High concurrency (thousands of WebSocket connections)
- Low latency (real-time messaging)
- Memory safety (handling user data)
- Performance (AI provider integration)

Options considered:
- Go: Good concurrency, garbage collection pauses
- Node.js: Great ecosystem, single-threaded limitations
- Rust: Zero-cost abstractions, memory safety, async support

## Decision

Use Rust as the primary backend language.

## Consequences

### Positive
- Memory safety without garbage collection
- Excellent async/await support with Tokio
- Type safety reduces runtime errors
- High performance (no GC pauses)
- Growing ecosystem for web services (Axum, Tower)

### Negative
- Steeper learning curve
- Slower development velocity initially
- Smaller talent pool compared to Go/Node.js

### Mitigation
- Comprehensive documentation
- Clear code patterns
- Code review process
