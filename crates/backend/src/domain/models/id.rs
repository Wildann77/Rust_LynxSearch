use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FolderId(Uuid);

impl FolderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for FolderId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for FolderId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<FolderId> for Uuid {
    fn from(id: FolderId) -> Self {
        id.0
    }
}

impl fmt::Display for FolderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for FolderId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(Uuid);

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for JobId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for JobId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<JobId> for Uuid {
    fn from(id: JobId) -> Self {
        id.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for JobId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(Uuid);

impl DocumentId {
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_relative_path(folder_id: FolderId, relative_path: impl AsRef<Path>) -> Self {
        let raw = relative_path.as_ref().to_string_lossy();
        let normalized = raw.replace('\\', "/");
        let mut trimmed = normalized.as_str();

        while let Some(stripped) = trimmed.strip_prefix("./") {
            trimmed = stripped;
        }
        while let Some(stripped) = trimmed.strip_prefix('/') {
            trimmed = stripped;
        }

        Self(Uuid::new_v5(folder_id.as_uuid(), trimmed.as_bytes()))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Deref for DocumentId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for DocumentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DocumentId> for Uuid {
    fn from(id: DocumentId) -> Self {
        id.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DocumentId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_id_and_job_id_random_v4() {
        let f1 = FolderId::new();
        let f2 = FolderId::new();
        assert_ne!(f1, f2);
        assert_eq!(f1.get_version_num(), 4);

        let j1 = JobId::new();
        let j2 = JobId::new();
        assert_ne!(j1, j2);
        assert_eq!(j1.get_version_num(), 4);
    }

    #[test]
    fn test_document_id_deterministic_v5() {
        let folder_id = FolderId::new();
        let doc1 = DocumentId::from_relative_path(folder_id, "src/main.rs");
        let doc2 = DocumentId::from_relative_path(folder_id, "src/main.rs");
        assert_eq!(doc1, doc2);
        assert_eq!(doc1.get_version_num(), 5);

        // Cross-platform slash normalization
        let doc_windows = DocumentId::from_relative_path(folder_id, "src\\main.rs");
        let doc_dot_slash = DocumentId::from_relative_path(folder_id, "./src/main.rs");
        let doc_leading_slash = DocumentId::from_relative_path(folder_id, "/src/main.rs");
        assert_eq!(doc1, doc_windows);
        assert_eq!(doc1, doc_dot_slash);
        assert_eq!(doc1, doc_leading_slash);

        // Different path yields different ID
        let doc_other = DocumentId::from_relative_path(folder_id, "src/lib.rs");
        assert_ne!(doc1, doc_other);

        // Different folder yields different ID
        let other_folder = FolderId::new();
        let doc_diff_folder = DocumentId::from_relative_path(other_folder, "src/main.rs");
        assert_ne!(doc1, doc_diff_folder);
    }

    #[test]
    fn test_id_serde_roundtrip() {
        let folder_id = FolderId::new();
        let json = serde_json::to_string(&folder_id).unwrap();
        assert_eq!(json, format!("\"{}\"", folder_id));
        let decoded: FolderId = serde_json::from_str(&json).unwrap();
        assert_eq!(folder_id, decoded);
    }
}
