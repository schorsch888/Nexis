//! Collaboration API routes for Phase 6 features.

use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;

const MAX_NAME_LEN: usize = 128;
const MAX_TITLE_LEN: usize = 200;
const MAX_IDENTIFIER_LEN: usize = 128;
const MAX_CONTENT_LEN: usize = 100_000;
const MAX_RATE_LIMIT_SUBJECT_LEN: usize = 64;
const API_VERSION_HEADER: &str = "x-api-version";
const SUPPORTED_API_VERSION: &str = "1";

#[derive(Debug, Clone, Deserialize)]
struct CreateMeetingRoomRequest {
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateMeetingRoomResponse {
    room_id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MeetingParticipantRequest {
    user_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct MeetingParticipantResponse {
    room_id: String,
    user_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateDocumentRequest {
    title: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateDocumentResponse {
    document_id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SyncDocumentRequest {
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct DocumentContentResponse {
    document_id: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateTaskRequest {
    title: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateTaskResponse {
    task_id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssignTaskRequest {
    assignee_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct TaskAssignmentResponse {
    task_id: String,
    assignee_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct CompleteTaskResponse {
    task_id: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateCalendarEventRequest {
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateCalendarEventResponse {
    event_id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ConflictCheckRequest {
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct ConflictCheckResponse {
    has_conflicts: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CollaborationErrorResponse {
    error: String,
    code: &'static str,
}

#[derive(Debug, Clone)]
enum CollaborationError {
    BadRequest(String),
    Unauthorized,
    InvalidApiVersion,
}

impl IntoResponse for CollaborationError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(CollaborationErrorResponse {
                    error: message,
                    code: "BAD_REQUEST",
                }),
            )
                .into_response(),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(CollaborationErrorResponse {
                    error: "Missing or invalid authorization token".to_string(),
                    code: "UNAUTHORIZED",
                }),
            )
                .into_response(),
            Self::InvalidApiVersion => (
                StatusCode::BAD_REQUEST,
                Json(CollaborationErrorResponse {
                    error: "X-API-Version header is required and must be set to '1'".to_string(),
                    code: "INVALID_API_VERSION",
                }),
            )
                .into_response(),
        }
    }
}

struct CollaborationRequestContext {
    _user: AuthenticatedUser,
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for CollaborationRequestContext
where
    S: Send + Sync,
{
    type Rejection = CollaborationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state)
            .await
            .map_err(|_| CollaborationError::Unauthorized)?;

        let version_header = parts
            .headers
            .get(API_VERSION_HEADER)
            .and_then(|value| value.to_str().ok());
        if version_header != Some(SUPPORTED_API_VERSION) {
            return Err(CollaborationError::InvalidApiVersion);
        }

        Ok(Self { _user: user })
    }
}

/// Fixed-window rate-limit policy for collaboration operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollaborationRateLimitPolicy {
    pub max_requests: u32,
    pub window_seconds: u64,
}

impl CollaborationRateLimitPolicy {
    /// Create a new rate-limit policy.
    pub fn new(max_requests: u32, window_seconds: u64) -> Result<Self, String> {
        if max_requests == 0 {
            return Err("max_requests must be greater than 0".to_string());
        }
        if window_seconds == 0 {
            return Err("window_seconds must be greater than 0".to_string());
        }

        Ok(Self {
            max_requests,
            window_seconds,
        })
    }

    /// Return true when `request_count` exceeds the configured maximum.
    pub const fn is_exceeded(self, request_count: u32) -> bool {
        request_count > self.max_requests
    }
}

/// Scope key used by collaboration endpoint throttling.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationRateLimitScope {
    Meetings,
    Documents,
    Tasks,
    Calendar,
}

/// Identity used for per-subject collaboration rate limiting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollaborationRateLimitKey {
    pub scope: CollaborationRateLimitScope,
    pub subject: String,
}

impl CollaborationRateLimitKey {
    /// Build a validated scope+subject key.
    pub fn new(
        scope: CollaborationRateLimitScope,
        subject: impl Into<String>,
    ) -> Result<Self, String> {
        let subject = validate_identifier("subject", &subject.into(), MAX_RATE_LIMIT_SUBJECT_LEN)?;
        Ok(Self { scope, subject })
    }
}

fn bad_request_response(message: impl Into<String>) -> Response {
    CollaborationError::BadRequest(message.into()).into_response()
}

fn validate_required_text(field: &str, value: &str, max_len: usize) -> Result<String, Response> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(bad_request_response(format!("{field} is required")));
    }
    if trimmed.len() > max_len {
        return Err(bad_request_response(format!(
            "{field} exceeds maximum length of {max_len} characters"
        )));
    }

    Ok(trimmed.to_string())
}

fn validate_identifier(field: &str, value: &str, max_len: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} is required"));
    }
    if trimmed.len() > max_len {
        return Err(format!(
            "{field} exceeds maximum length of {max_len} characters"
        ));
    }
    if !trimmed
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(format!(
            "{field} contains invalid characters; allowed: a-z, A-Z, 0-9, _ and -"
        ));
    }

    Ok(trimmed.to_string())
}

fn validate_path_id(field: &str, value: &str) -> Result<String, Response> {
    validate_identifier(field, value, MAX_IDENTIFIER_LEN).map_err(bad_request_response)
}

fn validate_time_window(starts_at: DateTime<Utc>, ends_at: DateTime<Utc>) -> Result<(), Response> {
    if starts_at >= ends_at {
        return Err(bad_request_response(
            "starts_at must be earlier than ends_at",
        ));
    }

    Ok(())
}

/// Build collaboration routes for meeting, document, task, and calendar features.
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/v1/collaboration/meetings/rooms",
            post(create_meeting_room),
        )
        .route(
            "/v1/collaboration/meetings/rooms/:room_id/join",
            post(join_meeting_room),
        )
        .route(
            "/v1/collaboration/meetings/rooms/:room_id/leave",
            post(leave_meeting_room),
        )
        .route("/v1/collaboration/documents", post(create_document))
        .route(
            "/v1/collaboration/documents/:document_id/sync",
            post(sync_document),
        )
        .route(
            "/v1/collaboration/documents/:document_id/content",
            get(get_document_content),
        )
        .route("/v1/collaboration/tasks", post(create_task))
        .route("/v1/collaboration/tasks/:task_id/assign", post(assign_task))
        .route(
            "/v1/collaboration/tasks/:task_id/complete",
            post(complete_task),
        )
        .route(
            "/v1/collaboration/calendar/events",
            post(create_calendar_event),
        )
        .route(
            "/v1/collaboration/calendar/conflicts",
            post(check_calendar_conflicts),
        )
}

async fn create_meeting_room(
    _ctx: CollaborationRequestContext,
    Json(payload): Json<CreateMeetingRoomRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_meeting::MeetingRoom> = None;
    let name = match validate_required_text("name", &payload.name, MAX_NAME_LEN) {
        Ok(name) => name,
        Err(response) => return response,
    };

    let response = CreateMeetingRoomResponse {
        room_id: format!("meeting_{}", Uuid::new_v4().simple()),
        name,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn join_meeting_room(
    _ctx: CollaborationRequestContext,
    Path(room_id): Path<String>,
    Json(payload): Json<MeetingParticipantRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_meeting::Participant> = None;
    let room_id = match validate_path_id("room_id", &room_id) {
        Ok(room_id) => room_id,
        Err(response) => return response,
    };
    let user_id = match validate_identifier("user_id", &payload.user_id, MAX_IDENTIFIER_LEN) {
        Ok(user_id) => user_id,
        Err(message) => return bad_request_response(message),
    };

    let response = MeetingParticipantResponse { room_id, user_id };

    (StatusCode::OK, Json(response)).into_response()
}

async fn leave_meeting_room(
    _ctx: CollaborationRequestContext,
    Path(room_id): Path<String>,
    Json(payload): Json<MeetingParticipantRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_meeting::Participant> = None;
    let room_id = match validate_path_id("room_id", &room_id) {
        Ok(room_id) => room_id,
        Err(response) => return response,
    };
    let user_id = match validate_identifier("user_id", &payload.user_id, MAX_IDENTIFIER_LEN) {
        Ok(user_id) => user_id,
        Err(message) => return bad_request_response(message),
    };

    let response = MeetingParticipantResponse { room_id, user_id };

    (StatusCode::OK, Json(response)).into_response()
}

async fn create_document(
    _ctx: CollaborationRequestContext,
    Json(payload): Json<CreateDocumentRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_doc::Document> = None;
    let title = match validate_required_text("title", &payload.title, MAX_TITLE_LEN) {
        Ok(title) => title,
        Err(response) => return response,
    };

    let response = CreateDocumentResponse {
        document_id: format!("doc_{}", Uuid::new_v4().simple()),
        title,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn sync_document(
    _ctx: CollaborationRequestContext,
    Path(document_id): Path<String>,
    Json(payload): Json<SyncDocumentRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_doc::CRDTOperation> = None;
    let document_id = match validate_path_id("document_id", &document_id) {
        Ok(document_id) => document_id,
        Err(response) => return response,
    };
    let content = match validate_required_text("content", &payload.content, MAX_CONTENT_LEN) {
        Ok(content) => content,
        Err(response) => return response,
    };

    let response = DocumentContentResponse {
        document_id,
        content,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn get_document_content(
    _ctx: CollaborationRequestContext,
    Path(document_id): Path<String>,
) -> Response {
    let _domain_type_marker: Option<nexis_doc::DocSnapshot> = None;
    let document_id = match validate_path_id("document_id", &document_id) {
        Ok(document_id) => document_id,
        Err(response) => return response,
    };

    let response = DocumentContentResponse {
        document_id,
        content: String::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn create_task(
    _ctx: CollaborationRequestContext,
    Json(payload): Json<CreateTaskRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_task::Task> = None;
    let title = match validate_required_text("title", &payload.title, MAX_TITLE_LEN) {
        Ok(title) => title,
        Err(response) => return response,
    };

    let response = CreateTaskResponse {
        task_id: format!("task_{}", Uuid::new_v4().simple()),
        title,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn assign_task(
    _ctx: CollaborationRequestContext,
    Path(task_id): Path<String>,
    Json(payload): Json<AssignTaskRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_task::Assignment> = None;
    let task_id = match validate_path_id("task_id", &task_id) {
        Ok(task_id) => task_id,
        Err(response) => return response,
    };
    let assignee_id =
        match validate_identifier("assignee_id", &payload.assignee_id, MAX_IDENTIFIER_LEN) {
            Ok(assignee_id) => assignee_id,
            Err(message) => return bad_request_response(message),
        };

    let response = TaskAssignmentResponse {
        task_id,
        assignee_id,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn complete_task(_ctx: CollaborationRequestContext, Path(task_id): Path<String>) -> Response {
    let _domain_type_marker: Option<nexis_task::TaskStatus> = None;
    let task_id = match validate_path_id("task_id", &task_id) {
        Ok(task_id) => task_id,
        Err(response) => return response,
    };

    let response = CompleteTaskResponse {
        task_id,
        status: "completed".to_string(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn create_calendar_event(
    _ctx: CollaborationRequestContext,
    Json(payload): Json<CreateCalendarEventRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_calendar::CalendarEvent> = None;
    let title = match validate_required_text("title", &payload.title, MAX_TITLE_LEN) {
        Ok(title) => title,
        Err(response) => return response,
    };
    if let Err(response) = validate_time_window(payload.starts_at, payload.ends_at) {
        return response;
    }
    let time_range = nexis_calendar::TimeRange::new(payload.starts_at, payload.ends_at);

    let response = CreateCalendarEventResponse {
        event_id: format!("event_{}", Uuid::new_v4().simple()),
        title,
    };

    let _time_range = time_range;

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn check_calendar_conflicts(
    _ctx: CollaborationRequestContext,
    Json(payload): Json<ConflictCheckRequest>,
) -> Response {
    let _domain_type_marker: Option<nexis_calendar::Conflict> = None;
    if let Err(response) = validate_time_window(payload.starts_at, payload.ends_at) {
        return response;
    }

    let has_conflicts = false;
    let response = ConflictCheckResponse { has_conflicts };

    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::auth::JwtConfig;

    #[tokio::test]
    async fn create_meeting_room_route_returns_created() {
        let token = JwtConfig::test_token("collab-user");
        let app = routes::<()>();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/collaboration/meetings/rooms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .header("x-api-version", "1")
                    .body(Body::from(
                        json!({
                            "name": "daily-sync"
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
        assert!(payload["room_id"].as_str().unwrap().starts_with("meeting_"));
    }

    #[tokio::test]
    async fn calendar_conflict_check_route_returns_ok() {
        let token = JwtConfig::test_token("collab-user");
        let app = routes::<()>();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/collaboration/calendar/conflicts")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .header("x-api-version", "1")
                    .body(Body::from(
                        json!({
                            "starts_at": "2026-03-04T09:00:00Z",
                            "ends_at": "2026-03-04T10:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_meeting_room_rejects_blank_name() {
        let token = JwtConfig::test_token("collab-user");
        let app = routes::<()>();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/collaboration/meetings/rooms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .header("x-api-version", "1")
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
    }

    #[tokio::test]
    async fn join_meeting_room_rejects_invalid_room_id() {
        let token = JwtConfig::test_token("collab-user");
        let app = routes::<()>();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/collaboration/meetings/rooms/room$bad/join")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .header("x-api-version", "1")
                    .body(Body::from(
                        json!({
                            "user_id": "user-123"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_calendar_event_rejects_invalid_time_range() {
        let token = JwtConfig::test_token("collab-user");
        let app = routes::<()>();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/collaboration/calendar/events")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .header("x-api-version", "1")
                    .body(Body::from(
                        json!({
                            "title": "Design review",
                            "starts_at": "2026-03-04T11:00:00Z",
                            "ends_at": "2026-03-04T10:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn rate_limit_policy_detects_exceeded_requests() {
        let policy = CollaborationRateLimitPolicy::new(100, 60).unwrap();

        assert!(!policy.is_exceeded(100));
        assert!(policy.is_exceeded(101));
    }

    #[test]
    fn rate_limit_key_rejects_invalid_subject() {
        let key =
            CollaborationRateLimitKey::new(CollaborationRateLimitScope::Documents, "user$invalid");

        assert!(key.is_err());
    }
}
