use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// API error codes terbekukan sesuai spesifikasi ARCHITECTURE.md & AGENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    FolderNotFound,
    DocumentNotFound,
    JobConflict,
    ValidationFailed,
    PathTraversalDetected,
    InvalidQuery,
    DatabaseUnavailable,
    SearchEngineUnavailable,
    IoError,
    InternalServerError,
    RequestTimeout,
}

impl ErrorCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FolderNotFound => "FOLDER_NOT_FOUND",
            Self::DocumentNotFound => "DOCUMENT_NOT_FOUND",
            Self::JobConflict => "JOB_CONFLICT",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::PathTraversalDetected => "PATH_TRAVERSAL_DETECTED",
            Self::InvalidQuery => "INVALID_QUERY",
            Self::DatabaseUnavailable => "DATABASE_UNAVAILABLE",
            Self::SearchEngineUnavailable => "SEARCH_ENGINE_UNAVAILABLE",
            Self::IoError => "IO_ERROR",
            Self::InternalServerError => "INTERNAL_SERVER_ERROR",
            Self::RequestTimeout => "REQUEST_TIMEOUT",
        }
    }
}

impl AsRef<str> for ErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Format seragam respons error HTTP untuk desktop client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }
}

/// Taksonomi error terpusat backend LynxSearch.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("Folder not found: {0}")]
    FolderNotFound(Uuid),

    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),

    #[error("Conflict: Indexing job is currently active for folder {0}")]
    JobConflict(Uuid),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Validation failed: {message}")]
    ValidationErrors {
        message: String,
        details: serde_json::Value,
    },

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

    #[error("Request timed out: {0}")]
    RequestTimeout(String),
}

impl AppError {
    /// Pemetaan HTTP status code per kategori error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::FolderNotFound(_) | Self::DocumentNotFound(_) => StatusCode::NOT_FOUND,
            Self::JobConflict(_) => StatusCode::CONFLICT,
            Self::ValidationFailed(_) | Self::ValidationErrors { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::PathTraversal(_) => StatusCode::FORBIDDEN,
            Self::InvalidQuery(_) => StatusCode::BAD_REQUEST,
            Self::Database(_) | Self::SearchEngine(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            Self::Io(_) | Self::Internal(_) | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Pemetaan API error code terbekukan.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::FolderNotFound(_) => ErrorCode::FolderNotFound,
            Self::DocumentNotFound(_) => ErrorCode::DocumentNotFound,
            Self::JobConflict(_) => ErrorCode::JobConflict,
            Self::ValidationFailed(_) | Self::ValidationErrors { .. } => {
                ErrorCode::ValidationFailed
            }
            Self::PathTraversal(_) => ErrorCode::PathTraversalDetected,
            Self::InvalidQuery(_) => ErrorCode::InvalidQuery,
            Self::Database(_) => ErrorCode::DatabaseUnavailable,
            Self::SearchEngine(_) => ErrorCode::SearchEngineUnavailable,
            Self::RequestTimeout(_) => ErrorCode::RequestTimeout,
            Self::Io(_) => ErrorCode::IoError,
            Self::Internal(_) | Self::Config(_) => ErrorCode::InternalServerError,
        }
    }

    /// Pesan ramah klien tanpa membocorkan internal detail/stack trace.
    pub fn client_message(&self) -> String {
        match self {
            Self::FolderNotFound(id) => format!("Folder with id '{id}' was not found."),
            Self::DocumentNotFound(id) => format!("Document with id '{id}' was not found."),
            Self::JobConflict(id) => {
                format!("Indexing job is currently active for folder '{id}'.")
            }
            Self::ValidationFailed(msg) => format!("Validation failed: {msg}"),
            Self::ValidationErrors { message, .. } => message.clone(),
            Self::PathTraversal(_) => {
                "Access to path outside allowed folder is forbidden.".to_string()
            }
            Self::InvalidQuery(msg) => format!("Invalid search query: {msg}"),
            Self::Database(_) => "Database service unavailable.".to_string(),
            Self::SearchEngine(_) => "Search engine service unavailable.".to_string(),
            Self::RequestTimeout(msg) => {
                if msg.is_empty() {
                    "Request execution exceeded timeout limit.".to_string()
                } else {
                    msg.clone()
                }
            }
            Self::Io(_) => "Internal file system error occurred.".to_string(),
            Self::Internal(_) | Self::Config(_) => "An internal server error occurred.".to_string(),
        }
    }

