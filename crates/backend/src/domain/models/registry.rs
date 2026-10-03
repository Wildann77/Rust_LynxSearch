use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    Indexed,
    Excluded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub relative_path: String,
    pub content_hash: String,
    pub file_size: i64,
    pub status: DocumentStatus,
    pub indexed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
