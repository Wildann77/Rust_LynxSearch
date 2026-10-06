use super::id::{DocumentId, FolderId};
use super::types::{DocumentType, Language};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub relative_path: String,
    pub absolute_path: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(rename = "type")]
    pub doc_type: DocumentType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(rename = "file_size_bytes", alias = "file_size")]
    pub file_size_bytes: u64,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkIndexReport {
    pub indexed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRawResponse {
    pub raw_json: String,
    #[serde(default)]
    pub took_ms: u64,
}

impl SearchRawResponse {
    pub fn new(raw_json: impl Into<String>, took_ms: u64) -> Self {
        Self {
            raw_json: raw_json.into(),
            took_ms,
        }
    }

    pub fn from_raw(raw_json: impl Into<String>) -> Self {
        Self {
            raw_json: raw_json.into(),
            took_ms: 0,
        }
    }

    pub fn parse_execution_result(
        &self,
    ) -> Result<super::search::SearchExecutionResult, crate::error::AppError> {
        super::search::parse_search_execution_result(&self.raw_json, self.took_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_indexed_document_serde_roundtrip() {
        let doc_id =
            DocumentId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap());
        let folder_id =
            FolderId::from_uuid(Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap());
        let now = Utc::now();

        let doc = IndexedDocument {
            id: doc_id,
            folder_id,
            relative_path: "src/main.rs".to_string(),
            absolute_path: "/workspace/src/main.rs".to_string(),
            title: "main.rs".to_string(),
            content: "fn main() {}".to_string(),
            tags: vec!["backend".to_string(), "entrypoint".to_string()],
            extension: Some("rs".to_string()),
            language: Some(Language::Rust),
            doc_type: DocumentType::Code,
            project: Some("lynx".to_string()),
            file_size_bytes: 1024,
            modified_at: now,
            indexed_at: now,
            content_hash: Some("sha256_mock".to_string()),
        };

        let json = serde_json::to_value(&doc).expect("Serialize to JSON");
        assert_eq!(json["type"], "code");
        assert_eq!(json["file_size_bytes"], 1024);
        assert_eq!(json["language"], "rust");
        assert_eq!(json["absolute_path"], "/workspace/src/main.rs");
        assert_eq!(json["relative_path"], "src/main.rs");

        let deserialized: IndexedDocument =
            serde_json::from_value(json).expect("Deserialize from JSON");
        assert_eq!(deserialized, doc);
    }

    #[test]
    fn test_indexed_document_alias_file_size() {
        let json_str = r##"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "folder_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "relative_path": "README.md",
            "absolute_path": "/workspace/README.md",
            "title": "README",
            "content": "# Hello",
            "type": "doc",
            "file_size": 256,
            "modified_at": "2026-10-03T00:00:00Z",
            "indexed_at": "2026-10-03T00:00:00Z"
        }"##;

        let doc: IndexedDocument =
            serde_json::from_str(json_str).expect("Deserialize with alias file_size");
        assert_eq!(doc.file_size_bytes, 256);
        assert_eq!(doc.doc_type, DocumentType::Doc);
        assert!(doc.tags.is_empty());
        assert!(doc.language.is_none());
        assert!(doc.project.is_none());
    }
}
