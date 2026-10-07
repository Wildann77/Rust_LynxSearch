use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDocumentResponseDto {
    pub success: bool,
    pub id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDetailResponseDto {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub folder_root_path: String,
    pub relative_path: String,
    pub title: String,
    pub content: String,
    pub r#type: String,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub file_size: i64,
    pub content_hash: String,
    pub updated_at: Option<String>,
}
