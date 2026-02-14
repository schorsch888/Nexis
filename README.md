# Nexis

**AI-Native Team Communication Platform**

打造一个 AI 与人类无缝协同的生产力平台。

## 核心理念

### 前提 A：AI 接入协议
建立一套标准化的"AI 接入协议"，让任何大模型或 Agent 都能像人类成员一样拥有身份、权限和交互界面。

### 前提 B：语义化数据底座
打破文档、消息、表格的界限，让 AI 能够理解全量语境。

### 前提 C：极简交互
CUI + GUI 混合界面，降低人类调度 AI 的成本。

## 架构

```
nexis/
├── protocol/           # 协议规范
│   ├── nexis-id.md    # 身份协议
│   ├── nexis-msg.md   # 消息协议
│   └── nexis-mcp.md   # AI 接入协议
├── packages/
│   ├── nexis-core/    # 核心库 (Rust)
│   └── nexis-cli/     # 命令行客户端
├── servers/
│   └── nexis-gateway/ # 网关服务
└── docs/              # 文档
```

## 快速开始

```bash
# 开发中...
```

## License

MIT
