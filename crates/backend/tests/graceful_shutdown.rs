use backend::domain::models::{Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus};
use backend::{
    AppState, SHUTDOWN_CANCEL_REASON, create_router_with_state, shutdown_in_flight_jobs,
};
use chrono::Utc;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_graceful_shutdown_cancels_in_flight_jobs_and_resets_folders() {
    let (state, mut rx) = AppState::test_state();
    let folder_id = FolderId::new();

    let mut folder = Folder::new(PathBuf::from("/test/graceful_shutdown"));
    folder.id = folder_id;
    folder.status = FolderStatus::Scanning;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let running_job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        job_type: Default::default(),
        status: JobStatus::Running,
        files_total: 100,
        files_processed: 30,
        files_indexed: 28,
        files_skipped: 2,
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
    state
        .repositories
        .job
        .create_job(&pending_job)
        .await
        .unwrap();

    let completed_job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        job_type: Default::default(),
        status: JobStatus::Completed,
        files_total: 50,
        files_processed: 50,
        files_indexed: 50,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
    };
    state
        .repositories
        .job
        .create_job(&completed_job)
        .await
        .unwrap();

    let failed_job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        job_type: Default::default(),
        status: JobStatus::Failed,
        files_total: 10,
        files_processed: 2,
        files_indexed: 1,
        files_skipped: 0,
        files_failed: 1,
        error_summary: Some("disk full".into()),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
    };
    state
        .repositories
        .job
        .create_job(&failed_job)
        .await
        .unwrap();

    // Register active job in job_tracker as well
    state
        .job_tracker
        .register_job(running_job.id, folder_id, CancellationToken::new());

    let shutdown_token = state.shutdown_token();
    let worker_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    break;
                }
                cmd_opt = rx.recv() => {
                    if cmd_opt.is_none() {
                        break;
                    }
                }
            }
        }
    });

    let report = state
        .graceful_shutdown(Some(worker_handle), Duration::from_millis(500))
        .await
        .unwrap();

    assert_eq!(report.jobs_cancelled, 2);
    assert_eq!(report.folders_reset, 1);
    assert!(!report.grace_period_timed_out);

    // Verify database / repository state
    let healed_running = state
        .repositories
        .job
        .get_job(&running_job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(healed_running.status, JobStatus::Cancelled);
    assert_eq!(
        healed_running.error_summary.as_deref(),
        Some(SHUTDOWN_CANCEL_REASON)
    );
    assert!(healed_running.completed_at.is_some());

    let healed_pending = state
        .repositories
        .job
        .get_job(&pending_job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(healed_pending.status, JobStatus::Cancelled);
    assert_eq!(
        healed_pending.error_summary.as_deref(),
        Some(SHUTDOWN_CANCEL_REASON)
    );
    assert!(healed_pending.completed_at.is_some());

    let untouched_completed = state
        .repositories
        .job
        .get_job(&completed_job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched_completed.status, JobStatus::Completed);

    let untouched_failed = state
        .repositories
        .job
        .get_job(&failed_job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched_failed.status, JobStatus::Failed);

    let healed_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(healed_folder.status, FolderStatus::Idle);

    assert!(state.job_tracker.is_shutting_down());
    assert!(state.db_pool.is_closed());

    // Idempotency: second shutdown returns 0 cancelled, 0 reset
    let second_report = shutdown_in_flight_jobs(&state.repositories, &state.job_tracker)
        .await
        .unwrap();
    assert_eq!(second_report.0, 0);
    assert_eq!(second_report.1, 0);
}

#[tokio::test]
async fn test_graceful_shutdown_grace_period_timeout() {
    let (state, _rx) = AppState::test_state();

    let stuck_worker = tokio::spawn(async move {
        // Simulates a worker that takes longer than the grace period
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    let report = state
        .graceful_shutdown(Some(stuck_worker), Duration::from_millis(50))
        .await
        .unwrap();

    assert!(report.grace_period_timed_out);
    assert!(state.db_pool.is_closed());
}

#[tokio::test]
async fn test_axum_server_graceful_drain_and_shutdown() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    // Make an active HTTP request over real TCP socket to ensure server is accepting
    let mut stream = tokio::net::TcpStream::connect(local_addr).await.unwrap();
    stream
        .write_all(b"GET /api/health/live HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.starts_with("HTTP/1.1 200 OK"));

    // Trigger graceful shutdown of Axum server
    let _ = shutdown_tx.send(());

    // Server should exit gracefully
    tokio::time::timeout(Duration::from_secs(2), server_handle)
        .await
        .expect("Server did not shut down within timeout")
        .unwrap();

    // Now execute AppState graceful shutdown
    let report = state
        .graceful_shutdown::<()>(None, Duration::from_secs(1))
        .await
        .unwrap();

    assert_eq!(report.jobs_cancelled, 0);
    assert_eq!(report.folders_reset, 0);
    assert!(state.db_pool.is_closed());
}
