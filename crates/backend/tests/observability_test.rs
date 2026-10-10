use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use backend::api::dtos::{HealthSummaryResponseDto, StatsResponseDto};
use backend::api::middlewares::X_REQUEST_ID;
use backend::domain::models::{Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus};
use backend::state::AppState;
use backend::telemetry::{cleanup_old_logs, get_log_directory, sanitize_secret_url};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

#[tokio::test]
async fn test_observability_telemetry_utilities() {
    // 1. Sanitize database URL secret
    let raw_db = "postgres://admin:super_secret_pwd@127.0.0.1:5432/lynxsearch";
    let sanitized = sanitize_secret_url(raw_db);
    assert_eq!(sanitized, "postgres://admin:***@127.0.0.1:5432/lynxsearch");

    // 2. Log directory path structure
    let log_dir = get_log_directory();
    assert!(log_dir.ends_with(Path::new("lynxsearch").join("logs")));

    // 3. Stale log rotation cleanup (older than 7 days)
    let temp = tempdir().unwrap();
    let old_log = temp.path().join("lynxsearch.log.2025-01-01");
    std::fs::write(&old_log, "stale log entry").unwrap();

    let removed = cleanup_old_logs(temp.path(), Duration::from_secs(0)).unwrap();
    assert_eq!(removed, 1);
    assert!(!old_log.exists());
}

#[tokio::test]
async fn test_request_spans_and_trace_headers() {
    let app = backend::create_router();

    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify x-request-id is propagated and returned in response
    let req_id = res.headers().get(&X_REQUEST_ID);
    assert!(
        req_id.is_some(),
        "Trace layer must ensure x-request-id is present"
    );
    let req_id_str = req_id.unwrap().to_str().unwrap();
    assert!(uuid::Uuid::parse_str(req_id_str).is_ok());

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let summary: HealthSummaryResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(summary.status == "ok" || summary.status == "degraded");
    assert!(!summary.version.is_empty());
    assert!(!summary.timestamp.is_empty());
}

#[tokio::test]
async fn test_metrics_api_stats_structure() {
    let app = backend::create_router();

    let req = Request::builder()
        .uri("/api/stats")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stats: StatsResponseDto = serde_json::from_slice(&body).unwrap();

    // Verify contract 13.4:
    // - total document count
    // - total/index size as contract defines
    // - distribution by type
    // - distribution by language
    // - indexed folders
    let _ = stats.total_documents;
    let _ = stats.total_size_bytes;
    let _ = stats.indexed_folders;
    let _ = stats.types;
    let _ = stats.languages;
}

#[tokio::test]
async fn test_job_span_and_panic_recovery_per_job() {
    let (state, worker_rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let job_id = JobId::new();

    // 1. Setup folder in scanning state
    let mut folder = Folder::new(PathBuf::from("/test/obs/folder"));
    folder.id = folder_id;
    folder.status = FolderStatus::Scanning;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 2. Setup job in running state
    let token = CancellationToken::new();
    state.job_tracker.register_job(job_id, folder_id, token);
    let job = IndexingJob {
        id: job_id,
        folder_id: Some(folder_id),
        job_type: Default::default(),
        status: JobStatus::Running,
        files_total: 10,
        files_processed: 2,
        files_indexed: 2,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    state.repositories.job.create_job(&job).await.unwrap();

    // Acquire lock in memory via try_lock_folder
    state.job_tracker.try_lock_folder(&folder_id).unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // Construct IndexOrchestrator to invoke recover_worker_panic directly
    let (worker_tx, _dummy_rx) = mpsc::channel(16);
    let orchestrator = backend::application::orchestrator::IndexOrchestrator::with_default_fs(
        state.repositories.clone(),
        state.job_tracker.clone(),
        worker_tx,
    );

    // 3. Trigger panic recovery
    let panic_msg = "Unexpected assertion failed in file processor";
    orchestrator
        .recover_worker_panic(job_id, Some(folder_id), panic_msg)
        .await;

    // 4. Verify in-memory state: job Failed, folder lock released
    let tracker_progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracker_progress.status, JobStatus::Failed);
    assert!(tracker_progress.error_summary.unwrap().contains(panic_msg));
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 5. Verify database state: job Failed, folder Idle
    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Failed);
    assert!(db_job.error_summary.unwrap().contains(panic_msg));

    let db_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_folder.status, FolderStatus::Idle);

    // Drop unused receiver to keep test clean
    drop(worker_rx);
}
