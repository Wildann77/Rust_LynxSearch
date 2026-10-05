use chrono::Utc;

use crate::api::dtos::{DeleteDocumentResponseDto, IndexDocumentResponseDto};
use crate::domain::models::{AppSettings, DocumentId, DocumentStatus, FolderId, RegistryEntry};
use crate::domain::ports::file_system::{FileReader, ReadOptions};
use crate::domain::services::document_extractor::{
    DocumentExtractor, ExtractOptions, ExtractionResult,
};
use crate::domain::services::scan_planner::normalize_path_str;
use crate::error::AppError;
use crate::state::Repositories;

/// Exclude a single document from search index and mark its registry status as EXCLUDED tombstone.
pub async fn exclude_document(
    repositories: &Repositories,
    doc_id: &DocumentId,
) -> Result<DeleteDocumentResponseDto, AppError> {
    let mut entry = repositories
        .registry
        .get_entry(doc_id)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound(doc_id.into_inner()))?;

    // 1. Delete document from Elasticsearch index (idempotent 404 is ignored by search repo)
    repositories.search.delete_document(doc_id).await?;

    // 2. Persist tombstone state in PostgreSQL registry
    entry.status = DocumentStatus::Excluded;
    entry.status_reason = Some("Manually excluded by user".to_string());
    entry.updated_at = Utc::now();
    repositories.registry.upsert_entry(&entry).await?;

    Ok(DeleteDocumentResponseDto {
        success: true,
        id: doc_id.into_inner(),
        status: "EXCLUDED".to_string(),
        message: "Document removed from search index and marked as EXCLUDED.".to_string(),
    })
}

/// Index a new document or un-exclude/restore a previously EXCLUDED document.
pub async fn index_or_restore_document(
    repositories: &Repositories,
    file_reader: &dyn FileReader,
    settings: &AppSettings,
    folder_id: &FolderId,
    relative_path: &str,
) -> Result<IndexDocumentResponseDto, AppError> {
    let norm_rel_path = normalize_path_str(relative_path);

    // 1. Verify registered folder exists
    let folder = repositories
        .folder
        .get_folder(folder_id)
        .await?
        .ok_or_else(|| AppError::FolderNotFound(folder_id.into_inner()))?;

    // 2. Verify file path boundary and existence on disk
    let abs_path = folder.path.join(&norm_rel_path);
    if !abs_path.starts_with(&folder.path) {
        return Err(AppError::PathTraversal(
            "Access to path outside allowed folder is forbidden.".to_string(),
        ));
    }

    if !abs_path.is_file() {
        return Err(AppError::ValidationFailed(format!(
            "Target file does not exist on disk: {norm_rel_path}"
        )));
    }

    let doc_id = DocumentId::from_relative_path(*folder_id, &norm_rel_path);

    // 3. Read file content safely
    let read_options = ReadOptions::new(Some(settings.max_file_size_bytes));
    let payload = file_reader.read_file(&abs_path, &read_options).await?;

    // 4. Extract content
    let extractor = DocumentExtractor::new();
    let extract_options = ExtractOptions {
        max_file_size_bytes: Some(settings.max_file_size_bytes),
    };
    let extraction_result = extractor.extract(
        std::path::Path::new(&norm_rel_path),
        &payload.bytes,
        payload.size,
        payload.modified_at,
        &extract_options,
    );

    let now = Utc::now();

    // 5. Update index & registry according to extraction outcome
    match extraction_result {
        ExtractionResult::Extracted(extracted) => {
            let indexed_doc = extracted.into_indexed_doc(
                doc_id,
                *folder_id,
                norm_rel_path.clone(),
                abs_path.to_string_lossy().into_owned(),
                Some(payload.hash.clone()),
                now,
            );

            repositories.search.index_document(&indexed_doc).await?;

            let entry = RegistryEntry {
                id: doc_id,
                folder_id: *folder_id,
                relative_path: norm_rel_path,
                content_hash: payload.hash,
                file_size: payload.size as i64,
                status: DocumentStatus::Indexed,
                status_reason: None,
                indexed_at: now,
                updated_at: payload.modified_at.unwrap_or(now),
            };

            repositories.registry.upsert_entry(&entry).await?;

            Ok(IndexDocumentResponseDto {
                document_id: *doc_id.as_uuid(),
                status: "INDEXED".to_string(),
                message: "Document indexed successfully.".to_string(),
            })
        }
        ExtractionResult::Skipped(reason) => {
            let reason_str = format!("{reason:?}");
            let entry = RegistryEntry {
                id: doc_id,
                folder_id: *folder_id,
                relative_path: norm_rel_path,
                content_hash: payload.hash,
                file_size: payload.size as i64,
                status: DocumentStatus::Skipped,
                status_reason: Some(reason_str.clone()),
                indexed_at: now,
                updated_at: payload.modified_at.unwrap_or(now),
            };

            repositories.registry.upsert_entry(&entry).await?;
            let _ = repositories.search.delete_document(&doc_id).await;

            Ok(IndexDocumentResponseDto {
                document_id: *doc_id.as_uuid(),
                status: "SKIPPED".to_string(),
                message: format!("Document skipped: {reason_str}"),
            })
        }
    }
}
