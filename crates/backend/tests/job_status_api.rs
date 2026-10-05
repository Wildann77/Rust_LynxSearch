use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::create_router_with_state;
use backend::domain::models::{
    FolderId, IndexingJob, JobId, JobProgressUpdate, JobStatus, JobType,
};
use backend::error::ErrorCode;
use backend::state::AppState;
use chrono::Utc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_job_status_api_active_job_from_memory() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let job_id = JobId::new();
    let folder_id = FolderId::new();

    state.job_tracker.register_job(
        job_id,
        folder_id,
        tokio_util::sync::CancellationToken::new(),
    );

    state.job_tracker.update_progress(
        &job_id,
        JobProgressUpdate {
            status: Some(JobStatus::Running),
            files_total: Some(50),
            processed_delta: 25,
            indexed_delta: 20,
            skipped_delta: 3,
            failed_delta: 2,
            error_summary: None,
        },
    );

    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["job_id"], job_id.to_string());
    assert_eq!(json["folder_id"], folder_id.to_string());
    assert_eq!(json["status"], "RUNNING");
    assert_eq!(json["total_files"], 50);
    assert_eq!(json["processed_files"], 25);
    assert_eq!(json["indexed_files"], 20);
    assert_eq!(json["skipped_files"], 3);
    assert_eq!(json["failed_files"], 2);
    assert!(json["error"].is_null());
    assert!(json["started_at"].is_string());
    assert!(json["finished_at"].is_null());
}

#[tokio::test]
async fn test_job_status_api_terminal_job_with_error() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let job_id = JobId::new();
    let folder_id = FolderId::new();

    state.job_tracker.register_job(
        job_id,
        folder_id,
        tokio_util::sync::CancellationToken::new(),
    );

    state.job_tracker.update_progress(
        &job_id,
        JobProgressUpdate {
            status: Some(JobStatus::Failed),
            files_total: Some(10),
            processed_delta: 5,
            indexed_delta: 3,
            skipped_delta: 0,
            failed_delta: 2,
            error_summary: Some("I/O error during file read".to_string()),
        },
    );

    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["job_id"], job_id.to_string());
    assert_eq!(json["status"], "FAILED");
    assert_eq!(json["error"], "I/O error during file read");
    assert!(json["finished_at"].is_string());
}

#[tokio::test]
async fn test_job_status_api_fallback_to_persistent_storage() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let job_id = JobId::new();
    let folder_id = FolderId::new();
    let now = Utc::now();

    let job = IndexingJob {
        id: job_id,
        folder_id: Some(folder_id),
        job_type: JobType::Rescan,
        status: JobStatus::Completed,
        files_total: 100,
        files_processed: 100,
        files_indexed: 90,
        files_skipped: 10,
        files_failed: 0,
        error_summary: None,
        started_at: now,
        completed_at: Some(now),
    };
    state.repositories.job.create_job(&job).await.unwrap();

    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["job_id"], job_id.to_string());
    assert_eq!(json["folder_id"], folder_id.to_string());
    assert_eq!(json["status"], "COMPLETED");
    assert_eq!(json["total_files"], 100);
    assert_eq!(json["processed_files"], 100);
    assert_eq!(json["indexed_files"], 90);
    assert_eq!(json["skipped_files"], 10);
    assert_eq!(json["failed_files"], 0);
    assert!(json["error"].is_null());
    assert!(json["finished_at"].is_string());
}

#[tokio::test]
async fn test_job_status_api_not_found() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let missing_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{missing_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["code"], ErrorCode::JobNotFound.as_str());
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains(&missing_id.to_string())
    );
}

#[tokio::test]
async fn test_job_status_api_invalid_uuid() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let req = Request::builder()
        .uri("/api/index/jobs/invalid-not-a-uuid")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["code"], ErrorCode::ValidationFailed.as_str());
}
