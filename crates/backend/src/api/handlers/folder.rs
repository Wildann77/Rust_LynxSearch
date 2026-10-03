use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

use crate::api::dtos::PathUuid;
use crate::api::extractors::ValidatedPath;
use crate::state::AppState;

pub async fn list_folders(_state: State<AppState>) -> impl IntoResponse {
    Json(json!([]))
}

pub async fn delete_folder(
    _state: State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> impl IntoResponse {
    Json(json!({
        "success": true,
        "folder_id": path.id,
        "deleted_documents": 0
    }))
}
