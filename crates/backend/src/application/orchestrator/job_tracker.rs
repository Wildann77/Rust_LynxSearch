use crate::domain::models::{FolderId, JobId, JobProgressUpdate, JobStatus};
use crate::error::AppError;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressState {
    pub job_id: JobId,
    pub folder_id: Option<FolderId>,
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

pub struct JobTracker {
    folder_locks: DashMap<FolderId, Arc<Mutex<()>>>,
    jobs: DashMap<JobId, JobProgressState>,
    cancellation_tokens: DashMap<JobId, CancellationToken>,
    shutdown_token: CancellationToken,
}

impl Default for JobTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl JobTracker {
    pub fn new() -> Self {
        Self {
            folder_locks: DashMap::new(),
            jobs: DashMap::new(),
            cancellation_tokens: DashMap::new(),
            shutdown_token: CancellationToken::new(),
        }
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_token.is_cancelled()
    }

    pub fn try_acquire_folder_lock(
        &self,
        folder_id: &FolderId,
    ) -> Result<OwnedMutexGuard<()>, AppError> {
        let lock_arc = self
            .folder_locks
            .entry(*folder_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        lock_arc
            .try_lock_owned()
            .map_err(|_| AppError::JobConflict(*folder_id.as_uuid()))
    }

    pub async fn acquire_folder_lock(&self, folder_id: &FolderId) -> OwnedMutexGuard<()> {
        let lock_arc = self
            .folder_locks
            .entry(*folder_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        lock_arc.lock_owned().await
    }

    pub fn register_job(
        &self,
        job_id: impl Into<JobId>,
        folder_id: impl Into<Option<FolderId>>,
        token: CancellationToken,
    ) {
        let job_id = job_id.into();
        let folder_id = folder_id.into();
        let shutting_down = self.is_shutting_down();
        if shutting_down {
            token.cancel();
        }

        let state = JobProgressState {
            job_id,
            folder_id,
            status: if shutting_down {
                JobStatus::Cancelled
            } else {
                JobStatus::Pending
            },
            files_total: 0,
            files_processed: 0,
            files_indexed: 0,
            files_skipped: 0,
            files_failed: 0,
            error_summary: if shutting_down {
                Some("Server is shutting down".to_string())
            } else {
                None
            },
            started_at: Utc::now(),
            completed_at: if shutting_down {
                Some(Utc::now())
            } else {
                None
            },
        };

        self.jobs.insert(job_id, state);
        self.cancellation_tokens.insert(job_id, token);
    }

    pub fn update_progress(&self, job_id: &JobId, update: JobProgressUpdate) {
        if let Some(mut entry) = self.jobs.get_mut(job_id) {
            if let Some(status) = update.status {
                entry.status = status;
            }
            if let Some(total) = update.files_total {
                entry.files_total = total;
            }
            entry.files_processed += update.processed_delta;
            entry.files_indexed += update.indexed_delta;
            entry.files_skipped += update.skipped_delta;
            entry.files_failed += update.failed_delta;

            if update.error_summary.is_some() {
                entry.error_summary = update.error_summary;
            }

            if entry.status == JobStatus::Completed
                || entry.status == JobStatus::Failed
                || entry.status == JobStatus::Cancelled
            {
                entry.completed_at = Some(Utc::now());
            }
        }
    }

    pub fn get_progress(&self, job_id: &JobId) -> Option<JobProgressState> {
        self.jobs.get(job_id).map(|entry| entry.clone())
    }

    pub fn cancel_job(&self, job_id: &JobId) -> bool {
        if let Some(token) = self.cancellation_tokens.get(job_id) {
            token.cancel();
            if let Some(mut entry) = self.jobs.get_mut(job_id) {
                entry.status = JobStatus::Cancelled;
                entry.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    pub fn cancel_all(&self) {
        self.shutdown_token.cancel();
        for entry in self.cancellation_tokens.iter() {
            entry.value().cancel();
        }
        for mut entry in self.jobs.iter_mut() {
            if entry.status == JobStatus::Running || entry.status == JobStatus::Pending {
                entry.status = JobStatus::Cancelled;
                entry.completed_at = Some(Utc::now());
            }
        }
    }

    pub fn fail_running_jobs(&self, error_summary: &str) -> u64 {
        let mut count = 0;
        let now = Utc::now();
        for mut entry in self.jobs.iter_mut() {
            if entry.status == JobStatus::Running || entry.status == JobStatus::Pending {
                entry.status = JobStatus::Failed;
                entry.completed_at = Some(now);
                entry.error_summary = Some(error_summary.to_string());
                count += 1;
            }
        }
        for entry in self.cancellation_tokens.iter() {
            entry.value().cancel();
        }
        count
    }

    pub fn clear_folder_locks(&self) {
        self.folder_locks.clear();
    }

    pub fn active_jobs_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Pending)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_tracker_shutdown_and_cancellation() {
        let tracker = JobTracker::new();
        assert!(!tracker.is_shutting_down());
        let shutdown_token = tracker.shutdown_token();
        assert!(!shutdown_token.is_cancelled());

        let job_id = JobId::new();
        let folder_id = FolderId::new();
        let job_token = CancellationToken::new();
        tracker.register_job(job_id, folder_id, job_token.clone());

        assert_eq!(tracker.active_jobs_count(), 1);
        let progress = tracker.get_progress(&job_id).unwrap();
        assert_eq!(progress.status, JobStatus::Pending);
        assert!(!job_token.is_cancelled());

        tracker.cancel_all();

        assert!(tracker.is_shutting_down());
        assert!(shutdown_token.is_cancelled());
        assert!(job_token.is_cancelled());
        assert_eq!(tracker.active_jobs_count(), 0);

        let cancelled_progress = tracker.get_progress(&job_id).unwrap();
        assert_eq!(cancelled_progress.status, JobStatus::Cancelled);
        assert!(cancelled_progress.completed_at.is_some());

        // Registering a job after shutdown immediately cancels it
        let job_id_2 = JobId::new();
        let token_2 = CancellationToken::new();
        tracker.register_job(job_id_2, folder_id, token_2.clone());
        assert!(token_2.is_cancelled());
        let progress_2 = tracker.get_progress(&job_id_2).unwrap();
        assert_eq!(progress_2.status, JobStatus::Cancelled);
    }
}
