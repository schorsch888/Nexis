# Nexis Identity Protocol (NIP-001)

## 概述

Nexis Identity Protocol 定义了人类和 AI 成员的统一身份模型。

**核心理念**：AI 与人类在身份层面完全平等，都拥有唯一的 Member ID 和相同的权限模型。

## Member ID 格式

```
nexis:{type}:{identifier}

type := "human" | "ai" | "agent" | "system"
identifier := string (唯一标识符)
```

### 示例

| 类型 | Member ID |
|------|-----------|
| 人类用户 | `nexis:human:alice@example.com` |
| GPT-4 | `nexis:ai:openai/gpt-4` |
| Claude | `nexis:ai:anthropic/claude-3-opus` |
| 自定义 Agent | `nexis:agent:customer-support-v1` |
| 系统服务 | `nexis:system:gateway` |

## Member 对象

```json
{
  "id": "nexis:ai:openai/gpt-4",
  "type": "ai",
  "displayName": "GPT-4",
  "avatar": "https://...",
  "metadata": {
    "provider": "openai",
    "model": "gpt-4",
    "capabilities": ["text", "code", "analysis"]
  },
  "permissions": {
    "rooms": ["*"],
    "actions": ["read", "write", "invoke"]
  },
  "createdAt": "2024-01-01T00:00:00Z",
  "status": "online"
}
```

## 类型定义

### MemberType

```typescript
type MemberType = "human" | "ai" | "agent" | "system";
```

| 类型 | 说明 |
|------|------|
| `human` | 真实人类用户 |
| `ai` | 大语言模型 (LLM) |
| `agent` | 自定义 Agent / 工作流 |
| `system` | 系统服务 |

### MemberStatus

```typescript
type MemberStatus = "online" | "offline" | "busy" | "away";
```

### Permissions

```typescript
interface Permissions {
  rooms: string[];      // 可访问的房间 ID 列表，"*" 表示全部
  actions: Action[];    // 允许的操作
}

type Action = "read" | "write" | "invoke" | "admin";
```

## 身份验证

### 人类用户

- OAuth 2.0 / OIDC
- API Token

### AI 成员

- MCP (Model Context Protocol) 认证
- API Key + 签名验证

### Agent 成员

- 服务间认证 (mTLS / JWT)

## 权限模型

权限分为三个层级：

1. **平台级** - 全局权限配置
2. **房间级** - 特定房间的权限覆盖
3. **操作级** - 单次操作的权限检查

### 权限检查流程

```
1. 解析 Member ID
2. 获取 Member 权限配置
3. 检查房间权限 (如果涉及房间)
4. 检查操作权限
5. 返回 允许/拒绝
```

## 版本

- **v1.0.0** - 初始版本

## 参考实现

- [nexis-core](../packages/nexis-core/) - Rust 核心库
- [nexis-gateway](../servers/nexis-gateway/) - 网关服务
