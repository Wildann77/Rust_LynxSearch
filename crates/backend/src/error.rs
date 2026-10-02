use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Folder not found: {0}")]
    FolderNotFound(Uuid),

    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),

    #[error("Conflict: Indexing job is currently active for folder {0}")]
    JobConflict(Uuid),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Invalid search query: {0}")]
    InvalidQuery(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Search engine error: {0}")]
    SearchEngine(String),

    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal server error: {0}")]
    Internal(String),
}
