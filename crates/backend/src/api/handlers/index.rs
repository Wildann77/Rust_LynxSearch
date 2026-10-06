use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::api::dtos::{
    IndexDocumentRequestDto, IndexFolderResponseDto, JobStatusResponseDto, PathUuid,
    RebuildIndexResponseDto, RegisterFolderRequestDto,
};
use crate::api::extractors::{ValidatedJson, ValidatedPath};
use crate::domain::models::{Folder, FolderId, JobId};
use crate::error::AppError;
use crate::state::AppState;

pub async fn index_folder(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterFolderRequestDto>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Existing folder -> trigger rescan via folder_id
    if let Some(folder_id_uuid) = payload.folder_id {
        let folder_id = FolderId::from_uuid(folder_id_uuid);
        let folder = state
            .repositories
            .folder
            .get_folder(&folder_id)
            .await?
            .ok_or_else(|| AppError::FolderNotFound(folder_id_uuid))?;

        let job_id = state
            .orchestrator
            .submit_index_folder(folder.id, true)
            .await?;

        return Ok((
            StatusCode::ACCEPTED,
            Json(IndexFolderResponseDto {
                job_id: *job_id.as_uuid(),
                folder_id: *folder.id.as_uuid(),
                status: "RUNNING".to_string(),
                message: "Background scanning job created successfully.".to_string(),
            }),
        ));
    }

    // 2. New folder registration or duplicate root check
    if let Some(ref path_str) = payload.root_path {
        let path = std::path::PathBuf::from(path_str.trim());
        if let Some(existing) = state.repositories.folder.find_by_path(&path).await? {
            return Err(AppError::FolderConflict {
                path: path_str.clone(),
                id: *existing.id.as_uuid(),
            });
        }

        let new_folder = Folder::new(path);
        state.repositories.folder.create_folder(&new_folder).await?;

        let job_id = state
            .orchestrator
            .submit_index_folder(new_folder.id, false)
            .await?;

        return Ok((
            StatusCode::ACCEPTED,
            Json(IndexFolderResponseDto {
                job_id: *job_id.as_uuid(),
                folder_id: *new_folder.id.as_uuid(),
                status: "RUNNING".to_string(),
                message: "Background scanning job created successfully.".to_string(),
            }),
        ));
    }

    Err(AppError::ValidationFailed(
        "Either folder_id or root_path must be provided.".to_string(),
    ))
}

pub async fn index_document(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<IndexDocumentRequestDto>,
) -> Result<impl IntoResponse, AppError> {
    let folder_id = FolderId::from_uuid(payload.folder_id);
    let settings = state.get_settings().await;
    let file_reader =
        crate::infrastructure::fs::LocalFileReader::new(state.file_io_semaphore.clone());

    let response = crate::application::commands::document::index_or_restore_document(
        &state.repositories,
        &file_reader,
        &settings,
        &folder_id,
        &payload.relative_path,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_job(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> Result<impl IntoResponse, AppError> {
    let job_id = JobId::from_uuid(path.id);

    // 1. Hot path: query active in-memory job tracker
    if let Some(progress) = state.job_tracker.get_progress(&job_id) {
        return Ok((StatusCode::OK, Json(JobStatusResponseDto::from(progress))));
    }

    // 2. Cold path: query persistent storage
    if let Some(job) = state.repositories.job.get_job(&job_id).await? {
        return Ok((StatusCode::OK, Json(JobStatusResponseDto::from(job))));
    }

    // 3. Not found
    Err(AppError::JobNotFound(path.id))
}

pub async fn cancel_job(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> Result<impl IntoResponse, AppError> {
    let job_id = JobId::from_uuid(path.id);
    state.orchestrator.submit_cancel_job(job_id).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "job_id": path.id,
            "status": "CANCELLED",
            "message": "Job cancellation signal sent."
        })),
    ))
}

pub async fn rebuild_index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (job_id, target_index) = state.orchestrator.submit_rebuild_index().await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(RebuildIndexResponseDto {
            job_id: *job_id.as_uuid(),
            target_index,
            status: "RUNNING".to_string(),
            message: "Zero-downtime index rebuild initiated.".to_string(),
        }),
    ))
}
