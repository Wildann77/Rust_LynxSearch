use backend::application::orchestrator::supervisor::{
    DEFAULT_SUPERVISOR_BACKOFF, WORKER_PANIC_ERROR_MSG, recover_panicked_worker, run_supervisor,
    spawn_worker_supervisor,
};
use backend::application::orchestrator::{JobTracker, WorkerCommand};
use backend::domain::models::{Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus};
use backend::state::{AppState, Repositories};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_recover_panicked_worker_marks_jobs_failed_and_releases_folders() {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();

    let mut folder = Folder::new(PathBuf::from("/test/supervisor/folder"));
    folder.id = folder_id;
    folder.status = FolderStatus::Scanning;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let job_id = JobId::new();
    let job_token = CancellationToken::new();
    state
        .job_tracker
        .register_job(job_id, folder_id, job_token.clone());

    let running_job = IndexingJob {
        id: job_id,
        folder_id: Some(folder_id),
        status: JobStatus::Running,
        files_total: 50,
        files_processed: 10,
        files_indexed: 9,
        files_skipped: 1,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    state
        .repositories
        .job
        .create_job(&running_job)
        .await
        .unwrap();

    // Acquire lock in job tracker
    let lock_guard = state.job_tracker.try_acquire_folder_lock(&folder_id);
    assert!(lock_guard.is_ok());

    // Recover from panic
    let (jobs_failed, folders_reset) =
        recover_panicked_worker(&state.repositories, &state.job_tracker)
            .await
            .unwrap();

    assert_eq!(jobs_failed, 1);
    assert_eq!(folders_reset, 1);

    // Verify DB job
    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Failed);
    assert_eq!(
        db_job.error_summary,
        Some(WORKER_PANIC_ERROR_MSG.to_string())
    );
    assert!(db_job.completed_at.is_some());

    // Verify DB folder
    let db_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_folder.status, FolderStatus::Idle);

    // Verify in-memory tracker
    let tracked = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracked.status, JobStatus::Failed);
    assert_eq!(
        tracked.error_summary,
        Some(WORKER_PANIC_ERROR_MSG.to_string())
    );
    assert!(job_token.is_cancelled());

    // Verify lock can be acquired again
    let new_lock = state.job_tracker.try_acquire_folder_lock(&folder_id);
    assert!(new_lock.is_ok());
}

#[tokio::test]
async fn test_supervisor_catches_panic_and_restarts_worker_with_backoff() {
    let repositories = Repositories::in_memory();
    let job_tracker = Arc::new(JobTracker::new());
    let shutdown_token = CancellationToken::new();

    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    let shutdown_clone = shutdown_token.clone();

    let factory = move || {
        let attempts = attempts_clone.clone();
        let shutdown = shutdown_clone.clone();
        async move {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                panic!("Worker crashed on iteration {n}!");
            } else {
                // Succeeded after 2 crashes
                shutdown.cancel();
            }
        }
    };

    let start = std::time::Instant::now();
    run_supervisor(
        factory,
        repositories,
        job_tracker,
        shutdown_token,
        Duration::from_millis(50),
    )
    .await;

    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    // 2 panics * 50ms backoff = at least 100ms
    assert!(start.elapsed() >= Duration::from_millis(80));
}

#[tokio::test]
async fn test_supervisor_aborts_immediately_if_shutdown_triggered_during_backoff() {
    let repositories = Repositories::in_memory();
    let job_tracker = Arc::new(JobTracker::new());
    let shutdown_token = CancellationToken::new();

    let factory = move || async move {
        panic!("Immediate worker panic");
    };

    let cancel_handle = shutdown_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        cancel_handle.cancel();
    });

    let start = std::time::Instant::now();
    run_supervisor(
        factory,
        repositories,
        job_tracker,
        shutdown_token,
        Duration::from_secs(10), // Very long backoff
    )
    .await;

    // Must exit shortly after the 25ms cancel, not wait for 10s
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn test_spawn_worker_supervisor_lifecycle_and_channel_processing() {
    let (state, rx) = AppState::test_state();
    let shutdown_token = state.shutdown_token();

    let supervisor_handle = spawn_worker_supervisor(
        state.repositories.clone(),
        state.job_tracker.clone(),
        rx,
        shutdown_token.clone(),
        DEFAULT_SUPERVISOR_BACKOFF,
    );

    // Send a command to verify channel works
    let job_id = JobId::new();
    let folder_id = FolderId::new();
    state
        .worker_sender
        .send(WorkerCommand::IndexFolder {
            job_id,
            folder_id,
            rescan: false,
        })
        .await
        .unwrap();

    // Allow worker to receive command
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Trigger shutdown
    shutdown_token.cancel();

    // Ensure supervisor terminates cleanly
    let result = tokio::time::timeout(Duration::from_secs(2), supervisor_handle).await;
    assert!(result.is_ok(), "Supervisor must join within timeout");
    assert!(result.unwrap().is_ok());
}
