use crate::domain::models::{FolderId, JobId, JobProgressUpdate, JobStatus};
use crate::error::AppError;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// RAII lock guard holding both a read lock on the global rebuild lock
/// and an owned mutex lock on a specific folder.
/// Automatically releases both locks when dropped.
#[derive(Debug)]
pub struct FolderLockGuard {
    _rebuild_guard: OwnedRwLockReadGuard<()>,
    _folder_guard: OwnedMutexGuard<()>,
}

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
    active_folder_locks: DashMap<FolderId, FolderLockGuard>,
    rebuild_lock: Arc<RwLock<()>>,
    active_rebuild_guard: std::sync::Mutex<Option<OwnedRwLockWriteGuard<()>>>,
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
            active_folder_locks: DashMap::new(),
            rebuild_lock: Arc::new(RwLock::new(())),
            active_rebuild_guard: std::sync::Mutex::new(None),
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

    /// Tries to acquire a non-blocking lock for a specific folder scan.
    ///
    /// Fails with `AppError::JobConflict` if:
    /// 1. A global index rebuild is currently in progress (cannot acquire rebuild read lock).
    /// 2. An indexing scan is already running or active on this folder (cannot acquire folder mutex).
    pub fn try_acquire_folder_lock(
        &self,
        folder_id: &FolderId,
    ) -> Result<FolderLockGuard, AppError> {
        if self.is_rebuilding() {
            return Err(AppError::JobConflict(*folder_id.as_uuid()));
        }

        if self.active_folder_locks.contains_key(folder_id) {
            return Err(AppError::JobConflict(*folder_id.as_uuid()));
        }

        let rebuild_guard = self
            .rebuild_lock
            .clone()
            .try_read_owned()
            .map_err(|_| AppError::JobConflict(*folder_id.as_uuid()))?;

        let lock_arc = self
            .folder_locks
            .entry(*folder_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        let folder_guard = lock_arc
            .try_lock_owned()
            .map_err(|_| AppError::JobConflict(*folder_id.as_uuid()))?;

        Ok(FolderLockGuard {
            _rebuild_guard: rebuild_guard,
            _folder_guard: folder_guard,
        })
    }

    /// Acquires and stores a folder lock guard in active state.
    /// Fails with `AppError::JobConflict` if folder is already locked or rebuild in progress.
    pub fn try_lock_folder(&self, folder_id: &FolderId) -> Result<(), AppError> {
        if self.is_rebuilding() {
            return Err(AppError::JobConflict(*folder_id.as_uuid()));
        }
        if self.active_folder_locks.contains_key(folder_id) {
            return Err(AppError::JobConflict(*folder_id.as_uuid()));
        }
        let guard = self.try_acquire_folder_lock(folder_id)?;
        self.active_folder_locks.insert(*folder_id, guard);
        Ok(())
    }

    /// Releases the active folder lock guard for a specific folder.
    pub fn release_folder_lock(&self, folder_id: &FolderId) {
        self.active_folder_locks.remove(folder_id);
    }

    /// Asynchronously acquires a lock for a folder scan.
    pub async fn acquire_folder_lock(&self, folder_id: &FolderId) -> FolderLockGuard {
        let rebuild_guard = self.rebuild_lock.clone().read_owned().await;
        let lock_arc = self
            .folder_locks
            .entry(*folder_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let folder_guard = lock_arc.lock_owned().await;
        FolderLockGuard {
            _rebuild_guard: rebuild_guard,
            _folder_guard: folder_guard,
        }
    }

    /// Tries to acquire a non-blocking exclusive global lock for an index rebuild.
    ///
    /// Fails with `AppError::JobConflict` if any folder scan or another rebuild is active.
    pub fn try_acquire_rebuild_lock(&self) -> Result<OwnedRwLockWriteGuard<()>, AppError> {
        if self.is_rebuilding() {
            return Err(AppError::JobConflict(Uuid::nil()));
        }
        if !self.active_folder_locks.is_empty() {
            return Err(AppError::JobConflict(Uuid::nil()));
        }
        self.rebuild_lock
            .clone()
            .try_write_owned()
            .map_err(|_| AppError::JobConflict(Uuid::nil()))
    }

    /// Acquires and stores an exclusive global rebuild lock guard.
    pub fn try_lock_rebuild(&self) -> Result<(), AppError> {
        let guard = self.try_acquire_rebuild_lock()?;
        let mut active = self
            .active_rebuild_guard
            .lock()
            .map_err(|_| AppError::Internal("Rebuild lock poisoned".into()))?;
        *active = Some(guard);
        Ok(())
    }

    /// Releases the active global rebuild lock guard.
    pub fn release_rebuild_lock(&self) {
        if let Ok(mut active) = self.active_rebuild_guard.lock() {
            *active = None;
        }
    }

    /// Clears the global rebuild lock.
    pub fn clear_rebuild_lock(&self) {
        self.release_rebuild_lock();
    }

    /// Asynchronously acquires an exclusive global lock for an index rebuild.
    pub async fn acquire_rebuild_lock(&self) -> OwnedRwLockWriteGuard<()> {
        self.rebuild_lock.clone().write_owned().await
    }

    /// Checks if a folder lock is currently held by an active job.
    pub fn is_folder_locked(&self, folder_id: &FolderId) -> bool {
        if self.active_folder_locks.contains_key(folder_id) {
            return true;
        }
        if let Some(entry) = self.folder_locks.get(folder_id) {
            entry.try_lock().is_err()
        } else {
            false
        }
    }

    /// Checks if a global rebuild lock is currently held.
    pub fn is_rebuilding(&self) -> bool {
        if let Ok(active) = self.active_rebuild_guard.lock()
            && active.is_some()
        {
            return true;
        }
        self.rebuild_lock.try_read().is_err()
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
                if entry.status == status || entry.status.can_transition_to(status) {
                    entry.status = status;
                } else {
                    tracing::warn!(
                        job_id = %job_id,
                        current = %entry.status,
                        attempted = %status,
                        "Ignored invalid job status transition"
                    );
                }
            }
            if let Some(total) = update.files_total {
                entry.files_total = total;
            }
            entry.files_processed += update.processed_delta;
            entry.files_indexed += update.indexed_delta;
            entry.files_skipped += update.skipped_delta;
            entry.files_failed += update.failed_delta;

            let sum_sub_counters = entry.files_indexed + entry.files_skipped + entry.files_failed;
            if sum_sub_counters > entry.files_processed {
                entry.files_processed = sum_sub_counters;
            }

            if update.error_summary.is_some() {
                entry.error_summary = update.error_summary;
            }

            if entry.status.is_terminal() && entry.completed_at.is_none() {
                entry.completed_at = Some(Utc::now());
            }
        }
    }

    pub fn get_progress(&self, job_id: &JobId) -> Option<JobProgressState> {
        self.jobs.get(job_id).map(|entry| entry.clone())
    }

    pub fn is_job_cancelled(&self, job_id: &JobId) -> bool {
        self.cancellation_tokens
            .get(job_id)
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    pub fn get_cancellation_token(&self, job_id: &JobId) -> Option<CancellationToken> {
        self.cancellation_tokens.get(job_id).map(|t| t.clone())
    }

    pub fn get_folder_for_job(&self, job_id: &JobId) -> Option<FolderId> {
        self.jobs.get(job_id).and_then(|j| j.folder_id)
    }

    pub fn cancel_job(&self, job_id: &JobId) -> bool {
        if let Some(token) = self.cancellation_tokens.get(job_id) {
            token.cancel();
            if let Some(mut entry) = self.jobs.get_mut(job_id)
                && !entry.status.is_terminal()
            {
                let was_pending = entry.status == JobStatus::Pending;
                let folder_id_opt = entry.folder_id;
                entry.status = JobStatus::Cancelled;
                entry.completed_at = Some(Utc::now());

                if was_pending {
                    if let Some(folder_id) = folder_id_opt {
                        self.release_folder_lock(&folder_id);
                    } else {
                        self.release_rebuild_lock();
                    }
                }
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
            if !entry.status.is_terminal() {
                entry.status = JobStatus::Cancelled;
                entry.completed_at = Some(Utc::now());
            }
        }
        self.clear_folder_locks();
        self.clear_rebuild_lock();
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
        self.active_folder_locks.clear();
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

    #[tokio::test]
    async fn test_concurrent_folder_locks_independent() {
        let tracker = JobTracker::new();
        let folder_a = FolderId::new();
        let folder_b = FolderId::new();

        // Lock folder A and folder B concurrently
        let guard_a = tracker.try_acquire_folder_lock(&folder_a);
        assert!(guard_a.is_ok(), "Folder A lock should succeed");

        let guard_b = tracker.try_acquire_folder_lock(&folder_b);
        assert!(
            guard_b.is_ok(),
            "Folder B lock should succeed concurrently with A"
        );

        assert!(tracker.is_folder_locked(&folder_a));
        assert!(tracker.is_folder_locked(&folder_b));
        assert!(!tracker.is_rebuilding());

        // Same folder A lock attempt must fail with JobConflict
        let guard_a_again = tracker.try_acquire_folder_lock(&folder_a);
        assert!(
            matches!(guard_a_again, Err(AppError::JobConflict(_))),
            "Concurrent lock on same folder A must fail with JobConflict"
        );

        // Drop guard A and verify A can be re-locked
        drop(guard_a);
        assert!(!tracker.is_folder_locked(&folder_a));
        assert!(tracker.is_folder_locked(&folder_b));

        let guard_a_new = tracker.try_acquire_folder_lock(&folder_a);
        assert!(guard_a_new.is_ok(), "Folder A can be re-locked after drop");
    }

    #[tokio::test]
    async fn test_rebuild_lock_blocks_folder_scans() {
        let tracker = JobTracker::new();
        let folder_a = FolderId::new();

        let rebuild_guard = tracker.try_acquire_rebuild_lock();
        assert!(rebuild_guard.is_ok(), "Rebuild lock should succeed");
        assert!(tracker.is_rebuilding());

        // Any folder scan lock attempt must fail with JobConflict
        let scan_guard = tracker.try_acquire_folder_lock(&folder_a);
        assert!(
            matches!(scan_guard, Err(AppError::JobConflict(_))),
            "Folder scan must fail when rebuild is active"
        );

        // Second rebuild must also fail
        let rebuild_again = tracker.try_acquire_rebuild_lock();
        assert!(
            matches!(rebuild_again, Err(AppError::JobConflict(_))),
            "Concurrent rebuild must fail"
        );

        // Drop rebuild lock
        drop(rebuild_guard);
        assert!(!tracker.is_rebuilding());

        // Now folder scan succeeds
        let scan_ok = tracker.try_acquire_folder_lock(&folder_a);
        assert!(scan_ok.is_ok(), "Folder scan succeeds after rebuild ends");
    }

    #[tokio::test]
    async fn test_folder_scan_blocks_rebuild() {
        let tracker = JobTracker::new();
        let folder_a = FolderId::new();

        let scan_guard = tracker.try_acquire_folder_lock(&folder_a);
        assert!(scan_guard.is_ok());

        // Rebuild lock must fail while folder scan holds read lock
        let rebuild_guard = tracker.try_acquire_rebuild_lock();
        assert!(
            matches!(rebuild_guard, Err(AppError::JobConflict(_))),
            "Rebuild lock must fail while folder scan is active"
        );

        drop(scan_guard);
        let rebuild_ok = tracker.try_acquire_rebuild_lock();
        assert!(
            rebuild_ok.is_ok(),
            "Rebuild succeeds after folder scan drops"
        );
    }

    #[test]
    fn test_job_tracker_status_transitions_and_counter_consistency() {
        let tracker = JobTracker::new();
        let job_id = JobId::new();
        let token = CancellationToken::new();

        tracker.register_job(job_id, None, token);
        let initial = tracker.get_progress(&job_id).unwrap();
        assert_eq!(initial.status, JobStatus::Pending);

        // Transition: PENDING -> RUNNING with files_total
        tracker.update_progress(
            &job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Running),
                files_total: Some(100),
                ..Default::default()
            },
        );
        let running = tracker.get_progress(&job_id).unwrap();
        assert_eq!(running.status, JobStatus::Running);
        assert_eq!(running.files_total, 100);

        // Counter increments
        tracker.update_progress(
            &job_id,
            JobProgressUpdate {
                indexed_delta: 20,
                skipped_delta: 5,
                failed_delta: 1,
                ..Default::default()
            },
        );
        let progress = tracker.get_progress(&job_id).unwrap();
        assert_eq!(progress.files_indexed, 20);
        assert_eq!(progress.files_skipped, 5);
        assert_eq!(progress.files_failed, 1);
        assert_eq!(progress.files_processed, 26);

        // Complete job
        tracker.update_progress(
            &job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Completed),
                ..Default::default()
            },
        );
        let completed = tracker.get_progress(&job_id).unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
        assert!(completed.completed_at.is_some());

        // Attempt invalid transition: COMPLETED -> RUNNING should be rejected
        tracker.update_progress(
            &job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Running),
                ..Default::default()
            },
        );
        let still_completed = tracker.get_progress(&job_id).unwrap();
        assert_eq!(still_completed.status, JobStatus::Completed);

        // Cancel job should not overwrite completed job
        assert!(tracker.cancel_job(&job_id));
        let after_cancel = tracker.get_progress(&job_id).unwrap();
        assert_eq!(after_cancel.status, JobStatus::Completed);
    }
}
