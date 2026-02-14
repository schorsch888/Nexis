# Nexis

<div align="center">

**AI-Native Team Communication Platform**

*Where humans and AI collaborate seamlessly*

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/schorsch888/Nexis/ci.yml?branch=main)](https://github.com/schorsch888/Nexis/actions)

[English](README.md) | [中文](docs/README.zh-CN.md)

</div>

---

## 🎯 Vision

**Build a productivity platform where AI and humans collaborate seamlessly.**

Nexis is not another Slack or Feishu. It is an **AI-Native** collaboration platform built from scratch, where AI becomes a first-class citizen, not just a plugin.

### Core Principles

| Traditional IM | Nexis |
|----------------|-------|
| AI as plugin/Bot | AI as team member |
| Fragmented context | Unified semantic layer |
| Passive response | Proactive collaboration |
| Single AI integration | Multi-AI collaboration |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Nexis Platform                          │
├──────────────────┬──────────────────┬───────────────────────┤
│   Nexis ID       │   Nexis Core     │   Nexis UI            │
│   ━━━━━━━━━      │   ━━━━━━━━━━     │   ━━━━━━━━━           │
│   AI Protocol    │   Semantic Layer │   Minimal UI          │
│                  │                  │                       │
│   • Identity     │   • Vector Store │   • CUI + GUI         │
│   • Permissions  │   • Knowledge    │   • Workflow          │
│   • MCP Integration│ • Context Engine│   • Multi-platform  │
└──────────────────┴──────────────────┴───────────────────────┘
```

### Three Pillars

#### Pillar A: AI Integration Protocol (NIP)

Let any LLM or Agent become a team member with:
- **Unified Identity** - `nexis:ai:openai/gpt-4` vs `nexis:human:alice@example.com`
- **Permission Control** - Room-level and operation-level permissions
- **Interaction Interface** - Same message protocol as humans

📖 [NIP-001: Identity Protocol](protocol/nexis-id.md) | [NIP-002: Message Protocol](protocol/nexis-msg.md) | [NIP-003: MCP Integration](protocol/nexis-mcp.md)

#### Pillar B: Semantic Data Layer

Break the boundaries between documents, messages, and tables:
- **Vector Storage** - All content automatically vectorized
- **Knowledge Graph** - Relationships between people, tasks, documents
- **Unified Context** - AI understands full context

#### Pillar C: Minimal Interaction

- **CUI First** - CLI interface, developer-friendly
- **AI Collaboration** - Multi-AI parallel, voting, discussion
- **Workflow Orchestration** - Visual AI task flows

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 20+ (for web app)
- PostgreSQL 15+ (planned)
- Qdrant (planned)

### Installation

```bash
# Clone the repository
git clone https://github.com/schorsch888/Nexis.git
cd Nexis

# Build workspace
cargo build --workspace

# Run CLI
cargo run -p nexis-cli -- create-room "general"

# Run gateway
cargo run -p nexis-gateway
```

### Docker

```bash
docker-compose up -d
```

---

## 📦 Project Structure

```
nexis/
├── crates/                    # Rust workspace
│   ├── nexis-core/           # Core library
│   ├── nexis-protocol/       # Protocol definitions
│   ├── nexis-mcp/            # MCP integration
│   ├── nexis-gateway/        # Control Plane
│   ├── nexis-runtime/        # Agent Runtime
│   └── nexis-cli/            # CLI client
├── libs/                      # Shared libraries
│   └── typescript/           # TypeScript SDK
├── apps/                      # Applications
│   └── web/                  # Web frontend
├── proto/                     # Protocol Buffers
├── config/                    # Configuration
├── tests/                     # Integration tests
├── docs/                      # Documentation
│   ├── security/             # Security docs
│   └── plans/                # Design docs
└── protocol/                  # Protocol specs
```

### Runtime Status (M3)

| Module | Status | Notes |
|--------|--------|-------|
| nexis-protocol | ✅ ready | NIP-001/002 types with tests |
| nexis-core | ✅ ready | Re-exports + domain extensions |
| nexis-gateway | ✅ ready | WebSocket + JWT auth |
| nexis-runtime | 🔄 stub | Provider trait ready |
| nexis-mcp | 🔄 stub | Interface ready |
| nexis-cli | 📝 planned | Basic structure |
| MCP providers | 📝 stub | Interface ready, real adapters pending |

---

## 🛠️ Development

### Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies
cargo install cargo-watch cargo-audit

# Run tests
cargo test --all

# Run with hot reload
cargo watch -x run
```

### Code Style

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Security audit
cargo audit
```

### Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(core): add member identity system
fix(gateway): resolve websocket connection leak
docs: update API documentation
test(core): add unit tests for MemberId
```

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details.

---

## 🗺️ Roadmap

### Phase 1: Foundation ✅
- [x] Protocol specification (NIP-001, NIP-002, NIP-003)
- [x] Core identity system
- [x] Message protocol implementation
- [x] Basic gateway with WebSocket

### Phase 2: MVP (Current)
- [ ] Real AI provider integration (OpenAI, Anthropic, Gemini)
- [ ] Single room + multi-user
- [ ] Message persistence
- [ ] CLI client

### Phase 3: Intelligence
- [ ] Vector storage (Qdrant)
- [ ] Context engine
- [ ] Knowledge graph
- [ ] Semantic search

### Phase 4: Scale
- [ ] Multi-tenant support
- [ ] Federation protocol
- [ ] Web UI
- [ ] Mobile apps

---

## 🔐 Security

See [SECURITY.md](SECURITY.md) for:
- Vulnerability reporting
- Security features
- Audit and compliance

---

## 🤝 Contributing

We welcome contributions! Please see:
- [Contributing Guide](docs/CONTRIBUTING.md)
- [Code of Conduct](docs/CODE_OF_CONDUCT.md)

---

## 📄 License

This project is licensed under the Apache-2.0 License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - AI integration standard
- [Matrix Protocol](https://matrix.org/) - Decentralized communication inspiration

---

<div align="center">

**Built with ❤️ by the Nexis Team**

[Discord](https://discord.gg/VMPC28gyQB) • [GitHub](https://github.com/schorsch888/Nexis)

</div>
