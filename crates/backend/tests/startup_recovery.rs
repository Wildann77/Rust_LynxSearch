use backend::AppState;
use backend::application::{STARTUP_CRASH_RECOVERY_ERROR_MSG, recover_on_startup};
use backend::domain::models::{Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus};
use backend::state::Repositories;
use chrono::Utc;
use std::path::PathBuf;

#[tokio::test]
async fn test_startup_recovery_heals_running_and_pending_jobs() {
    let repos = Repositories::in_memory();
    let folder_id = FolderId::new();

    // 1. Setup jobs with different statuses
    let running_job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        status: JobStatus::Running,
        files_total: 100,
        files_processed: 45,
        files_indexed: 40,
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
    repos.job.create_job(&completed_job).await.unwrap();

    let failed_job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        status: JobStatus::Failed,
        files_total: 20,
        files_processed: 5,
        files_indexed: 5,
        files_skipped: 0,
        files_failed: 1,
        error_summary: Some("Prior failure".to_string()),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
    };
    repos.job.create_job(&failed_job).await.unwrap();

    // 2. Execute startup recovery
    let report = recover_on_startup(&repos).await.unwrap();
    assert_eq!(report.jobs_recovered, 2);
    assert_eq!(report.folders_reset, 0);

    // 3. Verify running job was healed
    let healed_running = repos.job.get_job(&running_job.id).await.unwrap().unwrap();
    assert_eq!(healed_running.status, JobStatus::Failed);
    assert!(healed_running.completed_at.is_some());
    assert_eq!(
        healed_running.error_summary.as_deref(),
        Some(STARTUP_CRASH_RECOVERY_ERROR_MSG)
    );
    assert_eq!(healed_running.files_processed, 45); // preserves counters

    // 4. Verify pending job was healed
    let healed_pending = repos.job.get_job(&pending_job.id).await.unwrap().unwrap();
    assert_eq!(healed_pending.status, JobStatus::Failed);
    assert!(healed_pending.completed_at.is_some());
    assert_eq!(
        healed_pending.error_summary.as_deref(),
        Some(STARTUP_CRASH_RECOVERY_ERROR_MSG)
    );

    // 5. Verify completed and failed jobs were untouched
    let untouched_completed = repos.job.get_job(&completed_job.id).await.unwrap().unwrap();
    assert_eq!(untouched_completed.status, JobStatus::Completed);
    assert_eq!(untouched_completed.error_summary, None);

    let untouched_failed = repos.job.get_job(&failed_job.id).await.unwrap().unwrap();
    assert_eq!(untouched_failed.status, JobStatus::Failed);
    assert_eq!(
        untouched_failed.error_summary.as_deref(),
        Some("Prior failure")
    );
}

#[tokio::test]
async fn test_startup_recovery_resets_scanning_folders() {
    let repos = Repositories::in_memory();

    let mut scanning_folder_1 = Folder::new(PathBuf::from("/path/scanning1"));
    scanning_folder_1.status = FolderStatus::Scanning;
    repos
        .folder
        .create_folder(&scanning_folder_1)
        .await
        .unwrap();

    let mut scanning_folder_2 = Folder::new(PathBuf::from("/path/scanning2"));
    scanning_folder_2.status = FolderStatus::Scanning;
    repos
        .folder
        .create_folder(&scanning_folder_2)
        .await
        .unwrap();

    let mut idle_folder = Folder::new(PathBuf::from("/path/idle"));
    idle_folder.status = FolderStatus::Idle;
    repos.folder.create_folder(&idle_folder).await.unwrap();

    let mut error_folder = Folder::new(PathBuf::from("/path/error"));
    error_folder.status = FolderStatus::Error;
    repos.folder.create_folder(&error_folder).await.unwrap();

    let report = recover_on_startup(&repos).await.unwrap();
    assert_eq!(report.jobs_recovered, 0);
    assert_eq!(report.folders_reset, 2);

    let f1 = repos
        .folder
        .get_folder(&scanning_folder_1.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f1.status, FolderStatus::Idle);

    let f2 = repos
        .folder
        .get_folder(&scanning_folder_2.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f2.status, FolderStatus::Idle);

    let f_idle = repos
        .folder
        .get_folder(&idle_folder.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f_idle.status, FolderStatus::Idle);

    let f_err = repos
        .folder
        .get_folder(&error_folder.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f_err.status, FolderStatus::Error);
}

#[tokio::test]
async fn test_startup_recovery_is_idempotent() {
    let repos = Repositories::in_memory();
    let folder_id = FolderId::new();

    let mut folder = Folder::new(PathBuf::from("/path/scan_test"));
    folder.id = folder_id;
    folder.status = FolderStatus::Scanning;
    repos.folder.create_folder(&folder).await.unwrap();

    let job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder_id),
        status: JobStatus::Running,
        files_total: 50,
        files_processed: 10,
        files_indexed: 10,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    repos.job.create_job(&job).await.unwrap();

    // First recovery pass
    let first_report = recover_on_startup(&repos).await.unwrap();
    assert_eq!(first_report.jobs_recovered, 1);
    assert_eq!(first_report.folders_reset, 1);

    // Second recovery pass: must be completely idempotent
    let second_report = recover_on_startup(&repos).await.unwrap();
    assert_eq!(second_report.jobs_recovered, 0);
    assert_eq!(second_report.folders_reset, 0);

    // Third recovery pass: still 0
    let third_report = recover_on_startup(&repos).await.unwrap();
    assert_eq!(third_report.jobs_recovered, 0);
    assert_eq!(third_report.folders_reset, 0);

    let final_job = repos.job.get_job(&job.id).await.unwrap().unwrap();
    assert_eq!(final_job.status, JobStatus::Failed);
    assert_eq!(
        final_job.error_summary.as_deref(),
        Some(STARTUP_CRASH_RECOVERY_ERROR_MSG)
    );

    let final_folder = repos.folder.get_folder(&folder_id).await.unwrap().unwrap();
    assert_eq!(final_folder.status, FolderStatus::Idle);
}

#[tokio::test]
async fn test_app_state_recover_on_startup_integration() {
    let (state, _rx) = AppState::test_state();

    let folder = Folder::new(PathBuf::from("/workspace/project"));
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();
    state
        .repositories
        .folder
        .update_status(&folder.id, FolderStatus::Scanning)
        .await
        .unwrap();

    let job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder.id),
        status: JobStatus::Pending,
        files_total: 200,
        files_processed: 0,
        files_indexed: 0,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    state.repositories.job.create_job(&job).await.unwrap();

    let report = state.recover_on_startup().await.unwrap();
    assert_eq!(report.jobs_recovered, 1);
    assert_eq!(report.folders_reset, 1);

    let saved_job = state
        .repositories
        .job
        .get_job(&job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_job.status, JobStatus::Failed);
    assert!(saved_job.completed_at.is_some());

    let saved_folder = state
        .repositories
        .folder
        .get_folder(&folder.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_folder.status, FolderStatus::Idle);
}
