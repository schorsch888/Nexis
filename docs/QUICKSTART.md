# Nexis Quick Start

This guide will help you get Nexis up and running in minutes.

## Prerequisites

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Git** - [Install Git](https://git-scm.com/)

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/schorsch888/Nexis.git
cd Nexis
```

### 2. Build the Project

```bash
cargo build --release
```

### 3. Run the Gateway

```bash
cargo run --release -p nexis-gateway
```

The gateway will start on `http://127.0.0.1:8080`.

## Using the CLI

### Create a Room

```bash
cargo run --release -p nexis-cli -- create-room "general" --topic "Team discussion"
```

Output:
```
room created: room_abc123 (general)
```

### Send a Message

```bash
cargo run --release -p nexis-cli -- \
  send-message "room_abc123" \
  "nexis:human:alice@example.com" \
  "Hello, Nexis!"
```

Output:
```
message sent: msg_xyz789
```

### Connect via WebSocket

```bash
cargo run --release -p nexis-cli -- \
  connect --url "ws://127.0.0.1:8080/ws" \
  --message "ping"
```

Output:
```
ws reply: ping
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/v1/rooms` | Create a room |
| `POST` | `/v1/messages` | Send a message |
| `GET` | `/v1/rooms/:id` | Get room info |
| `GET` | `/ws` | WebSocket connection |

### Create Room

```bash
curl -X POST http://127.0.0.1:8080/v1/rooms \
  -H "Content-Type: application/json" \
  -d '{"name": "general", "topic": "Team discussion"}'
```

Response:
```json
{
  "id": "room_abc123",
  "name": "general"
}
```

### Send Message

```bash
curl -X POST http://127.0.0.1:8080/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "roomId": "room_abc123",
    "sender": "nexis:human:alice@example.com",
    "text": "Hello, Nexis!"
  }'
```

Response:
```json
{
  "id": "msg_xyz789"
}
```

### Get Room

```bash
curl http://127.0.0.1:8080/v1/rooms/room_abc123
```

Response:
```json
{
  "id": "room_abc123",
  "name": "general",
  "topic": "Team discussion",
  "messages": [
    {
      "id": "msg_xyz789",
      "sender": "nexis:human:alice@example.com",
      "text": "Hello, Nexis!"
    }
  ]
}
```

## WebSocket

Connect to the WebSocket endpoint:

```javascript
const ws = new WebSocket('ws://127.0.0.1:8080/ws');

ws.onopen = () => {
  console.log('Connected');
  ws.send('Hello from browser!');
};

ws.onmessage = (event) => {
  console.log('Received:', event.data);
};
```

## Next Steps

- Read the [Architecture Documentation](docs/plans/2026-02-14-nexis-architecture-design.md)
- Learn about [NIP Protocols](protocol/)
- Join our [Discord](https://discord.gg/VMPC28gyQB)

## Troubleshooting

### Port Already in Use

If port 8080 is in use, set a different port:

```bash
NEXIS_BIND_ADDR=0.0.0.0:3000 cargo run --release -p nexis-gateway
```

### Build Errors

Make sure you have Rust 1.75+:

```bash
rustc --version
```

Update Rust if needed:

```bash
rustup update stable
```
