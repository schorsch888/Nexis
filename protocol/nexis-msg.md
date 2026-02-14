# Nexis Message Protocol (NIP-002)

## 概述

Nexis Message Protocol 定义了人类和 AI 之间交换的消息格式。

**核心理念**：消息格式统一，人类消息和 AI 消息结构相同，通过发送者 ID 区分来源。

## Message 格式

```json
{
  "id": "msg_abc123",
  "roomId": "room_xyz",
  "sender": "nexis:ai:openai/gpt-4",
  "content": {
    "type": "text",
    "text": "你好，有什么可以帮助你的？"
  },
  "metadata": {
    "model": "gpt-4",
    "tokens": { "input": 15, "output": 12 }
  },
  "replyTo": "msg_def456",
  "createdAt": "2024-01-01T12:00:00Z",
  "updatedAt": null
}
```

## 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 消息唯一 ID |
| `roomId` | string | 是 | 所属房间 ID |
| `sender` | Member ID | 是 | 发送者 ID |
| `content` | Content | 是 | 消息内容 |
| `metadata` | object | 否 | 扩展元数据 |
| `replyTo` | string | 否 | 回复的消息 ID |
| `createdAt` | timestamp | 是 | 创建时间 |
| `updatedAt` | timestamp | 否 | 更新时间 |

## Content 类型

### 1. 文本消息 (text)

```json
{
  "type": "text",
  "text": "这是一条文本消息"
}
```

### 2. Markdown 消息 (markdown)

```json
{
  "type": "markdown",
  "text": "# 标题\n\n这是 **加粗** 文本"
}
```

### 3. 代码块 (code)

```json
{
  "type": "code",
  "language": "python",
  "code": "print('Hello, Nexis!')"
}
```

### 4. 结构化数据 (data)

```json
{
  "type": "data",
  "format": "json",
  "data": { "key": "value" }
}
```

### 5. 富媒体 (media)

```json
{
  "type": "media",
  "mediaType": "image",
  "url": "https://...",
  "thumbnail": "https://...",
  "alt": "图片描述"
}
```

### 6. 工具调用 (tool_call)

```json
{
  "type": "tool_call",
  "toolId": "web_search",
  "arguments": { "query": "Nexis protocol" }
}
```

### 7. 工具结果 (tool_result)

```json
{
  "type": "tool_result",
  "toolCallId": "tc_abc123",
  "result": { "status": "success", "data": [...] }
}
```

### 8. 系统消息 (system)

```json
{
  "type": "system",
  "action": "member_joined",
  "data": { "member": "nexis:human:alice@example.com" }
}
```

### 9. 思考过程 (thinking) - AI 专用

```json
{
  "type": "thinking",
  "text": "让我分析一下这个问题...",
  "duration_ms": 150
}
```

## 消息状态

```typescript
type MessageStatus =
  | "sending"    // 发送中
  | "sent"       // 已发送
  | "delivered"  // 已送达
  | "read"       // 已读
  | "failed";    // 发送失败
```

## 消息流式传输

对于 AI 生成的长消息，支持流式传输：

```json
{
  "type": "stream_start",
  "messageId": "msg_abc123",
  "sender": "nexis:ai:anthropic/claude-3"
}
```

```json
{
  "type": "stream_chunk",
  "messageId": "msg_abc123",
  "delta": "你好"
}
```

```json
{
  "type": "stream_chunk",
  "messageId": "msg_abc123",
  "delta": "，有什"
}
```

```json
{
  "type": "stream_end",
  "messageId": "msg_abc123",
  "usage": { "input": 15, "output": 128 }
}
```

## 消息引用与回复

### 引用消息

```json
{
  "id": "msg_reply",
  "sender": "nexis:human:bob@example.com",
  "content": { "type": "text", "text": "同意这个观点" },
  "replyTo": "msg_original",
  "mentions": ["nexis:human:alice@example.com"]
}
```

### 提及成员

```json
{
  "mentions": [
    "nexis:ai:openai/gpt-4",
    "nexis:human:alice@example.com"
  ]
}
```

## 消息线程

支持线程回复：

```json
{
  "id": "msg_thread_1",
  "roomId": "room_xyz",
  "threadId": "thread_abc",
  "sender": "nexis:human:alice@example.com",
  "content": { "type": "text", "text": "线程中的回复" }
}
```

## 版本

- **v1.0.0** - 初始版本
