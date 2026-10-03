use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingJob {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub status: JobStatus,
    pub files_total: i32,
    pub files_processed: i32,
    pub files_indexed: i32,
    pub files_skipped: i32,
    pub files_failed: i32,
    pub error_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobProgressUpdate {
    pub status: Option<JobStatus>,
    pub files_total: Option<i32>,
    pub processed_delta: i32,
    pub indexed_delta: i32,
    pub skipped_delta: i32,
    pub failed_delta: i32,
    pub error_summary: Option<String>,
}
