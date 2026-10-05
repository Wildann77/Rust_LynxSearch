use crate::application::orchestrator::JobTracker;
use crate::error::AppError;
use crate::state::Repositories;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::task::JoinHandle;

pub const SHUTDOWN_CANCEL_REASON: &str = "Job dibatalkan karena graceful shutdown server";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShutdownReport {
    pub jobs_cancelled: u64,
    pub folders_reset: u64,
    pub grace_period_timed_out: bool,
}

pub async fn shutdown_in_flight_jobs(
    repositories: &Repositories,
    job_tracker: &JobTracker,
) -> Result<(u64, u64), AppError> {
    // 1. Cancel in-memory jobs and signal cancellation tokens
    job_tracker.cancel_all();

    // 2. Mark unfinished jobs (RUNNING or PENDING) as CANCELLED in repository
    let jobs_cancelled = repositories
        .job
        .cancel_unfinished_jobs(SHUTDOWN_CANCEL_REASON)
        .await?;

    if jobs_cancelled > 0 {
        tracing::info!(
            count = jobs_cancelled,
            "Marked unfinished jobs as CANCELLED during graceful shutdown"
        );
    }

    // 3. Reset folder status SCANNING -> IDLE
    let folders_reset = repositories.folder.reset_scanning_folders().await?;
    if folders_reset > 0 {
        tracing::info!(
            count = folders_reset,
            "Reset folder scanning statuses to IDLE during graceful shutdown"
        );
    }

    Ok((jobs_cancelled, folders_reset))
}

pub async fn drain_worker_with_grace_period<T: Send + 'static>(
    worker_handle: JoinHandle<T>,
    grace_period: Duration,
) -> bool {
    match tokio::time::timeout(grace_period, worker_handle).await {
        Ok(Ok(_)) => {
            tracing::info!("Worker task drained and exited cleanly within grace period");
            false
        }
        Ok(Err(join_err)) => {
            tracing::error!("Worker task joined with error: {join_err}");
            false
        }
        Err(_) => {
            tracing::warn!(
                grace_seconds = grace_period.as_secs(),
                "Grace period expired, forcing shutdown of background worker"
            );
            true
        }
    }
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to install Ctrl+C signal handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!("Failed to install SIGTERM signal handler: {err}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(windows)]
    let terminate = async {
        let mut ctrl_close = tokio::signal::windows::ctrl_close().ok();
        let mut ctrl_shutdown = tokio::signal::windows::ctrl_shutdown().ok();

        match (ctrl_close.as_mut(), ctrl_shutdown.as_mut()) {
            (Some(close), Some(shutdown)) => {
                tokio::select! {
                    _ = close.recv() => tracing::info!("Shutdown signal received (Windows CTRL_CLOSE)"),
                    _ = shutdown.recv() => tracing::info!("Shutdown signal received (Windows CTRL_SHUTDOWN)"),
                }
            }
            (Some(close), None) => {
                close.recv().await;
            }
            (None, Some(shutdown)) => {
                shutdown.recv().await;
            }
            (None, None) => {
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(any(unix, windows)))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Shutdown signal received (SIGINT/Ctrl+C)");
        },
        _ = terminate => {
            tracing::info!("Shutdown signal received (SIGTERM/OS-terminate)");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Folder, FolderStatus, IndexingJob, JobStatus};
    use crate::domain::models::{FolderId, JobId};
    use chrono::Utc;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_shutdown_in_flight_jobs_cancels_jobs_and_resets_folders() {
        let repos = Repositories::in_memory();
        let tracker = JobTracker::new();
        let folder_id = FolderId::new();

        let mut folder = Folder::new(PathBuf::from("/test/shutdown"));
        folder.id = folder_id;
        folder.status = FolderStatus::Scanning;
        repos.folder.create_folder(&folder).await.unwrap();

        let running_job = IndexingJob {
            id: JobId::new(),
            folder_id: Some(folder_id),
            job_type: Default::default(),
            status: JobStatus::Running,
            files_total: 100,
            files_processed: 25,
            files_indexed: 20,
            files_skipped: 5,
            files_failed: 0,
            error_summary: None,
            started_at: Utc::now(),
            completed_at: None,
        };
        repos.job.create_job(&running_job).await.unwrap();

        let pending_job = IndexingJob {
            id: JobId::new(),
            folder_id: Some(folder_id),
            job_type: Default::default(),
            status: JobStatus::Pending,
            files_total: 0,
            files_processed: 0,
            files_indexed: 0,
            files_skipped: 0,
            files_failed: 0,
            error_summary: None,
            started_at: Utc::now(),
            completed_at: None,
        };
        repos.job.create_job(&pending_job).await.unwrap();

        let completed_job = IndexingJob {
            id: JobId::new(),
            folder_id: Some(folder_id),
            job_type: Default::default(),
            status: JobStatus::Completed,
            files_total: 10,
            files_processed: 10,
            files_indexed: 10,
            files_skipped: 0,
            files_failed: 0,
            error_summary: None,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        repos.job.create_job(&completed_job).await.unwrap();

        let (cancelled, reset) = shutdown_in_flight_jobs(&repos, &tracker).await.unwrap();
        assert_eq!(cancelled, 2);
        assert_eq!(reset, 1);

        let cancelled_running = repos.job.get_job(&running_job.id).await.unwrap().unwrap();
        assert_eq!(cancelled_running.status, JobStatus::Cancelled);
        assert_eq!(
            cancelled_running.error_summary.as_deref(),
            Some(SHUTDOWN_CANCEL_REASON)
        );
        assert!(cancelled_running.completed_at.is_some());

        let cancelled_pending = repos.job.get_job(&pending_job.id).await.unwrap().unwrap();
        assert_eq!(cancelled_pending.status, JobStatus::Cancelled);
        assert!(cancelled_pending.completed_at.is_some());

        let untouched_completed = repos.job.get_job(&completed_job.id).await.unwrap().unwrap();
        assert_eq!(untouched_completed.status, JobStatus::Completed);

        let updated_folder = repos.folder.get_folder(&folder_id).await.unwrap().unwrap();
        assert_eq!(updated_folder.status, FolderStatus::Idle);

        // Idempotency: second call returns 0 cancelled, 0 reset
        let (cancelled_again, reset_again) =
            shutdown_in_flight_jobs(&repos, &tracker).await.unwrap();
        assert_eq!(cancelled_again, 0);
        assert_eq!(reset_again, 0);
    }

    #[tokio::test]
    async fn test_drain_worker_clean_exit() {
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });

        let timed_out = drain_worker_with_grace_period(handle, Duration::from_millis(500)).await;
        assert!(!timed_out);
    }

    #[tokio::test]
    async fn test_drain_worker_timeout_grace_period() {
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        let timed_out = drain_worker_with_grace_period(handle, Duration::from_millis(30)).await;
        assert!(timed_out);
    }
}
