# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- AI Provider integration with OpenAI and Anthropic support
- Provider Registry for dynamic provider management
- Streaming response support for AI providers
- CLI `test-provider` command for testing AI integrations

### Changed
- Improved error handling in HTTP provider

### Security
- Added gitleaks and detect-secrets for secret scanning
- Configured CodeQL for static security analysis

## [0.1.0] - 2026-02-14

### Added
- Initial project structure with Rust workspace
- Core protocol definitions (NIP-001, NIP-002, NIP-003)
- Member identity system with support for human/ai/agent/system types
- Message protocol implementation
- WebSocket gateway with JWT authentication
- Basic CLI structure
- Comprehensive CI/CD pipeline with GitHub Actions
- Pre-commit hooks for security and code quality
- Docker and Docker Compose configuration
- Complete project documentation

### Security
- Security policy and reporting guidelines
- Automated dependency auditing with cargo-audit
- Secret scanning in CI/CD pipeline

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-02-14 | Initial release - Foundation phase complete |
| 0.2.0 | TBD | MVP release - AI integration + persistence |

---

[Unreleased]: https://github.com/schorsch888/Nexis/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/schorsch888/Nexis/releases/tag/v0.1.0
