use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::api::dtos::{DeleteFolderResponseDto, FolderResponseDto, PathUuid};
use crate::api::extractors::ValidatedPath;
use crate::domain::models::FolderId;
use crate::error::AppError;
use crate::state::AppState;

pub async fn list_folders(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let folders_with_counts = state.repositories.folder.list_folders_with_counts().await?;
    let dtos: Vec<FolderResponseDto> = folders_with_counts
        .into_iter()
        .map(|(folder, count)| FolderResponseDto {
            id: *folder.id.as_uuid(),
            root_path: folder.path.to_string_lossy().to_string(),
            status: folder.status,
            document_count: count,
            last_scanned_at: folder.last_scanned_at,
            created_at: folder.created_at,
        })
        .collect();

    Ok(Json(dtos))
}

pub async fn delete_folder(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<PathUuid>,
) -> Result<impl IntoResponse, AppError> {
    let folder_id = FolderId::from_uuid(path.id);

    // 1. Verify folder exists in database
    let _folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await?
        .ok_or_else(|| AppError::FolderNotFound(path.id))?;

    // 2. Reject deletion if folder is actively locked / scanning
    if state.job_tracker.is_folder_locked(&folder_id) {
        return Err(AppError::JobConflict(path.id));
    }

    // 3. Delete corresponding Elasticsearch documents
    let deleted_documents = state
        .repositories
        .search
        .delete_documents_by_folder(&folder_id)
        .await?;

    // 4. Delete folder from database (PostgreSQL FK cascades delete child rows)
    state.repositories.folder.delete_folder(&folder_id).await?;

    Ok(Json(DeleteFolderResponseDto {
        success: true,
        folder_id: path.id,
        deleted_documents,
    }))
}
