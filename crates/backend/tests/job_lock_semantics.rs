use axum::http::StatusCode;
use axum::response::IntoResponse;
use backend::application::orchestrator::supervisor::{
    WORKER_PANIC_ERROR_MSG, recover_panicked_worker,
};
use backend::domain::models::{Folder, FolderId, FolderStatus, JobStatus};
use backend::error::{AppError, ErrorCode};
use backend::state::AppState;
use chrono::Utc;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn test_only_one_indexing_job_active_per_folder_second_trigger_conflict() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_a = FolderId::new();
    let folder_b = FolderId::new();

    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_a,
            path: "/workspace/folder_a".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_b,
            path: "/workspace/folder_b".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. First trigger for folder_a succeeds
    let job_a1 = orchestrator
        .submit_index_folder(folder_a, false)
        .await
        .unwrap();
    assert_eq!(
        state.job_tracker.get_progress(&job_a1).unwrap().status,
        JobStatus::Pending
    );
    assert!(state.job_tracker.is_folder_locked(&folder_a));

    // 2. Second trigger for folder_a returns 409 JOB_CONFLICT
    let err = orchestrator
        .submit_index_folder(folder_a, false)
        .await
        .unwrap_err();

    assert_eq!(err.status_code(), StatusCode::CONFLICT);
    assert_eq!(err.error_code(), ErrorCode::JobConflict);
    match err {
        AppError::JobConflict(conflict_folder_id) => {
            assert_eq!(conflict_folder_id, *folder_a);
        }
        other => panic!("Expected JobConflict, got: {other:?}"),
    }

    // Verify HTTP response mapping conforms to ARCHITECTURE.md contract
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // 3. Different folder (folder_b) can still be triggered concurrently
    let job_b = orchestrator
        .submit_index_folder(folder_b, false)
        .await
        .unwrap();
    assert_ne!(job_a1, job_b);
    assert!(state.job_tracker.is_folder_locked(&folder_b));

    // Drain queue commands
    let _ = rx.recv().await.unwrap();
    let _ = rx.recv().await.unwrap();
}

#[tokio::test]
async fn test_lock_released_on_completed() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("main.rs");
    fs::write(&test_file, "fn main() { println!(\"LynxSearch\"); }").unwrap();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: temp_dir.path().to_path_buf(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Submit scan job
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // 2. Dispatch worker command to execute scan until completion
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    // 3. Verify job completed, folder idle, and lock released
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Completed);
    let folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(folder.status, FolderStatus::Idle);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 4. Submitting a new scan for the same folder now succeeds without conflict
    let second_job = orchestrator
        .submit_index_folder(folder_id, true)
        .await
        .unwrap();
    assert_ne!(job_id, second_job);
    assert!(state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_lock_released_on_failed() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let non_existent_path = std::env::temp_dir().join(format!("lynx_missing_{}", Uuid::new_v4()));
    let folder_id = FolderId::new();

    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: non_existent_path.clone(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Submit scan job
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // 2. Dispatch command -> Fails gracefully because folder directory does not exist on disk
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    // 3. Verify job failed, folder reset to idle, and lock released
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Failed);
    assert!(progress.error_summary.is_some());

    let folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(folder.status, FolderStatus::Idle);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 4. Fix folder directory on disk and verify new submission succeeds
    fs::create_dir_all(&non_existent_path).unwrap();
    let retry_job = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert_ne!(job_id, retry_job);
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    let _ = fs::remove_dir_all(&non_existent_path);
}

#[tokio::test]
async fn test_lock_released_on_cancelled_pending() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/cancel_pending".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Submit scan job (remains Pending in queue)
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // 2. Cancel while pending
    orchestrator.submit_cancel_job(job_id).await.unwrap();

    // 3. Verify job cancelled and lock released immediately
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Cancelled);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 4. Submitting a new job for the folder succeeds immediately
    let new_job = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert_ne!(job_id, new_job);
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // Drain queued commands
    let cmd1 = rx.recv().await.unwrap();
    // Dispatching cancelled command is a safe no-op
    orchestrator.dispatch(cmd1).await.unwrap();
}

