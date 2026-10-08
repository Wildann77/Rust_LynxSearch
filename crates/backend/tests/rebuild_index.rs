use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::routes::create_router_with_state;
use backend::domain::models::{
    DocumentId, DocumentStatus, Folder, FolderId, FolderStatus, JobStatus,
};
use backend::error::{AppError, ErrorCode};
use backend::state::AppState;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn test_rebuild_index_full_lifecycle_and_zero_downtime() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    // 1. Initial setup: create folder and initial documents on disk
    let temp = tempdir().unwrap();
    let root = temp.path();

    for i in 0..250 {
        let file_path = root.join(format!("doc_{i:03}.md"));
        fs::write(
            &file_path,
            format!(
                "# Document {i}\nContent for doc {i} with searchable identifier indexable_{i}."
            ),
        )
        .unwrap();
    }

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Initial folder scan to establish v1 index with 250 documents
    let scan_job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let scan_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(scan_cmd).await.unwrap();

    assert_eq!(
        state.job_tracker.get_progress(&scan_job_id).unwrap().status,
        JobStatus::Completed
    );

    // Bootstrap initial alias and verify active index is v1
    let initial_index = state
        .repositories
        .search
        .ensure_initial_index()
        .await
        .unwrap();
    assert_eq!(initial_index, "lynx_documents_v1");

    // Add an excluded document in registry to verify it is NOT streamed during rebuild
    let excluded_doc_id = DocumentId::from_relative_path(folder_id, "excluded.md");
    state
        .repositories
        .registry
        .upsert_entry(&backend::domain::models::RegistryEntry {
            id: excluded_doc_id,
            folder_id,
            relative_path: "excluded.md".into(),
            content_hash: "hash_excluded".into(),
            file_size: 100,
            status: DocumentStatus::Excluded,
            status_reason: Some("Manual exclusion test".into()),
            indexed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // 2. Trigger rebuild via POST /api/index/rebuild
    let req = Request::builder()
        .uri("/api/index/rebuild")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    let rebuild_job_uuid = body_json["job_id"].as_str().unwrap();
    let target_index = body_json["target_index"].as_str().unwrap();
    let status_str = body_json["status"].as_str().unwrap();

    assert_eq!(status_str, "RUNNING");
    assert_eq!(target_index, "lynx_documents_v2");

    let rebuild_job_id =
        backend::domain::models::JobId::from_uuid(uuid::Uuid::parse_str(rebuild_job_uuid).unwrap());

    // 3. Verify global rebuild lock acquired immediately
    assert!(state.job_tracker.is_rebuilding());

    // Concurrent scan attempt fails with 409 Conflict
    let conflict_scan = orchestrator.submit_index_folder(folder_id, false).await;
    assert!(matches!(conflict_scan, Err(AppError::JobConflict(_))));

    // Concurrent rebuild attempt fails with 409 Conflict
    let conflict_rebuild = orchestrator.submit_rebuild_index().await;
    assert!(matches!(conflict_rebuild, Err(AppError::JobConflict(_))));

    // 4. Dispatch rebuild command from worker queue
    let rebuild_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(rebuild_cmd).await.unwrap();

    // 5. Verify rebuild completed successfully
    let rebuild_progress = state
        .job_tracker
        .get_progress(&rebuild_job_id)
        .expect("Rebuild progress should exist");
    assert_eq!(rebuild_progress.status, JobStatus::Completed);
    assert_eq!(rebuild_progress.files_total, 250);
    assert_eq!(rebuild_progress.files_indexed, 250);
    assert_eq!(rebuild_progress.files_skipped, 0);
    assert_eq!(rebuild_progress.files_failed, 0);

    let db_job = state
        .repositories
        .job
        .get_job(&rebuild_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Completed);
    assert_eq!(db_job.files_indexed, 250);

    // 6. Verify alias swap and verification: alias now points to lynx_documents_v2
    let active_after = state
        .repositories
        .search
        .get_active_physical_index()
        .await
        .unwrap();
    assert_eq!(active_after.as_deref(), Some("lynx_documents_v2"));

    let current_alias_indices = state
        .repositories
        .search
        .get_alias_indices("lynx_documents")
        .await
        .unwrap();
    assert_eq!(current_alias_indices, vec!["lynx_documents_v2".to_string()]);

    // 7. Verify old index v1 removed
    // (In InMemorySearchRepository, delete_index removed lynx_documents_v1)
    assert!(!current_alias_indices.contains(&"lynx_documents_v1".to_string()));

    // 8. Verify global rebuild lock released
    assert!(!state.job_tracker.is_rebuilding());

    // Subsequent folder scan now succeeds
    let post_rebuild_scan = orchestrator.submit_index_folder(folder_id, false).await;
    assert!(post_rebuild_scan.is_ok());
}

#[tokio::test]
async fn test_rebuild_cancelled_during_streaming_preserves_old_alias() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    for i in 0..10 {
        let file_path = root.join(format!("file_{i}.txt"));
        fs::write(&file_path, format!("Content {i}")).unwrap();
    }

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Populate initial index
    let _scan_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    state
        .repositories
        .search
        .ensure_initial_index()
        .await
        .unwrap();

    // Submit rebuild
    let (rebuild_job, _target_index) = orchestrator.submit_rebuild_index().await.unwrap();
    assert!(state.job_tracker.is_rebuilding());

    // Signal cancellation before/during execution
    state.job_tracker.cancel_job(&rebuild_job);

    let rebuild_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(rebuild_cmd).await.unwrap();

    // Rebuild lock released
    assert!(!state.job_tracker.is_rebuilding());

    // Old alias still points to v1
    let active = state
        .repositories
        .search
        .get_active_physical_index()
        .await
        .unwrap();
    assert_eq!(active.as_deref(), Some("lynx_documents_v1"));

    // Job marked Cancelled
    let progress = state.job_tracker.get_progress(&rebuild_job).unwrap();
    assert_eq!(progress.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn test_rebuild_lock_conflict_errors() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    // 1. Holding rebuild lock blocks new rebuild
    let guard = state.job_tracker.try_acquire_rebuild_lock().unwrap();
    let err = orchestrator.submit_rebuild_index().await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::CONFLICT);
    assert_eq!(err.error_code(), ErrorCode::JobConflict);

    drop(guard);

    // 2. Lock is clean after guard dropped
    let res = orchestrator.submit_rebuild_index().await;
    assert!(res.is_ok());
}
