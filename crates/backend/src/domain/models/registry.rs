use super::id::{DocumentId, FolderId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    Indexed,
    Skipped,
    Failed,
    Excluded,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Indexed => "INDEXED",
            Self::Skipped => "SKIPPED",
            Self::Failed => "FAILED",
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
            "SKIPPED" => Ok(Self::Skipped),
            "FAILED" => Ok(Self::Failed),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<String>,
    pub indexed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_status_parsing_and_display() {
        assert_eq!(DocumentStatus::Indexed.as_str(), "INDEXED");
        assert_eq!(DocumentStatus::Skipped.as_str(), "SKIPPED");
        assert_eq!(DocumentStatus::Failed.as_str(), "FAILED");
        assert_eq!(DocumentStatus::Excluded.as_str(), "EXCLUDED");

        assert_eq!(
            DocumentStatus::from_str("INDEXED").unwrap(),
            DocumentStatus::Indexed
        );
        assert_eq!(
            DocumentStatus::from_str("skipped").unwrap(),
            DocumentStatus::Skipped
        );
        assert_eq!(
            DocumentStatus::from_str("FAILED").unwrap(),
            DocumentStatus::Failed
        );
        assert_eq!(
            DocumentStatus::from_str("excluded").unwrap(),
            DocumentStatus::Excluded
        );
        assert!(DocumentStatus::from_str("UNKNOWN").is_err());
    }

    #[test]
    fn test_document_status_serde_roundtrip() {
        let status = DocumentStatus::Skipped;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"SKIPPED\"");
        let deserialized: DocumentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}
