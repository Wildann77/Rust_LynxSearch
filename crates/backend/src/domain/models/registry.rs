use super::id::{DocumentId, FolderId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    Indexed,
    Excluded,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Indexed => "INDEXED",
            Self::Excluded => "EXCLUDED",
        }
    }
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DocumentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "INDEXED" => Ok(Self::Indexed),
            "EXCLUDED" => Ok(Self::Excluded),
            other => Err(format!("Unknown document status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub relative_path: String,
    pub content_hash: String,
    pub file_size: i64,
    pub status: DocumentStatus,
    pub indexed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
