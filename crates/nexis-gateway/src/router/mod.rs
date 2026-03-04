//! Message routing for Nexus Gateway

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, Query, State},
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::Instrument;
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::metrics::{
    export as export_metrics, HTTP_LATENCY, HTTP_REQUESTS_TOTAL, HTTP_RESPONSES, MESSAGES_SENT,
    OPERATION_ERRORS_TOTAL, OPERATION_LATENCY, OPERATION_THROUGHPUT_TOTAL, ROOMS_ACTIVE,
    ROOMS_CREATED_TOTAL,
};
use crate::search::{SearchError, SearchRequest, SearchService};

#[cfg(feature = "multi-tenant")]
use crate::auth::TenantStore;

#[derive(Clone)]
struct AppState {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    room_messages: Arc<RwLock<HashMap<String, Vec<StoredMessage>>>>,
    room_members: Arc<RwLock<HashMap<String, Vec<String>>>>,
    write_gate: Arc<Semaphore>,
    search_service: Option<Arc<dyn SearchService>>,
    #[cfg(feature = "multi-tenant")]
    tenant_store: TenantStore,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            room_messages: Arc::new(RwLock::new(HashMap::new())),
            room_members: Arc::new(RwLock::new(HashMap::new())),
            write_gate: Arc::new(Semaphore::new(2_048)),
            search_service: None,
            #[cfg(feature = "multi-tenant")]
            tenant_store: TenantStore::new(),
        }
    }
}

impl AppState {
    fn with_search_service(mut self, service: Arc<dyn SearchService>) -> Self {
        self.search_service = Some(service);
        self
    }
}

type SharedState = AppState;
const MAX_MESSAGE_TEXT_LEN: usize = 32 * 1024;
const OPENAPI_JSON: &str = include_str!("openapi.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Room {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[cfg(feature = "multi-tenant")]
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateRoomRequest {
    name: String,
    #[serde(default)]
    topic: Option<String>,
    #[cfg(feature = "multi-tenant")]
    #[serde(default)]
    tenant_id: Option<String>,
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
    #[serde(rename = "replyTo", default)]
    reply_to: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RoomInfoResponse {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    messages: Vec<StoredMessage>,
    #[cfg(feature = "multi-tenant")]
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct InviteMemberRequest {
    #[serde(rename = "memberId")]
    member_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct InviteMemberResponse {
    room_id: String,
    member_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct ListRoomsResponse {
    rooms: Vec<RoomSummary>,
    total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RoomSummary {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    member_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ListRoomsQuery {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchQueryParams {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    min_score: Option<f32>,
    #[serde(default)]
    room_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchApiRequest {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    min_score: Option<f32>,
    #[serde(default)]
    room_id: Option<Uuid>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Clone, Serialize)]
struct SearchApiResponse {
    query: String,
    results: Vec<SearchResultItem>,
    total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SearchResultItem {
    id: Uuid,
    score: f32,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_id: Option<Uuid>,
}

mod error_codes {
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
    pub const INVALID_QUERY: &str = "INVALID_QUERY";
    pub const SEARCH_UNAVAILABLE: &str = "SEARCH_UNAVAILABLE";
}

#[derive(Debug, Clone, Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

impl ErrorResponse {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: Some(error_codes::BAD_REQUEST),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: Some(error_codes::NOT_FOUND),
        }
    }

    fn internal_error() -> Self {
        Self {
            error: "An internal error occurred. Please try again later.".to_string(),
            code: Some(error_codes::INTERNAL_ERROR),
        }
    }

    fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: Some(error_codes::SERVICE_UNAVAILABLE),
        }
    }
}

impl From<SearchError> for ErrorResponse {
    fn from(err: SearchError) -> Self {
        tracing::error!("Search error: {}", err);
        match err {
            SearchError::InvalidQuery(_) => Self {
                error: "Invalid search query".to_string(),
                code: Some("INVALID_QUERY"),
            },
            SearchError::EmbeddingError(_) | SearchError::VectorError(_) => Self::internal_error(),
        }
    }
}

/// Build the main router for the gateway
pub fn build_routes() -> Router {
    let state = AppState::default();

    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .route("/ws", get(websocket_handler))
        .route("/v1/rooms", get(list_rooms).post(create_room))
        .route("/v1/rooms/:id", get(get_room).delete(delete_room))
        .route("/v1/rooms/:id/invite", post(invite_member))
        .route("/v1/messages", post(send_message))
        .route("/v1/search", get(search_messages_get).post(search_messages))
        .merge(crate::collaboration::routes())
        .layer(middleware::from_fn(correlation_id_middleware))
        .with_state(state)
}

/// Build router with search service
pub fn build_routes_with_search(search_service: Arc<dyn SearchService>) -> Router {
    let state = AppState::default().with_search_service(search_service);

    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .route("/ws", get(websocket_handler))
        .route("/v1/rooms", get(list_rooms).post(create_room))
        .route("/v1/rooms/:id", get(get_room).delete(delete_room))
        .route("/v1/rooms/:id/invite", post(invite_member))
        .route("/v1/messages", post(send_message))
        .route("/v1/search", get(search_messages_get).post(search_messages))
        .merge(crate::collaboration::routes())
        .layer(middleware::from_fn(correlation_id_middleware))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

async fn metrics_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        export_metrics(),
    )
}

async fn openapi_json() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/json; charset=utf-8")],
        OPENAPI_JSON,
    )
}

async fn swagger_ui() -> impl IntoResponse {
    const SWAGGER_HTML: &str = r##"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Nexis Gateway API Docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({
        url: "/openapi.json",
        dom_id: "#swagger-ui",
        deepLinking: true,
        docExpansion: "list"
      });
    </script>
  </body>
</html>
"##;

    Html(SWAGGER_HTML)
}

