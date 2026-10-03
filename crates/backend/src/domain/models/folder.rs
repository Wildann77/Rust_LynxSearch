use super::id::FolderId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FolderStatus {
    Idle,
    Scanning,
    Error,
}

impl FolderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Scanning => "SCANNING",
            Self::Error => "ERROR",
        }
    }
}

impl fmt::Display for FolderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for FolderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "IDLE" => Ok(Self::Idle),
            "SCANNING" => Ok(Self::Scanning),
            "ERROR" => Ok(Self::Error),
            other => Err(format!("Unknown folder status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub path: PathBuf,
    pub status: FolderStatus,
    pub created_at: DateTime<Utc>,
    pub last_scanned_at: Option<DateTime<Utc>>,
}

impl Folder {
    pub fn new(path: PathBuf) -> Self {
        Self {
            id: FolderId::new(),
            path,
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        }
    }
}
