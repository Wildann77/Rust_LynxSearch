use super::id::{FolderId, JobId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobType {
    #[default]
    Import,
    Rescan,
    Rebuild,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "IMPORT",
            Self::Rescan => "RESCAN",
            Self::Rebuild => "REBUILD",
        }
    }
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "IMPORT" => Ok(Self::Import),
            "RESCAN" => Ok(Self::Rescan),
            "REBUILD" => Ok(Self::Rebuild),
            other => Err(format!("Unknown job type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(&self, next: JobStatus) -> bool {
        match self {
            Self::Pending => matches!(next, Self::Running | Self::Cancelled | Self::Failed),
            Self::Running => matches!(next, Self::Completed | Self::Failed | Self::Cancelled),
            Self::Completed | Self::Failed | Self::Cancelled => false,
        }
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "PENDING" => Ok(Self::Pending),
            "RUNNING" => Ok(Self::Running),
            "COMPLETED" => Ok(Self::Completed),
            "FAILED" => Ok(Self::Failed),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(format!("Unknown job status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingJob {
    pub id: JobId,
    pub folder_id: Option<FolderId>,
    #[serde(default)]
    pub job_type: JobType,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_parsing_and_display() {
        assert_eq!(JobType::Import.as_str(), "IMPORT");
        assert_eq!(JobType::Rescan.as_str(), "RESCAN");
        assert_eq!(JobType::Rebuild.as_str(), "REBUILD");

        assert_eq!(JobType::from_str("IMPORT").unwrap(), JobType::Import);
        assert_eq!(JobType::from_str("rescan").unwrap(), JobType::Rescan);
        assert_eq!(JobType::from_str("REBUILD").unwrap(), JobType::Rebuild);
        assert!(JobType::from_str("UNKNOWN").is_err());
    }

    #[test]
    fn test_job_type_serde_roundtrip() {
        let job_type = JobType::Rescan;
        let json = serde_json::to_string(&job_type).unwrap();
        assert_eq!(json, "\"RESCAN\"");
        let deserialized: JobType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, job_type);
    }

    #[test]
    fn test_job_status_parsing_and_display() {
        assert_eq!(JobStatus::Pending.as_str(), "PENDING");
        assert_eq!(JobStatus::Running.as_str(), "RUNNING");
        assert_eq!(JobStatus::Completed.as_str(), "COMPLETED");
        assert_eq!(JobStatus::Failed.as_str(), "FAILED");
        assert_eq!(JobStatus::Cancelled.as_str(), "CANCELLED");

        assert_eq!(JobStatus::from_str("PENDING").unwrap(), JobStatus::Pending);
        assert_eq!(JobStatus::from_str("running").unwrap(), JobStatus::Running);
        assert_eq!(
            JobStatus::from_str("COMPLETED").unwrap(),
            JobStatus::Completed
        );
        assert_eq!(JobStatus::from_str("FAILED").unwrap(), JobStatus::Failed);
        assert_eq!(
            JobStatus::from_str("cancelled").unwrap(),
            JobStatus::Cancelled
        );
        assert!(JobStatus::from_str("UNKNOWN").is_err());
    }

    #[test]
    fn test_job_status_terminal_and_transitions() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());

        // Pending transitions
        assert!(JobStatus::Pending.can_transition_to(JobStatus::Running));
        assert!(JobStatus::Pending.can_transition_to(JobStatus::Cancelled));
        assert!(JobStatus::Pending.can_transition_to(JobStatus::Failed));
        assert!(!JobStatus::Pending.can_transition_to(JobStatus::Completed));

        // Running transitions
        assert!(JobStatus::Running.can_transition_to(JobStatus::Completed));
        assert!(JobStatus::Running.can_transition_to(JobStatus::Failed));
        assert!(JobStatus::Running.can_transition_to(JobStatus::Cancelled));
        assert!(!JobStatus::Running.can_transition_to(JobStatus::Pending));

        // Terminal cannot transition
        for terminal in [
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert!(!terminal.can_transition_to(JobStatus::Pending));
            assert!(!terminal.can_transition_to(JobStatus::Running));
            assert!(!terminal.can_transition_to(JobStatus::Completed));
            assert!(!terminal.can_transition_to(JobStatus::Failed));
            assert!(!terminal.can_transition_to(JobStatus::Cancelled));
        }
    }
}