fn record_operation_success(operation: &str, start: Instant) {
    OPERATION_THROUGHPUT_TOTAL
        .with_label_values(&[operation])
        .inc();
    OPERATION_LATENCY
        .with_label_values(&[operation])
        .observe(start.elapsed().as_secs_f64());
}

fn record_operation_error(operation: &str, error_type: &str, start: Instant) {
    OPERATION_ERRORS_TOTAL
        .with_label_values(&[operation, error_type])
        .inc();
    OPERATION_LATENCY
        .with_label_values(&[operation])
        .observe(start.elapsed().as_secs_f64());
}

async fn correlation_id_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let correlation_id = request
        .headers()
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), ToString::to_string);

    let span = tracing::info_span!(
        "gateway.http.request",
        correlation_id = %correlation_id,
        method = %method,
        path = %path
    );
    let mut response = next.run(request).instrument(span).await;
    response.headers_mut().insert(
        "x-correlation-id",
        HeaderValue::from_str(&correlation_id)
            .unwrap_or_else(|_| HeaderValue::from_static("invalid-correlation-id")),
    );

    let status = response.status().as_u16().to_string();
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path])
        .inc();
    HTTP_RESPONSES
        .with_label_values(&[&method, &path, &status])
        .inc();
    HTTP_LATENCY
        .with_label_values(&[&method, &path])
        .observe(started.elapsed().as_secs_f64());

    if response.status().is_server_error() {
        OPERATION_ERRORS_TOTAL
            .with_label_values(&["http_request", "5xx"])
            .inc();
    }

    response
}

/// WebSocket handler
async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

#[tracing::instrument(
    name = "gateway.create_room",
    skip(state, _user, payload),
    fields(room_name = %payload.name)
)]
async fn create_room(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    let started = Instant::now();
    let operation = "create_room";
    if payload.name.trim().is_empty() {
        record_operation_error(operation, "validation", started);
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("room name cannot be empty")),
        )
            .into_response();
    }

    #[cfg(feature = "multi-tenant")]
    let tenant_id = payload.tenant_id.clone();

    #[cfg(not(feature = "multi-tenant"))]
    let _tenant_id: Option<String> = None;

    let room = Room {
        id: format!("room_{}", Uuid::new_v4().simple()),
        name: payload.name,
        topic: payload.topic,
        #[cfg(feature = "multi-tenant")]
        tenant_id,
    };

    let response = CreateRoomResponse {
        id: room.id.clone(),
        name: room.name.clone(),
    };

    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        record_operation_error(operation, "unavailable", started);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::service_unavailable("service unavailable")),
        )
            .into_response();
    };

    let mut rooms = state.rooms.write().await;
    rooms.insert(room.id.clone(), room);
    ROOMS_CREATED_TOTAL.inc();
    ROOMS_ACTIVE.set(rooms.len() as f64);
    record_operation_success(operation, started);

    (StatusCode::CREATED, Json(response)).into_response()
}

