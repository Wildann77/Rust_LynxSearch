use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::create_router_with_state;
use backend::domain::models::{
    Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus, JobType,
};
use backend::error::ErrorCode;
use backend::state::AppState;
use chrono::Utc;
use std::fs;
use tempfile::tempdir;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_cancel_api_endpoint_success() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/cancel_api_test".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Verify folder locked
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // Send POST /api/index/jobs/:id/cancel
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json_resp["job_id"], job_id.to_string());
    assert_eq!(json_resp["status"], "CANCELLED");
    assert_eq!(json_resp["message"], "Job cancellation signal sent.");

    // Verify in-memory tracker
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Cancelled);
    assert!(progress.completed_at.is_some());

    // Verify PostgreSQL/in-memory persisted state
    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Cancelled);
    assert!(db_job.completed_at.is_some());

    // Verify folder lock released immediately for pending job
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // Drain queued command
    let _ = rx.recv().await.unwrap();
}

#[tokio::test]
async fn test_cancel_api_not_found_returns_404() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let non_existent_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{non_existent_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json_resp["code"], "JOB_NOT_FOUND");
    assert!(
        json_resp["message"]
            .as_str()
            .unwrap()
            .contains(&non_existent_id.to_string())
    );
}

#[tokio::test]
async fn test_cancel_api_terminal_job_returns_409_conflict() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let job_id = JobId::new();
    let completed_job = IndexingJob {
        id: job_id,
        folder_id: None,
        job_type: JobType::Import,
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
    state
        .repositories
        .job
        .create_job(&completed_job)
        .await
        .unwrap();

    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json_resp["code"], ErrorCode::JobConflict.as_str());
    assert!(json_resp["message"].as_str().unwrap().contains("COMPLETED"));
}

#[tokio::test]
async fn test_cancel_api_signals_cancellation_token() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/token_test".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    let token = state.job_tracker.get_cancellation_token(&job_id).unwrap();
    assert!(!token.is_cancelled());

    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    assert!(token.is_cancelled());

    let _ = rx.recv().await.unwrap();
}

struct CancelOnJobStarted {
    sender: tokio::sync::mpsc::Sender<()>,
}

impl backend::domain::events::DomainEventHandler for CancelOnJobStarted {
    fn handle(&self, event: &backend::domain::events::DomainEvent) {
        if matches!(
            event,
            backend::domain::events::DomainEvent::IndexingJobStarted(_)
        ) {
            let _ = self.sender.try_send(());
        }
    }
}

#[tokio::test]
async fn test_cancel_api_stops_acquisition_and_finishes_active_batch_within_grace() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    let (started_tx, mut started_rx) = tokio::sync::mpsc::channel(1);
    orchestrator
        .event_dispatcher()
        .register(std::sync::Arc::new(CancelOnJobStarted {
            sender: started_tx,
        }));

    let temp_dir = tempdir().unwrap();
    let root = temp_dir.path();

    // Create 30 files
    for i in 0..30 {
        fs::write(
            root.join(format!("file_{i:02}.rs")),
            format!("pub fn fn_{i}() -> i32 {{ {i} }}\n"),
        )
        .unwrap();
    }

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: root.to_path_buf(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Pop the command from the channel and dispatch in background
    let command = rx.recv().await.unwrap();
    let orch_clone = orchestrator.clone();
    let worker_handle = tokio::spawn(async move { orch_clone.dispatch(command).await });

    // Wait until IndexingJobStarted has fired
    started_rx.recv().await.unwrap();

    // Send cancel request via HTTP API while execution is active
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Worker should finish active batch within grace and exit cleanly
    let dispatch_res = worker_handle.await.unwrap();
    assert!(dispatch_res.is_ok());

    // Verify job persisted as CANCELLED
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Cancelled);
    assert!(progress.completed_at.is_some());

    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Cancelled);
    assert!(db_job.completed_at.is_some());

    // Verify folder lock released and folder status Idle
    assert!(!state.job_tracker.is_folder_locked(&folder_id));
    let folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(folder.status, FolderStatus::Idle);
}
