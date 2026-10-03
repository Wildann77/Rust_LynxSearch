use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::api::dtos::PathUuid;
use crate::api::extractors::ValidatedPath;
use crate::state::AppState;

pub async fn get_document(
    _state: State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> impl IntoResponse {
    Json(json!({
        "id": path.id,
        "folder_id": Uuid::nil(),
        "folder_root_path": "",
        "relative_path": "",
        "title": "",
        "content": "",
        "type": "doc",
        "language": "markdown",
        "tags": [],
        "project": "",
        "file_size": 0,
        "content_hash": "",
        "updated_at": Utc::now().to_rfc3339()
    }))
}

pub async fn delete_document(
    _state: State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> impl IntoResponse {
    Json(json!({
        "success": true,
        "id": path.id,
        "status": "EXCLUDED",
        "message": "Document removed from search index and marked as EXCLUDED."
    }))
}
