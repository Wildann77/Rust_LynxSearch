use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub title: String,
    pub relative_path: String,
    pub content: String,
    pub doc_type: String,
    pub language: Option<String>,
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