#[tokio::test]
async fn test_lock_released_on_worker_panic() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/panic_recovery".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Submit scan job -> Folder is locked
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // 2. Simulate worker crash/panic recovery
    let (jobs_failed, folders_reset) =
        recover_panicked_worker(&state.repositories, &state.job_tracker)
            .await
            .unwrap();

    assert_eq!(jobs_failed, 1);
    assert_eq!(folders_reset, 0); // was pending

    // 3. Verify job failed, folder idle, and lock released
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Failed);
    assert_eq!(
        progress.error_summary,
        Some(WORKER_PANIC_ERROR_MSG.to_string())
    );
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 4. Triggering another scan on the folder succeeds
    let recovered_job = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert_ne!(job_id, recovered_job);
    assert!(state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_rebuild_uses_global_lock_and_rejects_new_scans() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/rebuild_lock_test".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Submit rebuild job -> Global rebuild lock acquired
    let (rebuild_job, _target_index) = orchestrator.submit_rebuild_index().await.unwrap();
    assert!(state.job_tracker.is_rebuilding());

    // 2. Second rebuild trigger fails with 409 JOB_CONFLICT
    let rebuild_err = orchestrator.submit_rebuild_index().await.unwrap_err();
    assert_eq!(rebuild_err.status_code(), StatusCode::CONFLICT);
    assert_eq!(rebuild_err.error_code(), ErrorCode::JobConflict);

    // 3. New scan job for ANY folder is rejected with 409 JOB_CONFLICT
    let scan_err = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap_err();
    assert_eq!(scan_err.status_code(), StatusCode::CONFLICT);
    assert_eq!(scan_err.error_code(), ErrorCode::JobConflict);

    // 4. Dispatch rebuild job to completion
    let rebuild_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(rebuild_cmd).await.unwrap();

    let rebuild_progress = state.job_tracker.get_progress(&rebuild_job).unwrap();
    assert_eq!(rebuild_progress.status, JobStatus::Completed);
    assert!(!state.job_tracker.is_rebuilding());

    // 5. After rebuild completes, new folder scan succeeds
    let scan_job = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.get_progress(&scan_job).is_some());
    assert!(state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_active_scan_blocks_rebuild() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/scan_blocks_rebuild".into(),
            status: FolderStatus::Idle,
            created_at: Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // 1. Active scan job submitted
    let scan_job = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert!(state.job_tracker.is_folder_locked(&folder_id));

    // 2. Rebuild index fails with 409 JOB_CONFLICT
    let rebuild_err = orchestrator.submit_rebuild_index().await.unwrap_err();
    assert_eq!(rebuild_err.status_code(), StatusCode::CONFLICT);
    assert_eq!(rebuild_err.error_code(), ErrorCode::JobConflict);

    // 3. Cancel scan job -> releases folder lock
    orchestrator.submit_cancel_job(scan_job).await.unwrap();
    assert!(!state.job_tracker.is_folder_locked(&folder_id));

    // 4. Rebuild submission now succeeds
    let (rebuild_job, _target_index) = orchestrator.submit_rebuild_index().await.unwrap();
    assert!(state.job_tracker.get_progress(&rebuild_job).is_some());
    assert!(state.job_tracker.is_rebuilding());

    // Drain queued commands
    let _ = rx.recv().await.unwrap();
    let _ = rx.recv().await.unwrap();
}

#[tokio::test]
async fn test_rebuild_lock_released_on_worker_panic() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    // 1. Submit rebuild job -> Rebuild lock held
    let (rebuild_job, _target_index) = orchestrator.submit_rebuild_index().await.unwrap();
    assert!(state.job_tracker.is_rebuilding());

    // 2. Worker panics during rebuild
    let (jobs_failed, _) = recover_panicked_worker(&state.repositories, &state.job_tracker)
        .await
        .unwrap();
    assert_eq!(jobs_failed, 1);

    // 3. Verify rebuild lock released
    assert!(!state.job_tracker.is_rebuilding());
    let progress = state.job_tracker.get_progress(&rebuild_job).unwrap();
    assert_eq!(progress.status, JobStatus::Failed);

    // 4. New rebuild can be submitted without conflict
    let (new_rebuild, _new_index) = orchestrator.submit_rebuild_index().await.unwrap();
    assert_ne!(rebuild_job, new_rebuild);
    assert!(state.job_tracker.is_rebuilding());
}
