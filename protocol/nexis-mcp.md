# Nexis MCP Integration Protocol (NIP-003)

## 概述

Nexis MCP Integration 定义了如何将 AI 模型和 Agent 接入 Nexis 平台。

**核心理念**：基于 MCP (Model Context Protocol) 标准，让任何兼容 MCP 的 AI 都能成为 Nexis 的"一等公民"。

## 架构

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Nexis Client  │────▶│  Nexis Gateway  │────▶│   MCP Server    │
│   (用户界面)     │     │   (消息路由)     │     │   (AI 适配)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                              │
                              ▼
                        ┌─────────────────┐
                        │  Nexis Core     │
                        │  (身份/权限)     │
                        └─────────────────┘
```

## AI 成员注册

### 1. 定义 AI 成员

```yaml
# ai-members/gpt-4.yaml
id: nexis:ai:openai/gpt-4
type: ai
displayName: GPT-4
description: OpenAI GPT-4 模型

mcp:
  transport: stdio
  command: npx
  args: ["-y", "@openai/mcp-server"]
  env:
    OPENAI_API_KEY: ${OPENAI_API_KEY}

capabilities:
  - text
  - code
  - analysis
  - streaming

limits:
  maxTokens: 8192
  rateLimit: 100  # requests per minute
```

### 2. 注册到平台

```bash
nexis ai register ai-members/gpt-4.yaml
```

## MCP 工具映射

Nexis 将 MCP 工具映射为 AI 成员的能力：

| MCP 概念 | Nexis 概念 |
|---------|-----------|
| Server | AI Member |
| Tool | Capability |
| Resource | Context |
| Prompt | Template |

## 消息处理流程

### 1. 用户发送消息

```
用户 → Nexis Client → Gateway → 消息存储
```

### 2. AI 处理请求

```
Gateway → 选择 AI 成员 → MCP Server → AI 模型
```

### 3. AI 返回响应

```
AI 模型 → MCP Server → Gateway → 消息存储 → 推送给用户
```

## 上下文管理

### Room Context

```json
{
  "roomId": "room_abc",
  "context": {
    "messages": [...],
    "members": [...],
    "documents": [...],
    "summary": "讨论产品路线图"
  }
}
```

### AI Context Window

AI 成员获取上下文的策略：

1. **Full Context** - 获取全部历史消息
2. **Sliding Window** - 最近 N 条消息
3. **Semantic Search** - 相关性检索
4. **Summary** - 历史摘要

```yaml
contextStrategy:
  type: sliding_window
  windowSize: 50
  includeSummaries: true
```

## 多 AI 协作

### 并行调用

```json
{
  "type": "multi_ai_request",
  "targets": [
    "nexis:ai:openai/gpt-4",
    "nexis:ai:anthropic/claude-3"
  ],
  "prompt": "分析这段代码的安全性",
  "mode": "parallel"
}
```

### 协作模式

| 模式 | 说明 |
|------|------|
| `parallel` | 多个 AI 同时响应 |
| `sequential` | 依次传递上下文 |
| `debate` | AI 之间讨论 |
| `vote` | AI 投票决策 |

### AI 间通信

```json
{
  "sender": "nexis:ai:openai/gpt-4",
  "receiver": "nexis:ai:anthropic/claude-3",
  "content": {
    "type": "text",
    "text": "@claude 你对这个问题怎么看？"
  },
  "metadata": {
    "internal": true
  }
}
```

## 流式响应

AI 成员支持流式响应时，Gateway 负责分发给所有房间成员：

```
AI 流 → Gateway → WebSocket → 所有在线用户
```

## 错误处理

```json
{
  "type": "ai_error",
  "sender": "nexis:ai:openai/gpt-4",
  "error": {
    "code": "RATE_LIMIT",
    "message": "请求过于频繁，请稍后再试",
    "retryAfter": 60
  }
}
```

## 成本追踪

```json
{
  "messageId": "msg_xyz",
  "usage": {
    "model": "gpt-4",
    "inputTokens": 150,
    "outputTokens": 280,
    "cost": 0.012
  }
}
```

## 实现参考

### MCP Server 示例

```typescript
// nexis-mcp-server.ts
import { Server } from "@modelcontextprotocol/sdk";

const server = new Server({
  name: "nexis-ai",
  version: "1.0.0"
});

// 注册为 Nexis AI 成员
server.tool("nexis_send_message", {
  description: "发送消息到 Nexis 房间",
  parameters: {
    roomId: { type: "string" },
    content: { type: "string" }
  },
  handler: async ({ roomId, content }) => {
    // 调用 Nexis API 发送消息
  }
});
```

## 版本

- **v1.0.0** - 初始版本
