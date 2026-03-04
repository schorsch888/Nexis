//! Collaboration API routes for Phase 6 features.

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;

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
    _user: AuthenticatedUser,
    Json(payload): Json<CreateMeetingRoomRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_meeting::MeetingRoom> = None;

    let response = CreateMeetingRoomResponse {
        room_id: format!("meeting_{}", Uuid::new_v4().simple()),
        name: payload.name,
    };

    (StatusCode::CREATED, Json(response))
}

async fn join_meeting_room(
    _user: AuthenticatedUser,
    Path(room_id): Path<String>,
    Json(payload): Json<MeetingParticipantRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_meeting::Participant> = None;

    let response = MeetingParticipantResponse {
        room_id,
        user_id: payload.user_id,
    };

    (StatusCode::OK, Json(response))
}

async fn leave_meeting_room(
    _user: AuthenticatedUser,
    Path(room_id): Path<String>,
    Json(payload): Json<MeetingParticipantRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_meeting::Participant> = None;

    let response = MeetingParticipantResponse {
        room_id,
        user_id: payload.user_id,
    };

    (StatusCode::OK, Json(response))
}

async fn create_document(
    _user: AuthenticatedUser,
    Json(payload): Json<CreateDocumentRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_doc::Document> = None;

    let response = CreateDocumentResponse {
        document_id: format!("doc_{}", Uuid::new_v4().simple()),
        title: payload.title,
    };

    (StatusCode::CREATED, Json(response))
}

async fn sync_document(
    _user: AuthenticatedUser,
    Path(document_id): Path<String>,
    Json(payload): Json<SyncDocumentRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_doc::CRDTOperation> = None;

    let response = DocumentContentResponse {
        document_id,
        content: payload.content,
    };

    (StatusCode::OK, Json(response))
}

async fn get_document_content(
    _user: AuthenticatedUser,
    Path(document_id): Path<String>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_doc::DocSnapshot> = None;

    let response = DocumentContentResponse {
        document_id,
        content: String::new(),
    };

    (StatusCode::OK, Json(response))
}

async fn create_task(
    _user: AuthenticatedUser,
    Json(payload): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_task::Task> = None;

    let response = CreateTaskResponse {
        task_id: format!("task_{}", Uuid::new_v4().simple()),
        title: payload.title,
    };

    (StatusCode::CREATED, Json(response))
}

async fn assign_task(
    _user: AuthenticatedUser,
    Path(task_id): Path<String>,
    Json(payload): Json<AssignTaskRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_task::Assignment> = None;

    let response = TaskAssignmentResponse {
        task_id,
        assignee_id: payload.assignee_id,
    };

    (StatusCode::OK, Json(response))
}

async fn complete_task(_user: AuthenticatedUser, Path(task_id): Path<String>) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_task::TaskStatus> = None;

    let response = CompleteTaskResponse {
        task_id,
        status: "completed".to_string(),
    };

    (StatusCode::OK, Json(response))
}

async fn create_calendar_event(
    _user: AuthenticatedUser,
    Json(payload): Json<CreateCalendarEventRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_calendar::CalendarEvent> = None;

    let response = CreateCalendarEventResponse {
        event_id: format!("event_{}", Uuid::new_v4().simple()),
        title: payload.title,
    };

    let _time_range = nexis_calendar::TimeRange::new(payload.starts_at, payload.ends_at);

    (StatusCode::CREATED, Json(response))
}

async fn check_calendar_conflicts(
    _user: AuthenticatedUser,
    Json(payload): Json<ConflictCheckRequest>,
) -> impl IntoResponse {
    let _domain_type_marker: Option<nexis_calendar::Conflict> = None;

    let has_conflicts = payload.starts_at >= payload.ends_at;
    let response = ConflictCheckResponse { has_conflicts };

    (StatusCode::OK, Json(response))
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
}
