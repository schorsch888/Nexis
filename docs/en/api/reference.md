# API Reference

Nexis provides REST and WebSocket APIs for integration.

## Base URL

```
https://api.nexis.ai
```

All v1 endpoints are prefixed with `/v1`.

## Authentication

All `/v1/*` API requests require a JWT token in the Authorization header:

```
Authorization: Bearer <token>
```

Unauthenticated requests to protected endpoints return `401 Unauthorized`.

## Endpoints

### Health

| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | /health | Health check | No |

**Response:** `200 OK` - Plain text `OK`

### Rooms

| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | /v1/rooms | List rooms | Yes |
| POST | /v1/rooms | Create a room | Yes |
| GET | /v1/rooms/{id} | Get room details | Yes |
| DELETE | /v1/rooms/{id} | Delete a room | Yes |
| POST | /v1/rooms/{id}/invite | Invite member | Yes |

#### GET /v1/rooms

Query parameters:
- `limit` (optional, default: 100, max: 1000) - Max rooms to return
- `offset` (optional, default: 0) - Pagination offset

Response:
```json
{
  "rooms": [
    {
      "id": "room_abc123",
      "name": "general",
      "topic": "Team chat",
      "member_count": 5
    }
  ],
  "total": 42
}
```

#### POST /v1/rooms

Request:
```json
{
  "name": "general",
  "topic": "Team chat"
}
```

Response: `201 Created`
```json
{
  "id": "room_abc123",
  "name": "general"
}
```

#### GET /v1/rooms/{id}

Response:
```json
{
  "id": "room_abc123",
  "name": "general",
  "topic": "Team chat",
  "messages": [
    {
      "id": "msg_xyz",
      "sender": "alice",
      "text": "Hello!",
      "reply_to": null
    }
  ]
}
```

#### DELETE /v1/rooms/{id}

Response: `204 No Content` (empty body)

### Messages

| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| POST | /v1/messages | Send a message | Yes |

#### POST /v1/messages

Request:
```json
{
  "roomId": "room_abc123",
  "sender": "alice",
  "text": "Hello, world!",
  "replyTo": null
}
```

Response: `201 Created`
```json
{
  "id": "msg_xyz"
}
```

### Search

| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | /v1/search | Semantic search | Yes |
| POST | /v1/search | Semantic search | Yes |

#### GET /v1/search

Query parameters:
- `q` (required) - Search query string
- `limit` (optional, default: 10) - Max results
- `min_score` (optional) - Minimum relevance score
- `room_id` (optional, UUID) - Filter by room

Response:
```json
{
  "query": "project updates",
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "score": 0.95,
      "content": "Here are the latest project updates...",
      "room_id": "550e8400-e29b-41d4-a716-446655440001"
    }
  ],
  "total": 5
}
```

#### POST /v1/search

Request body:
```json
{
  "query": "project updates",
  "limit": 10,
  "min_score": 0.5,
  "room_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

Response: Same as GET /v1/search

## WebSocket

Connect to `/ws` for real-time messaging. No authentication required on the WebSocket endpoint.

### Events

- `message:create` - New message
- `message:update` - Message updated
- `room:join` - User joined room
- `room:leave` - User left room

## Error Response Format

All errors return a consistent JSON format:

```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE"
}
```

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| BAD_REQUEST | 400 | Invalid request parameters |
| UNAUTHORIZED | 401 | Missing or invalid authentication |
| FORBIDDEN | 403 | Insufficient permissions |
| NOT_FOUND | 404 | Resource not found |
| SERVICE_UNAVAILABLE | 503 | Service temporarily unavailable |
| INTERNAL_ERROR | 500 | Internal server error |
| INVALID_QUERY | 400 | Invalid search query |
| SEARCH_UNAVAILABLE | 503 | Search service not configured |

## Rate Limiting

API endpoints are protected by a write gate to prevent overload. When at capacity, endpoints return `503 Service Unavailable` with code `SERVICE_UNAVAILABLE`.
