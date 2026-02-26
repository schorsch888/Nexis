# Nexis

<div align="center">

[![CI](https://img.shields.io/github/actions/workflow/status/schorsch888/Nexis/ci.yml?branch=main&label=ci)](https://github.com/schorsch888/Nexis/actions)
[![Security](https://img.shields.io/badge/security-policy-green)](SECURITY.md)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-index-blue)](docs/index.md)

**AI-Native Team Communication Platform / AI 原生团队协作平台**

</div>

## Table of Contents / 目录

- [Overview / 项目概述](#overview--项目概述)
- [Features / 核心能力](#features--核心能力)
- [Screenshot / 截图](#screenshot--截图)
- [Quick Start / 快速开始](#quick-start--快速开始)
- [Installation / 安装](#installation--安装)
- [Documentation / 文档导航](#documentation--文档导航)
- [Contributing / 参与贡献](#contributing--参与贡献)
- [Security / 安全](#security--安全)
- [License / 许可证](#license--许可证)

## Overview / 项目概述

Nexis is an AI-native collaboration platform where human members and AI members share a unified identity, messaging protocol, and runtime context.

Nexis 是一个 AI 原生协作平台，核心是让人类成员与 AI 成员在统一身份、统一消息协议、统一上下文中协作。

## Features / 核心能力

- Unified identity model (`human`, `ai`, `agent`, `system`) / 统一成员身份模型（`human`、`ai`、`agent`、`system`）
- Real-time gateway over WebSocket + JWT auth / 基于 WebSocket + JWT 的实时网关
- AI provider integration abstraction (OpenAI, Anthropic, Gemini adapters) / AI 提供商抽象层（OpenAI、Anthropic、Gemini）
- Rust workspace with modular crates / Rust 多 crate 模块化工作区
- Security baseline + enterprise profile docs / Baseline 与 Enterprise 双安全基线文档

## Screenshot / 截图

### Product Overview (Mock Screenshot) / 产品概览（示意图）

![Nexis Overview](docs/assets/nexis-overview.svg)

## Quick Start / 快速开始

```bash
# 1) Clone / 克隆
git clone https://github.com/schorsch888/Nexis.git
cd Nexis

# 2) Build workspace / 构建工作区
cargo build --workspace

# 3) Run gateway / 启动网关
cargo run -p nexis-gateway

# 4) Create a room via CLI / 通过 CLI 创建房间
cargo run -p nexis-cli -- create-room "general"
```

## Installation / 安装

### Prerequisites / 环境要求

- Rust `1.75+`
- Git `2.30+`
- Optional: Docker / 可选：Docker

### Local Development / 本地开发

```bash
# Format + lint + test
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Docker / 容器方式

```bash
docker-compose up -d
```

## Documentation / 文档导航

- [Documentation Home / 文档首页](docs/index.md)
- [Getting Started / 快速开始](docs/zh-CN/getting-started/quickstart.md)
- [Architecture / 架构设计](docs/en/architecture/tenant-model.md)
- [API Reference / API 参考](docs/en/api/reference.md)
- [Deployment Guide / 部署指南](docs/deployment/guide.md)
- [Development Guide / 开发指南](docs/development/guide.md)
- [Security Docs / 安全文档](docs/security/README.md)

## Contributing / 参与贡献

- [Contributing Guide / 贡献指南](CONTRIBUTING.md)
- [Code of Conduct / 行为准则](CODE_OF_CONDUCT.md)

## Security / 安全

Please report vulnerabilities privately via [SECURITY.md](SECURITY.md).

如发现安全漏洞，请通过 [SECURITY.md](SECURITY.md) 中的私有渠道提交。

## License / 许可证

Apache-2.0. See [LICENSE](LICENSE).