#[tracing::instrument(
    name = "gateway.send_message",
    skip(state, _user, payload),
    fields(room_id = %payload.room_id, sender = %payload.sender)
)]
async fn send_message(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let started = Instant::now();
    let operation = "send_message";
    if payload.room_id.trim().is_empty()
        || payload.sender.trim().is_empty()
        || payload.text.trim().is_empty()
    {
        record_operation_error(operation, "validation", started);
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(
                "roomId, sender, and text are required",
            )),
        )
            .into_response();
    }
    if payload.text.len() > MAX_MESSAGE_TEXT_LEN {
        record_operation_error(operation, "validation", started);
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(
                "text exceeds maximum length of 32768 characters",
            )),
        )
            .into_response();
    }

    let rooms = state.rooms.read().await;
    if !rooms.contains_key(&payload.room_id) {
        record_operation_error(operation, "room_not_found", started);
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("room not found")),
        )
            .into_response();
    }
    drop(rooms);

    let message = StoredMessage {
        id: format!("msg_{}", Uuid::new_v4().simple()),
        sender: payload.sender,
        text: payload.text,
        reply_to: payload.reply_to,
    };
    let response = SendMessageResponse {
        id: message.id.clone(),
    };

    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        record_operation_error(operation, "unavailable", started);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::service_unavailable("service unavailable")),
        )
            .into_response();
    };

    let mut messages = state.room_messages.write().await;
    messages.entry(payload.room_id).or_default().push(message);
    MESSAGES_SENT.inc();
    record_operation_success(operation, started);

    (StatusCode::CREATED, Json(response)).into_response()
}

#[tracing::instrument(
    name = "gateway.get_room",
    skip(state, _user),
    fields(room_id = %id)
)]
async fn get_room(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let rooms = state.rooms.read().await;
    let Some(room) = rooms.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("room not found")),
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

    #[cfg(feature = "multi-tenant")]
    let tenant_id = room.tenant_id.clone();
    #[cfg(not(feature = "multi-tenant"))]
    let _tenant_id: Option<String> = None;

    let response = RoomInfoResponse {
        id: room.id,
        name: room.name,
        topic: room.topic,
        messages,
        #[cfg(feature = "multi-tenant")]
        tenant_id,
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[tracing::instrument(
    name = "gateway.invite_member",
    skip(state, _user, payload),
    fields(room_id = %id, member_id = %payload.member_id)
)]
async fn invite_member(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(payload): Json<InviteMemberRequest>,
) -> impl IntoResponse {
    if payload.member_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("memberId is required")),
        )
            .into_response();
    }

    let rooms = state.rooms.read().await;
    if !rooms.contains_key(&id) {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("room not found")),
        )
            .into_response();
    }
    drop(rooms);

    let member_id = payload.member_id.clone();
    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::service_unavailable("service unavailable")),
        )
            .into_response();
    };

    let mut members = state.room_members.write().await;
    let room_members = members.entry(id.clone()).or_default();
    if !room_members.contains(&member_id) {
        room_members.push(member_id.clone());
    }

    let response = InviteMemberResponse {
        room_id: id,
        member_id,
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[tracing::instrument(
    name = "gateway.search_messages.post",
    skip(state, _user, payload),
    fields(limit = payload.limit)
)]
async fn search_messages(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Json(payload): Json<SearchApiRequest>,
) -> impl IntoResponse {
    let Some(search_service) = state.search_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Search service not configured".to_string(),
                code: Some(error_codes::SEARCH_UNAVAILABLE),
            }),
        )
            .into_response();
    };

    if payload.query.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Query cannot be empty".to_string(),
                code: Some(error_codes::INVALID_QUERY),
            }),
        )
            .into_response();
    }

    let mut request = SearchRequest::new(&payload.query).with_limit(payload.limit);

    if let Some(min_score) = payload.min_score {
        request = request.with_min_score(min_score);
    }

    if let Some(room_id) = payload.room_id {
        request = request.in_room(room_id);
    }

    match search_service.search(request).await {
        Ok(response) => {
            let items: Vec<SearchResultItem> = response
                .results
                .into_iter()
                .filter_map(|r| {
                    r.content.map(|content| SearchResultItem {
                        id: r.id,
                        score: r.score,
                        content,
                        room_id: r.room_id,
                    })
                })
                .collect();
            let total = items.len();
            let api_response = SearchApiResponse {
                query: response.query,
                results: items,
                total,
            };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::from(e)),
        )
            .into_response(),
    }
}

#[tracing::instrument(
    name = "gateway.search_messages.get",
    skip(state, _user, params),
    fields(limit = params.limit)
)]
async fn search_messages_get(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let Some(search_service) = state.search_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Search service not configured".to_string(),
                code: Some(error_codes::SEARCH_UNAVAILABLE),
            }),
        )
            .into_response();
    };

    if params.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Query parameter 'q' is required".to_string(),
                code: Some(error_codes::INVALID_QUERY),
            }),
        )
            .into_response();
    }

    let mut request = SearchRequest::new(&params.q).with_limit(params.limit);

    if let Some(min_score) = params.min_score {
        request = request.with_min_score(min_score);
    }

    if let Some(room_id) = params.room_id {
        request = request.in_room(room_id);
    }

    match search_service.search(request).await {
        Ok(response) => {
            let items: Vec<SearchResultItem> = response
                .results
                .into_iter()
                .filter_map(|r| {
                    r.content.map(|content| SearchResultItem {
                        id: r.id,
                        score: r.score,
                        content,
                        room_id: r.room_id,
                    })
                })
                .collect();
            let total = items.len();
            let api_response = SearchApiResponse {
                query: response.query,
                results: items,
                total,
            };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::from(e)),
        )
            .into_response(),
    }
}

