# Nexis

<div align="center">

**AI-Native Team Communication Platform**

*Where humans and AI collaborate seamlessly*

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/schorsch888/Nexis/ci.yml?branch=main)](https://github.com/schorsch888/Nexis/actions)

[English](README.md) | [中文](docs/README.zh-CN.md)

</div>

---

## 🎯 Vision

**打造一个 AI 与人类无缝协同的生产力平台。**

Nexis 不是另一个 Slack 或 Feishu。它是从零开始构建的 **AI-Native** 协作平台，让 AI 成为一等公民，而非外挂插件。

### 核心理念

| 传统 IM | Nexis |
|---------|-------|
| AI 是插件/Bot | AI 是团队成员 |
| 上下文碎片化 | 统一语义底座 |
| 被动响应 | 主动协作 |
| 单一 AI 接入 | 多 AI 协作 |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Nexis Platform                          │
├──────────────────┬──────────────────┬───────────────────────┤
│   Nexis ID       │   Nexis Core     │   Nexis UI            │
│   ━━━━━━━━━      │   ━━━━━━━━━━     │   ━━━━━━━━━           │
│   AI 接入协议     │   语义化数据层    │   极简交互界面         │
│                  │                  │                       │
│   • 统一身份      │   • 向量存储      │   • CUI + GUI         │
│   • 权限系统      │   • 知识图谱      │   • 工作流编排         │
│   • MCP 集成     │   • 上下文引擎    │   • 多端同步           │
└──────────────────┴──────────────────┴───────────────────────┘
```

### 三大前提

#### 前提 A：AI 接入协议 (NIP - Nexis Identity Protocol)

让任何大模型或 Agent 都能像人类成员一样拥有：
- **统一身份** - `nexis:ai:openai/gpt-4` vs `nexis:human:alice@example.com`
- **权限控制** - 房间级、操作级权限
- **交互界面** - 与人类相同的消息协议

📖 [NIP-001: Identity Protocol](protocol/nexis-id.md)

#### 前提 B：语义化数据底座

打破文档、消息、表格的界限：
- **向量化存储** - 所有内容自动向量化
- **知识图谱** - 人、任务、文档的关联
- **统一上下文** - AI 理解全量语境

#### 前提 C：极简交互

- **CUI 优先** - 命令行界面，开发者友好
- **AI 协作** - 多 AI 并行、投票、讨论
- **工作流编排** - 可视化 AI 任务流

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 20+
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

# Run web shell
cd apps/web && npm install && npm run dev
```

### Runtime Status (M3)

| Module | Status | Notes |
|--------|--------|-------|
| `packages/nexis-core` | minimal | NIP-001/002 primitives + validation |
| `packages/nexis-cli` | minimal | `create-room`, `send`, `member parse` |
| `servers/nexis-gateway` | minimal | `/health`, message endpoint, auth/mcp stubs |
| `apps/web` | shell | React + TypeScript + Vite bootstrap |
| MCP providers | stub | interface ready, provider adapters pending |
| Semantic engine | planned | vector/KG/intelligence capabilities pending |

---

## 📦 Project Structure

```
nexis/
├── Cargo.toml             # Workspace 配置（members + 共享依赖）
├── packages/
│   ├── nexis-core/       # Rust 核心库
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── identity/mod.rs
│   │   │   ├── message/mod.rs
│   │   │   ├── permission/mod.rs
│   │   │   └── context/mod.rs
│   │   └── Cargo.toml
│   │
│   └── nexis-cli/        # 命令行客户端
│       ├── src/lib.rs
│       └── Cargo.toml
│
├── servers/
│   └── nexis-gateway/    # WebSocket 网关
│       ├── src/
│       │   ├── lib.rs
│       │   ├── router/mod.rs
│       │   ├── auth/mod.rs
│       │   └── mcp/mod.rs
│       └── Cargo.toml
│
├── apps/
│   └── web/              # Web 前端 (React + TypeScript)
│
├── protocol/             # 协议规范
│   ├── nexis-id.md       # NIP-001: 身份协议
│   ├── nexis-msg.md      # NIP-002: 消息协议
│   └── nexis-mcp.md      # NIP-003: AI 接入协议
│
└── docs/                 # 文档
    └── plans/            # 设计与执行计划
```

---

## 🛠️ Development

### Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies
cargo install cargo-watch cargo-audit

# Run tests
cargo test --workspace

# Run with hot reload
cargo watch -x 'run -p nexis-gateway'
```

### Code Style

We follow strict code quality standards:

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

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

### Phase 1: Foundation (Current)
- [x] Protocol specification (NIP-001, NIP-002, NIP-003)
- [ ] Core identity system
- [ ] Message protocol implementation
- [ ] Basic CLI client

### Phase 2: MVP
- [ ] WebSocket gateway
- [ ] Single room + multi-user
- [ ] AI member integration (via MCP)
- [ ] Message persistence

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

## 🤝 Contributing

We welcome contributions! Please see:

- [Contributing Guide](docs/CONTRIBUTING.md)
- [Code of Conduct](docs/CODE_OF_CONDUCT.md)

### Development Philosophy

We follow the **Superpowers** methodology:

1. **Brainstorming** - Refine ideas through questions
2. **Design** - Create clear specifications
3. **Plan** - Break into bite-sized tasks
4. **TDD** - Red-Green-Refactor cycle
5. **Review** - Code quality checks

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - AI integration standard
- [Matrix Protocol](https://matrix.org/) - Decentralized communication inspiration
- [Superpowers](https://github.com/obra/superpowers) - Development methodology

---

<div align="center">

**Built with ❤️ by the Nexis Team**

[Website](https://nexis.ai) • [Documentation](https://docs.nexis.ai) • [Discord](https://discord.gg/VMPC28gyQB)

</div>
