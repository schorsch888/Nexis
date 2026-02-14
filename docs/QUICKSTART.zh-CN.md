# Nexis 快速开始

本指南帮助你在几分钟内启动 Nexis。

## 环境要求

- **Rust 1.75+** - [安装 Rust](https://rustup.rs/)
- **Git** - [安装 Git](https://git-scm.com/)

## 安装

### 1. 克隆仓库

```bash
git clone https://github.com/schorsch888/Nexis.git
cd Nexis
```

### 2. 构建项目

```bash
cargo build --release
```

### 3. 运行网关

```bash
cargo run --release -p nexis-gateway
```

网关将在 `http://127.0.0.1:8080` 启动。

## 使用 CLI

### 创建房间

```bash
cargo run --release -p nexis-cli -- create-room "general" --topic "团队讨论"
```

输出：
```
room created: room_abc123 (general)
```

### 发送消息

```bash
cargo run --release -p nexis-cli -- \
  send-message "room_abc123" \
  "nexis:human:alice@example.com" \
  "你好，Nexis！"
```

输出：
```
message sent: msg_xyz789
```

### WebSocket 连接

```bash
cargo run --release -p nexis-cli -- \
  connect --url "ws://127.0.0.1:8080/ws" \
  --message "ping"
```

输出：
```
ws reply: ping
```

## API 端点

| 方法 | 端点 | 描述 |
|------|------|------|
| `GET` | `/health` | 健康检查 |
| `POST` | `/v1/rooms` | 创建房间 |
| `POST` | `/v1/messages` | 发送消息 |
| `GET` | `/v1/rooms/:id` | 获取房间信息 |
| `GET` | `/ws` | WebSocket 连接 |

### 创建房间

```bash
curl -X POST http://127.0.0.1:8080/v1/rooms \
  -H "Content-Type: application/json" \
  -d '{"name": "general", "topic": "团队讨论"}'
```

响应：
```json
{
  "id": "room_abc123",
  "name": "general"
}
```

### 发送消息

```bash
curl -X POST http://127.0.0.1:8080/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "roomId": "room_abc123",
    "sender": "nexis:human:alice@example.com",
    "text": "你好，Nexis！"
  }'
```

响应：
```json
{
  "id": "msg_xyz789"
}
```

### 获取房间

```bash
curl http://127.0.0.1:8080/v1/rooms/room_abc123
```

响应：
```json
{
  "id": "room_abc123",
  "name": "general",
  "topic": "团队讨论",
  "messages": [
    {
      "id": "msg_xyz789",
      "sender": "nexis:human:alice@example.com",
      "text": "你好，Nexis！"
    }
  ]
}
```

## WebSocket

连接到 WebSocket 端点：

```javascript
const ws = new WebSocket('ws://127.0.0.1:8080/ws');

ws.onopen = () => {
  console.log('已连接');
  ws.send('来自浏览器的问候！');
};

ws.onmessage = (event) => {
  console.log('收到:', event.data);
};
```

## 下一步

- 阅读 [架构文档](docs/plans/2026-02-14-nexis-architecture-design.md)
- 了解 [NIP 协议](protocol/)
- 加入我们的 [Discord](https://discord.gg/VMPC28gyQB)

## 故障排除

### 端口被占用

如果端口 8080 被占用，设置其他端口：

```bash
NEXIS_BIND_ADDR=0.0.0.0:3000 cargo run --release -p nexis-gateway
```

### 构建错误

确保 Rust 版本 >= 1.75：

```bash
rustc --version
```

如需更新：

```bash
rustup update stable
```
