# ADR-003: AI Provider Abstraction

## Status

Accepted

## Context

Nexis needs to integrate with multiple AI providers:
- OpenAI (GPT-4, GPT-4o-mini)
- Anthropic (Claude)
- Future: Gemini, local models, custom providers

Each provider has:
- Different API formats
- Different authentication methods
- Different streaming protocols

## Decision

Create a unified `AIProvider` trait with provider-specific implementations.

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError>;
    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError>;
}
```

Use Provider Registry pattern for dynamic registration and selection.

## Consequences

### Positive
- Easy to add new providers
- Consistent interface for all AI operations
- Support for fallback/redundancy
- Easy testing with mock providers

### Negative
- Abstraction may not capture provider-specific features
- Additional complexity layer

### Mitigation
- Allow provider-specific options via metadata field
- Document provider-specific behavior clearly