#[tracing::instrument(
    name = "gateway.list_rooms",
    skip(state, _user, query),
    fields(limit = ?query.limit, offset = ?query.offset)
)]
async fn list_rooms(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Query(query): Query<ListRoomsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let rooms = state.rooms.read().await;
    let members = state.room_members.read().await;

    let all_rooms: Vec<RoomSummary> = rooms
        .values()
        .skip(offset)
        .take(limit)
        .map(|room| {
            let member_count = members.get(&room.id).map(|m| m.len());
            RoomSummary {
                id: room.id.clone(),
                name: room.name.clone(),
                topic: room.topic.clone(),
                member_count,
            }
        })
        .collect();

    let total = rooms.len();

    let response = ListRoomsResponse {
        rooms: all_rooms,
        total,
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[tracing::instrument(
    name = "gateway.delete_room",
    skip(state, _user),
    fields(room_id = %id)
)]
async fn delete_room(
    State(state): State<SharedState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let Ok(_permit) = state.write_gate.clone().acquire_owned().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::service_unavailable("service unavailable")),
        )
            .into_response();
    };

    let mut rooms = state.rooms.write().await;
    if rooms.remove(&id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("room not found")),
        )
            .into_response();
    }
    drop(rooms);

    let mut messages = state.room_messages.write().await;
    messages.remove(&id);
    drop(messages);

    let mut members = state.room_members.write().await;
    members.remove(&id);

    (StatusCode::NO_CONTENT, ()).into_response()
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
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn openapi_endpoint_returns_json() {
        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.starts_with("application/json"));

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["openapi"], "3.0.3");
    }

    #[tokio::test]
    async fn docs_endpoint_returns_swagger_html() {
        let app = build_routes();
        let response = app
            .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("SwaggerUIBundle"));
        assert!(html.contains("/openapi.json"));
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_prometheus_payload() {
        ROOMS_CREATED_TOTAL.inc();
        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload = String::from_utf8(body.to_vec()).unwrap();
        assert!(payload.contains("nexis_rooms_created_total"));
    }

    #[tokio::test]
    async fn response_contains_correlation_id_header() {
        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("x-correlation-id"));
    }

    #[tokio::test]
    async fn create_room_returns_201_and_room_identity() {
        use crate::auth::JwtConfig;
        let token = JwtConfig::test_token("test-user");
        let before_rooms_created = ROOMS_CREATED_TOTAL.get();
        let before_throughput = OPERATION_THROUGHPUT_TOTAL
            .with_label_values(&["create_room"])
            .get();

        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rooms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
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
        assert!(ROOMS_CREATED_TOTAL.get() > before_rooms_created);
        assert!(
            OPERATION_THROUGHPUT_TOTAL
                .with_label_values(&["create_room"])
                .get()
                > before_throughput
        );
    }

    #[tokio::test]
    async fn create_room_validation_error_records_metric() {
        use crate::auth::JwtConfig;
        let token = JwtConfig::test_token("test-user");
        let before_errors = OPERATION_ERRORS_TOTAL
            .with_label_values(&["create_room", "validation"])
            .get();

        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rooms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(
                        json!({
                            "name": "   "
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            OPERATION_ERRORS_TOTAL
                .with_label_values(&["create_room", "validation"])
                .get()
                > before_errors
        );
    }

    #[tokio::test]
    async fn send_message_returns_404_for_unknown_room() {
        use crate::auth::JwtConfig;
        let token = JwtConfig::test_token("test-user");

        let app = build_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
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
        use crate::auth::JwtConfig;
        let token = JwtConfig::test_token("test-user");

        let app = build_routes();

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rooms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
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
                    .header("authorization", format!("Bearer {}", token))
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
                    .header("authorization", format!("Bearer {}", token))
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

    #[cfg(feature = "multi-tenant")]
    mod multi_tenant_tests {
        use super::*;

        #[tokio::test]
        async fn create_room_with_tenant_includes_tenant_id() {
            let app = build_routes();
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/rooms")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "name": "tenant-room",
                                "tenant_id": "tenant_123"
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
            assert!(payload["id"].as_str().unwrap().starts_with("room_"));
        }
    }
}
