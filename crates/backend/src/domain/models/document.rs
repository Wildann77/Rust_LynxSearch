use super::id::{DocumentId, FolderId};
use super::types::{DocumentType, Language};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub title: String,
    pub relative_path: String,
    pub content: String,
    pub doc_type: DocumentType,
    pub language: Option<Language>,
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub file_size: u64,
    pub content_hash: String,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkIndexReport {
    pub indexed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRawResponse {
    pub raw_json: String,
}
