use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::api::dtos::PathUuid;
use crate::api::extractors::ValidatedPath;
use crate::domain::models::DocumentId;
use crate::error::AppError;
use crate::infrastructure::fs::LocalFileReader;
use crate::state::AppState;

pub async fn get_document(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> Result<impl IntoResponse, AppError> {
    let doc_id = DocumentId::from_uuid(path.id);
    let settings = state.get_settings().await;
    let file_reader = LocalFileReader::new(state.file_io_semaphore.clone());

    let detail = crate::application::queries::document::get_document_detail(
        &state.repositories,
        &file_reader,
        &settings,
        &doc_id,
    )
    .await?;

    Ok((StatusCode::OK, Json(detail)))
}

pub async fn delete_document(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let doc_id = crate::domain::models::DocumentId::from_uuid(path.id);
    let response =
        crate::application::commands::document::exclude_document(&state.repositories, &doc_id)
            .await?;
    Ok((axum::http::StatusCode::OK, Json(response)))
}
