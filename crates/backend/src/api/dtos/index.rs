use std::borrow::Cow;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

pub fn validate_relative_path(path: &str) -> Result<(), ValidationError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        let mut err = ValidationError::new("empty");
        err.message = Some(Cow::Borrowed("Path relatif tidak boleh kosong"));
        return Err(err);
    }

    if trimmed.contains('\0') {
        let mut err = ValidationError::new("null_byte");
        err.message = Some(Cow::Borrowed(
            "Path relatif tidak boleh mengandung null byte",
        ));
        return Err(err);
    }

    let p = Path::new(trimmed);
    if p.is_absolute() {
        let mut err = ValidationError::new("not_relative");
        err.message = Some(Cow::Borrowed(
            "Path relatif tidak boleh berupa path absolut",
        ));
        return Err(err);
    }

    for component in p.components() {
        if matches!(component, Component::ParentDir) {
            let mut err = ValidationError::new("path_traversal");
            err.message = Some(Cow::Borrowed(
                "Path relatif tidak boleh mengandung '..' (path traversal)",
            ));
            return Err(err);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct IndexDocumentRequestDto {
    pub folder_id: Uuid,

    #[validate(custom(function = "validate_relative_path"))]
    pub relative_path: String,
}

impl IndexDocumentRequestDto {
    pub fn new(folder_id: Uuid, relative_path: impl Into<String>) -> Self {
        Self {
            folder_id,
            relative_path: relative_path.into(),
        }
    }
}

use chrono::{DateTime, Utc};

use crate::application::orchestrator::JobProgressState;
use crate::domain::models::IndexingJob;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocumentResponseDto {
    pub document_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebuildIndexResponseDto {
    pub job_id: Uuid,
    pub target_index: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStatusResponseDto {
    pub job_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub status: String,
    pub processed_files: i32,
    pub indexed_files: i32,
    pub skipped_files: i32,
    pub failed_files: i32,
    pub total_files: i32,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<&JobProgressState> for JobStatusResponseDto {
    fn from(state: &JobProgressState) -> Self {
        Self {
            job_id: *state.job_id.as_uuid(),
            folder_id: state.folder_id.as_ref().map(|f| *f.as_uuid()),
            status: state.status.as_str().to_string(),
            processed_files: state.files_processed,
            indexed_files: state.files_indexed,
            skipped_files: state.files_skipped,
            failed_files: state.files_failed,
            total_files: state.files_total,
            error: state.error_summary.clone(),
            started_at: state.started_at,
            finished_at: state.completed_at,
        }
    }
}

impl From<JobProgressState> for JobStatusResponseDto {
    fn from(state: JobProgressState) -> Self {
        Self::from(&state)
    }
}

impl From<&IndexingJob> for JobStatusResponseDto {
    fn from(job: &IndexingJob) -> Self {
        Self {
            job_id: *job.id.as_uuid(),
            folder_id: job.folder_id.as_ref().map(|f| *f.as_uuid()),
            status: job.status.as_str().to_string(),
            processed_files: job.files_processed,
            indexed_files: job.files_indexed,
            skipped_files: job.files_skipped,
            failed_files: job.files_failed,
            total_files: job.files_total,
            error: job.error_summary.clone(),
            started_at: job.started_at,
            finished_at: job.completed_at,
        }
    }
}

impl From<IndexingJob> for JobStatusResponseDto {
    fn from(job: IndexingJob) -> Self {
        Self::from(&job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "src/main.rs");
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_empty_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "   ");
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_traversal_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "../secret.rs");
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_absolute_relative_path() {
        #[cfg(unix)]
        let path = "/etc/passwd";
        #[cfg(windows)]
        let path = "C:\\secret.rs";
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), path);
        assert!(dto.validate().is_err());
    }
}
