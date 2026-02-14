# Nexis

<div align="center">

**AI 原生团队协作平台**

*让人与 AI 无缝协作*

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/schorsch888/Nexis/ci.yml?branch=main)](https://github.com/schorsch888/Nexis/actions)

[English](../README.md) | **中文**

</div>

---

## 🎯 愿景

**打造一个 AI 与人类无缝协同的生产力平台。**

Nexis 不是另一个 Slack 或飞书。它是从零开始构建的 **AI 原生** 协作平台，让 AI 成为一等公民，而非外挂插件。

### 核心理念

| 传统 IM | Nexis |
|---------|-------|
| AI 是插件/Bot | AI 是团队成员 |
| 上下文碎片化 | 统一语义底座 |
| 被动响应 | 主动协作 |
| 单一 AI 接入 | 多 AI 协作 |

---

## 🏗️ 架构

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

### 三大支柱

#### 支柱 A：AI 接入协议 (NIP)

让任何大模型或 Agent 都能像人类成员一样拥有：
- **统一身份** - `nexis:ai:openai/gpt-4` vs `nexis:human:alice@example.com`
- **权限控制** - 房间级、操作级权限
- **交互界面** - 与人类相同的消息协议

📖 [NIP-001: 身份协议](../protocol/nexis-id.md) | [NIP-002: 消息协议](../protocol/nexis-msg.md) | [NIP-003: MCP 集成](../protocol/nexis-mcp.md)

#### 支柱 B：语义化数据层

打破文档、消息、表格的界限：
- **向量存储** - 所有内容自动向量化
- **知识图谱** - 人、任务、文档的关联
- **统一上下文** - AI 理解全量语境

#### 支柱 C：极简交互

- **CUI 优先** - 命令行界面，开发者友好
- **AI 协作** - 多 AI 并行、投票、讨论
- **工作流编排** - 可视化 AI 任务流

---

## 🚀 快速开始

### 环境要求

- Rust 1.75+
- Node.js 20+（Web 应用）
- PostgreSQL 15+（计划中）
- Qdrant（计划中）

### 安装

```bash
# 克隆仓库
git clone https://github.com/schorsch888/Nexis.git
cd Nexis

# 构建工作区
cargo build --workspace

# 运行 CLI
cargo run -p nexis-cli -- create-room "general"

# 运行网关
cargo run -p nexis-gateway
```

### Docker

```bash
docker-compose up -d
```

---

## 📦 项目结构

```
nexis/
├── crates/                    # Rust 工作区
│   ├── nexis-core/           # 核心库
│   ├── nexis-protocol/       # 协议定义
│   ├── nexis-mcp/            # MCP 集成
│   ├── nexis-gateway/        # 控制面
│   ├── nexis-runtime/        # 执行面
│   └── nexis-cli/            # CLI 客户端
├── libs/                      # 共享库
│   └── typescript/           # TypeScript SDK
├── apps/                      # 应用
│   └── web/                  # Web 前端
├── proto/                     # Protocol Buffers
├── config/                    # 配置文件
├── tests/                     # 集成测试
├── docs/                      # 文档
│   ├── security/             # 安全文档
│   └── plans/                # 设计文档
└── protocol/                  # 协议规范
```

### 运行状态 (M3)

| 模块 | 状态 | 说明 |
|------|------|------|
| nexis-protocol | ✅ 就绪 | NIP-001/002 类型与测试 |
| nexis-core | ✅ 就绪 | 重导出 + 领域扩展 |
| nexis-gateway | ✅ 就绪 | WebSocket + JWT 认证 |
| nexis-runtime | 🔄 存根 | Provider trait 就绪 |
| nexis-mcp | 🔄 存根 | 接口就绪 |
| nexis-cli | 📝 计划中 | 基础结构 |
| MCP 适配器 | 📝 存根 | 接口就绪，真实适配器待实现 |

---

## 🛠️ 开发

### 设置

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装依赖
cargo install cargo-watch cargo-audit

# 运行测试
cargo test --all

# 热重载运行
cargo watch -x run
```

### 代码风格

```bash
# 格式化代码
cargo fmt

# Lint 检查
cargo clippy -- -D warnings

# 安全审计
cargo audit
```

### 提交规范

使用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：

```
feat(core): 添加成员身份系统
fix(gateway): 修复 WebSocket 连接泄漏
docs: 更新 API 文档
test(core): 添加 MemberId 单元测试
```

详见 [贡献指南](CONTRIBUTING.md)。

---

## 🗺️ 路线图

### 阶段 1：基础 ✅
- [x] 协议规范 (NIP-001, NIP-002, NIP-003)
- [x] 核心身份系统
- [x] 消息协议实现
- [x] 基础网关与 WebSocket

### 阶段 2：MVP（当前）
- [ ] 真实 AI 适配器集成 (OpenAI, Anthropic, Gemini)
- [ ] 单房间 + 多用户
- [ ] 消息持久化
- [ ] CLI 客户端

### 阶段 3：智能
- [ ] 向量存储 (Qdrant)
- [ ] 上下文引擎
- [ ] 知识图谱
- [ ] 语义搜索

### 阶段 4：扩展
- [ ] 多租户支持
- [ ] 联邦协议
- [ ] Web UI
- [ ] 移动应用

---

## 🔐 安全

参见 [安全策略](../SECURITY.md)：
- 漏洞报告
- 安全特性
- 审计与合规

---

## 🤝 贡献

欢迎贡献！请查看：
- [贡献指南](CONTRIBUTING.md)
- [行为准则](CODE_OF_CONDUCT.md)

---

## 📄 许可证

本项目采用 Apache-2.0 许可证 - 详见 [LICENSE](../LICENSE) 文件。

---

## 🙏 致谢

- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - AI 集成标准
- [Matrix Protocol](https://matrix.org/) - 去中心化通信灵感

---

<div align="center">

**由 Nexis 团队用 ❤️ 构建**

[Discord](https://discord.gg/VMPC28gyQB) • [GitHub](https://github.com/schorsch888/Nexis)

</div>
