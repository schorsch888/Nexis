# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Documentation structure normalized into strict `docs/en` and `docs/zh-CN` trees.
- `CODE_OF_CONDUCT.md` based on Contributor Covenant.
- `docs/en/getting-started/development-guide.md` as the development handbook entry.
- **nexis-meeting**: `SfuRoom::try_join_room()` with capacity enforcement.
- **nexis-meeting**: `SfuRoom::leave_room()` and cleanup methods.

### Changed
- Root `README.md` is now English only.
- `README.zh-CN.md` is now Chinese only.
- Security policy now points to the new architecture security docs.
- **web**: Upgraded vite 5→6, vitest 1→4 to fix security vulnerabilities.
- **web**: Updated tsconfig to ES2022 for `.at()` support.
- **web**: Excluded e2e tests (playwright) from vitest runner.

### Fixed
- Internal markdown links updated after docs reorganization.
- **nexis-meeting**: SfuRoom now properly enforces `max_participants` via `try_join_room()`.
- **web**: Fixed duplicate `ConnectionState` export in messages/index.ts.
- **web**: Fixed import path in authStore.test.ts.
- **web**: Fixed mock AxiosResponse shape in messagesStore.test.ts.
- **web**: Removed unused imports to pass strict TS checks.
- **web**: npm audit now reports 0 vulnerabilities.

## [0.1.0] - 2026-02-14

### Added
- Initial Rust workspace and crate layout.
- Core protocols (NIP-001, NIP-002, NIP-003).
- WebSocket gateway with JWT authentication.
- CLI foundations and core workflows.
- CI/CD baseline with code quality and security checks.

### Security
- Initial `SECURITY.md` policy and private disclosure channel.
- Automated dependency and secret scanning in CI.

## Version Links

[Unreleased]: https://github.com/gbrothersgroup/Nexis/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gbrothersgroup/Nexis/releases/tag/v0.1.0
