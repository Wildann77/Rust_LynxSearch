use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::api::dtos::{IndexDocumentRequestDto, PathUuid, RegisterFolderRequestDto};
use crate::api::extractors::{ValidatedJson, ValidatedPath};
use crate::state::AppState;

pub async fn index_folder(
    _state: State<AppState>,
    ValidatedJson(_payload): ValidatedJson<RegisterFolderRequestDto>,
) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "job_id": Uuid::new_v4(),
            "folder_id": Uuid::new_v4(),
            "status": "RUNNING",
            "message": "Background scanning job created successfully."
        })),
    )
}

pub async fn index_document(
    _state: State<AppState>,
    ValidatedJson(_payload): ValidatedJson<IndexDocumentRequestDto>,
) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "document_id": Uuid::new_v4(),
            "status": "INDEXED",
            "message": "Document indexed successfully."
        })),
    )
}

pub async fn get_job(
    _state: State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> impl IntoResponse {
    Json(json!({
        "job_id": path.id,
        "folder_id": null,
        "status": "RUNNING",
        "processed_files": 0,
        "skipped_files": 0,
        "failed_files": 0,
        "total_files": 0,
        "error": null,
        "started_at": Utc::now().to_rfc3339(),
        "finished_at": null
    }))
}

pub async fn cancel_job(
    _state: State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> impl IntoResponse {
    Json(json!({
        "job_id": path.id,
        "status": "CANCELLED",
        "message": "Job cancellation signal sent."
    }))
}

pub async fn rebuild_index(_state: State<AppState>) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "job_id": Uuid::new_v4(),
            "target_index": "lynx_documents_v2",
            "status": "RUNNING",
            "message": "Zero-downtime index rebuild initiated."
        })),
    )
}
