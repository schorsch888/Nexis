//! Message routing for Nexis Gateway

use axum::{
    extract::{Path, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    room_messages: Arc<RwLock<HashMap<String, Vec<StoredMessage>>>>,
    write_gate: Arc<Semaphore>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            room_messages: Arc::new(RwLock::new(HashMap::new())),
            write_gate: Arc::new(Semaphore::new(2_048)),
        }
    }
}

type SharedState = AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Room {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateRoomRequest {
    name: String,
    #[serde(default)]
    topic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateRoomResponse {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SendMessageRequest {
    #[serde(rename = "roomId")]
    room_id: String,
    sender: String,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct SendMessageResponse {
    id: String,
}

#[derive(Debug, Clone, Serialize)]
struct StoredMessage {
    id: String,
    sender: String,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct RoomInfoResponse {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    messages: Vec<StoredMessage>,
}

/// Build the main router for the gateway
pub fn build_routes() -> Router {
    let state = AppState::default();

    Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .route("/v1/rooms", post(create_room))
        .route("/v1/messages", post(send_message))
        .route("/v1/rooms/:id", get(get_room))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// WebSocket handler
async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn create_room(
    State(state): State<SharedState>,
    Json(payload): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    if payload.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "room name cannot be empty" })),
        )
            .into_response();
    }

    let room = Room {
        id: format!("room_{}", Uuid::new_v4().simple()),
        name: payload.name,
        topic: payload.topic,
    };

    let response = CreateRoomResponse {
        id: room.id.clone(),
        name: room.name.clone(),
    };

    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "service unavailable" })),
        )
            .into_response();
    };

    let mut rooms = state.rooms.write().await;
    rooms.insert(room.id.clone(), room);

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn send_message(
    State(state): State<SharedState>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    if payload.room_id.trim().is_empty() || payload.sender.trim().is_empty() || payload.text.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "roomId, sender, and text are required" })),
        )
            .into_response();
    }

    let rooms = state.rooms.read().await;
    if !rooms.contains_key(&payload.room_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "room not found" })),
        )
            .into_response();
    }
    drop(rooms);

    let message = StoredMessage {
        id: format!("msg_{}", Uuid::new_v4().simple()),
        sender: payload.sender,
        text: payload.text,
    };
    let response = SendMessageResponse {
        id: message.id.clone(),
    };

    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "service unavailable" })),
        )
            .into_response();
    };

    let mut messages = state.room_messages.write().await;
    messages
        .entry(payload.room_id)
        .or_default()
        .push(message);

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn get_room(State(state): State<SharedState>, Path(id): Path<String>) -> impl IntoResponse {
    let rooms = state.rooms.read().await;
    let Some(room) = rooms.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "room not found" })),
        )
            .into_response();
    };
    let room = room.clone();
    drop(rooms);

    let messages = state
        .room_messages
        .read()
        .await
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let response = RoomInfoResponse {
        id: room.id,
        name: room.name,
        topic: room.topic,
        messages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket) {
    use futures::{SinkExt, StreamExt};
    
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(256);

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });
    
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("Received: {}", text);
                if tx.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                tracing::debug!("Client disconnected");
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    writer.abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_check_returns_ok() {
        let app = build_routes();
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_room_returns_201_and_room_identity() {
        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rooms")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "general",
                            "topic": "team"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["name"], "general");
        assert!(payload["id"].as_str().unwrap().starts_with("room_"));
    }

    #[tokio::test]
    async fn send_message_returns_404_for_unknown_room() {
        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "roomId": "room_missing",
                            "sender": "alice",
                            "text": "hello"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_room_returns_messages_after_send() {
        let app = build_routes();

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rooms")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "general"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_payload: Value = serde_json::from_slice(&create_body).unwrap();
        let room_id = create_payload["id"].as_str().unwrap().to_string();

        let send_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "roomId": room_id.clone(),
                            "sender": "alice",
                            "text": "hello"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(send_response.status(), StatusCode::CREATED);

        let get_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/rooms/{}", room_id.clone()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::OK);
        let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let get_payload: Value = serde_json::from_slice(&get_body).unwrap();
        assert_eq!(get_payload["id"], room_id);
        assert_eq!(get_payload["messages"].as_array().unwrap().len(), 1);
        assert_eq!(get_payload["messages"][0]["text"], "hello");
    }
}