    /// Ekstraksi rincian opsional terstruktur untuk klien.
    pub fn client_details(&self) -> Option<serde_json::Value> {
        match self {
            Self::ValidationErrors { details, .. } => Some(details.clone()),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();

        // Simpan detail teknis pada tracing log tanpa membocorkan ke UI
        if status.is_server_error() {
            tracing::error!(
                error_code = code.as_str(),
                status = status.as_u16(),
                error = %self,
                "Internal server error in API handler"
            );
        } else {
            tracing::warn!(
                error_code = code.as_str(),
                status = status.as_u16(),
                error = %self,
                "Client error in API request"
            );
        }

        let body = ErrorResponse {
            code,
            message: self.client_message(),
            details: self.client_details(),
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn test_error_code_strings_and_serialization() {
        let cases = [
            (ErrorCode::FolderNotFound, "FOLDER_NOT_FOUND"),
            (ErrorCode::DocumentNotFound, "DOCUMENT_NOT_FOUND"),
            (ErrorCode::JobConflict, "JOB_CONFLICT"),
            (ErrorCode::ValidationFailed, "VALIDATION_FAILED"),
            (ErrorCode::PathTraversalDetected, "PATH_TRAVERSAL_DETECTED"),
            (ErrorCode::InvalidQuery, "INVALID_QUERY"),
            (ErrorCode::DatabaseUnavailable, "DATABASE_UNAVAILABLE"),
            (
                ErrorCode::SearchEngineUnavailable,
                "SEARCH_ENGINE_UNAVAILABLE",
            ),
            (ErrorCode::IoError, "IO_ERROR"),
            (ErrorCode::InternalServerError, "INTERNAL_SERVER_ERROR"),
            (ErrorCode::RequestTimeout, "REQUEST_TIMEOUT"),
        ];

        for (code, expected_str) in cases {
            assert_eq!(code.as_str(), expected_str);
            assert_eq!(code.to_string(), expected_str);
            assert_eq!(code.as_ref(), expected_str);

            let serialized = serde_json::to_string(&code).expect("Serialize ErrorCode failed");
            assert_eq!(serialized, format!("\"{expected_str}\""));

            let deserialized: ErrorCode =
                serde_json::from_str(&serialized).expect("Deserialize ErrorCode failed");
            assert_eq!(deserialized, code);
        }
    }

    #[test]
    fn test_error_status_and_code_mappings() {
        let folder_id = Uuid::new_v4();
        let doc_id = Uuid::new_v4();

        let not_found_folder = AppError::FolderNotFound(folder_id);
        assert_eq!(not_found_folder.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(not_found_folder.error_code(), ErrorCode::FolderNotFound);
        assert!(
            not_found_folder
                .client_message()
                .contains(&folder_id.to_string())
        );

        let not_found_doc = AppError::DocumentNotFound(doc_id);
        assert_eq!(not_found_doc.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(not_found_doc.error_code(), ErrorCode::DocumentNotFound);
        assert!(not_found_doc.client_message().contains(&doc_id.to_string()));

        let conflict = AppError::JobConflict(folder_id);
        assert_eq!(conflict.status_code(), StatusCode::CONFLICT);
        assert_eq!(conflict.error_code(), ErrorCode::JobConflict);

        let val_err = AppError::ValidationFailed("page must be >= 1".into());
        assert_eq!(val_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(val_err.error_code(), ErrorCode::ValidationFailed);

        let path_err = AppError::PathTraversal("../../../etc/passwd".into());
        assert_eq!(path_err.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(path_err.error_code(), ErrorCode::PathTraversalDetected);
        assert!(!path_err.client_message().contains("etc/passwd"));

        let query_err = AppError::InvalidQuery("unbalanced quotes".into());
        assert_eq!(query_err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(query_err.error_code(), ErrorCode::InvalidQuery);

        let es_err = AppError::SearchEngine("Connection refused: 9200".into());
        assert_eq!(es_err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(es_err.error_code(), ErrorCode::SearchEngineUnavailable);
        assert_eq!(
            es_err.client_message(),
            "Search engine service unavailable."
        );

        let io_err = AppError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "secret kernel error",
        ));
        assert_eq!(io_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(io_err.error_code(), ErrorCode::IoError);
        assert_eq!(
            io_err.client_message(),
            "Internal file system error occurred."
        );

        let internal_err = AppError::Internal("sensitive database password in memory".into());
        assert_eq!(
            internal_err.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(internal_err.error_code(), ErrorCode::InternalServerError);
        assert_eq!(
            internal_err.client_message(),
            "An internal server error occurred."
        );

        let timeout_err = AppError::RequestTimeout("timed out after 30s".into());
        assert_eq!(timeout_err.status_code(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(timeout_err.error_code(), ErrorCode::RequestTimeout);
        assert_eq!(timeout_err.client_message(), "timed out after 30s");
    }

    #[tokio::test]
    async fn test_into_response_sanitization_and_json_payload() {
        let err = AppError::Internal("Raw database connection string with password".into());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Read response body failed");
        let payload: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("Parse ErrorResponse JSON failed");

        assert_eq!(payload.code, ErrorCode::InternalServerError);
        assert_eq!(payload.message, "An internal server error occurred.");
        assert_eq!(payload.details, None);
        assert!(!String::from_utf8_lossy(&body_bytes).contains("password"));
    }

    #[tokio::test]
    async fn test_validation_errors_with_details_payload() {
        let details = serde_json::json!({
            "size": ["Ukuran halaman maksimal adalah 100"]
        });

        let err = AppError::ValidationErrors {
            message: "Parameter input tidak valid.".into(),
            details: details.clone(),
        };

        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Read response body failed");
        let payload: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("Parse ErrorResponse JSON failed");

        assert_eq!(payload.code, ErrorCode::ValidationFailed);
        assert_eq!(payload.message, "Parameter input tidak valid.");
        assert_eq!(payload.details, Some(details));
    }
}
