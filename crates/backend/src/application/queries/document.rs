use std::path::Path;

use crate::api::dtos::DocumentDetailResponseDto;
use crate::domain::models::{AppSettings, DocumentId};
use crate::domain::ports::{FileReader, ReadOptions};
use crate::domain::services::{DocumentExtractor, ExtractOptions, ExtractionResult};
use crate::error::AppError;
use crate::state::Repositories;

/// Fetches full document content safely from disk along with metadata from the database.
pub async fn get_document_detail(
    repositories: &Repositories,
    file_reader: &dyn FileReader,
    settings: &AppSettings,
    doc_id: &DocumentId,
) -> Result<DocumentDetailResponseDto, AppError> {
    // 1. Get registry entry
    let entry = repositories
        .registry
        .get_entry(doc_id)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound(doc_id.into_inner()))?;

    // 2. Get folder
    let folder = repositories
        .folder
        .get_folder(&entry.folder_id)
        .await?
        .ok_or_else(|| AppError::FolderNotFound(*entry.folder_id.as_uuid()))?;

    // 3. Path traversal defense
    let rel_path_str = &entry.relative_path;
    if rel_path_str.contains("..") || rel_path_str.contains('\0') {
        return Err(AppError::PathTraversal(
            "Relative path contains forbidden traversal characters".into(),
        ));
    }

    let abs_path = folder.path.join(rel_path_str);

    // Canonicalize paths when possible to verify containment
    if let (Ok(canonical_root), Ok(canonical_doc)) =
        (folder.path.canonicalize(), abs_path.canonicalize())
    {
        if !canonical_doc.starts_with(&canonical_root) {
            return Err(AppError::PathTraversal(
                "Document path is outside registered folder boundary".into(),
            ));
        }
    } else if !abs_path.starts_with(&folder.path) {
        return Err(AppError::PathTraversal(
            "Document path is outside registered folder boundary".into(),
        ));
    }

    if !abs_path.is_file() {
        return Err(AppError::DocumentNotFound(doc_id.into_inner()));
    }

    // 4. Read file content via file reader
    let read_options = ReadOptions::new(Some(settings.max_file_size_bytes));
    let payload = file_reader.read_file(&abs_path, &read_options).await?;

    // 5. Extract document metadata and text content
    let extractor = DocumentExtractor::new();
    let extract_options = ExtractOptions {
        max_file_size_bytes: Some(settings.max_file_size_bytes),
    };

    let extraction_result = extractor.extract(
        Path::new(rel_path_str),
        &payload.bytes,
        payload.size,
        payload.modified_at,
        &extract_options,
    );

    let (title, content, doc_type, language, tags, project) = match extraction_result {
        ExtractionResult::Extracted(extracted) => (
            extracted.title,
            extracted.content,
            extracted.doc_type.as_str().to_string(),
            extracted.language.map(|l| l.as_str().to_string()),
            extracted.tags,
            extracted.project,
        ),
        ExtractionResult::Skipped(reason) => {
            let lossy_text = String::from_utf8_lossy(&payload.bytes).into_owned();
            let fallback_title = Path::new(rel_path_str)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| rel_path_str.clone());
            (
                fallback_title,
                if lossy_text.trim().is_empty() {
                    format!("[Content skipped: {reason:?}]")
                } else {
                    lossy_text
                },
                "doc".to_string(),
                None,
                vec![],
                None,
            )
        }
    };

    Ok(DocumentDetailResponseDto {
        id: *entry.id.as_uuid(),
        folder_id: *entry.folder_id.as_uuid(),
        folder_root_path: folder.path.to_string_lossy().into_owned(),
        relative_path: entry.relative_path,
        title,
        content,
        r#type: doc_type,
        language,
        tags,
        project,
        file_size: entry.file_size,
        content_hash: entry.content_hash,
        updated_at: Some(entry.updated_at.to_rfc3339()),
    })
}
