use backend::application::orchestrator::{
    DEFAULT_SUPERVISOR_BACKOFF, WorkerCommand, spawn_orchestrator_supervisor,
};
use backend::domain::models::{Folder, FolderId, FolderStatus, JobStatus};
use backend::error::AppError;
use backend::state::AppState;
use std::time::Duration;

#[tokio::test]
async fn test_submit_index_folder_and_dispatch_lifecycle() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();
    let temp_dir = tempfile::tempdir().unwrap();

    // 1. Create a registered folder
    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: temp_dir.path().to_path_buf(),
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

    // 2. Submit index folder job
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Verify job in JobTracker is Pending
    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Pending);

    // Verify job in Repository is Pending
    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Pending);

    // 3. Receive command from worker channel
    let command = rx.recv().await.unwrap();
    assert_eq!(
        command,
        WorkerCommand::IndexFolder {
            job_id,
            folder_id,
            rescan: false,
        }
    );

    // 4. Dispatch the command
    orchestrator.dispatch(command).await.unwrap();

    // 5. Verify terminal state: Job Completed, Folder Idle, Lock Released
    let finished_progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(finished_progress.status, JobStatus::Completed);
    assert!(finished_progress.completed_at.is_some());

    let finished_db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(finished_db_job.status, JobStatus::Completed);

    let finished_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(finished_folder.status, FolderStatus::Idle);

    assert!(!state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_folder_lock_conflict_fails_fast_on_submit() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: "/test/locked".into(),
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

    // Acquire folder lock to simulate an active running scan
    let lock_guard = state.job_tracker.try_acquire_folder_lock(&folder_id);
    assert!(lock_guard.is_ok());

    // Submit attempt on locked folder must fail fast with JobConflict
    let err = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap_err();
    assert!(
        matches!(err, AppError::JobConflict(_)),
        "Expected JobConflict on locked folder, got: {err:?}"
    );

    drop(lock_guard);

    // After dropping lock, submit succeeds
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    assert_eq!(
        state.job_tracker.get_progress(&job_id).unwrap().status,
        JobStatus::Pending
    );
}

#[tokio::test]
async fn test_rebuild_lock_blocks_folder_scans_and_vice_versa() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: "/test/rebuild_conflict".into(),
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

    // Rebuild lock acquired
    let rebuild_guard = state.job_tracker.try_acquire_rebuild_lock().unwrap();

    // Submitting folder scan must fail with JobConflict
    let scan_err = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap_err();
    assert!(matches!(scan_err, AppError::JobConflict(_)));

    // Submitting another rebuild must also fail with JobConflict
    let rebuild_err = orchestrator.submit_rebuild_index().await.unwrap_err();
    assert!(matches!(rebuild_err, AppError::JobConflict(_)));

    drop(rebuild_guard);

    // After rebuild ends, folder scan submission succeeds
    let job_id = orchestrator.submit_index_folder(folder_id, false).await;
    assert!(job_id.is_ok());
}

#[tokio::test]
async fn test_concurrent_folder_scans_independent() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_a = FolderId::new();
    let folder_b = FolderId::new();
    let temp_dir_a = tempfile::tempdir().unwrap();
    let temp_dir_b = tempfile::tempdir().unwrap();

    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_a,
            path: temp_dir_a.path().to_path_buf(),
            status: FolderStatus::Idle,
            created_at: chrono::Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_b,
            path: temp_dir_b.path().to_path_buf(),
            status: FolderStatus::Idle,
            created_at: chrono::Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    // Submit both folder jobs concurrently
    let job_a = orchestrator
        .submit_index_folder(folder_a, false)
        .await
        .unwrap();
    let job_b = orchestrator
        .submit_index_folder(folder_b, false)
        .await
        .unwrap();

    assert_ne!(job_a, job_b);

    // Receive and dispatch both
    let cmd1 = rx.recv().await.unwrap();
    let cmd2 = rx.recv().await.unwrap();

    orchestrator.dispatch(cmd1).await.unwrap();
    orchestrator.dispatch(cmd2).await.unwrap();

    assert_eq!(
        state.job_tracker.get_progress(&job_a).unwrap().status,
        JobStatus::Completed
    );
    assert_eq!(
        state.job_tracker.get_progress(&job_b).unwrap().status,
        JobStatus::Completed
    );
}

#[tokio::test]
async fn test_cancel_job_flow() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/test/cancel".into(),
            status: FolderStatus::Idle,
            created_at: chrono::Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Cancel before execution
    orchestrator.submit_cancel_job(job_id).await.unwrap();

    assert_eq!(
        state.job_tracker.get_progress(&job_id).unwrap().status,
        JobStatus::Cancelled
    );
    assert_eq!(
        state
            .repositories
            .job
            .get_job(&job_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Cancelled
    );

    // Drain queued command
    let cmd = rx.recv().await.unwrap();
    // Dispatching cancelled job should be a no-op
    orchestrator.dispatch(cmd).await.unwrap();

    assert_eq!(
        state.job_tracker.get_progress(&job_id).unwrap().status,
        JobStatus::Cancelled
    );
}

#[tokio::test]
async fn test_spawn_orchestrator_supervisor_processes_jobs_automatically() {
    let (state, rx) = AppState::test_state();
    let shutdown_token = state.shutdown_token();

    let supervisor_handle = spawn_orchestrator_supervisor(
        state.orchestrator().clone(),
        rx,
        shutdown_token.clone(),
        DEFAULT_SUPERVISOR_BACKOFF,
    );

    let folder_id = FolderId::new();
    let temp_dir = tempfile::tempdir().unwrap();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: temp_dir.path().to_path_buf(),
            status: FolderStatus::Idle,
            created_at: chrono::Utc::now(),
            last_scanned_at: None,
        })
        .await
        .unwrap();

    let job_id = state
        .orchestrator()
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Wait briefly for background supervisor worker to process the job
    tokio::time::sleep(Duration::from_millis(50)).await;

    let progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(progress.status, JobStatus::Completed);

    // Clean shutdown
    shutdown_token.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), supervisor_handle).await;
}
