use backend::domain::models::{
    Folder, FolderId, FolderStatus, IndexingJob, JobId, JobProgressUpdate, JobStatus, JobType,
};
use backend::state::AppState;
use chrono::Utc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_lifecycle_pending_to_running_to_completed() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    // Setup folder
    let folder_id = FolderId::new();
    let temp = tempfile::tempdir().unwrap();
    let folder = Folder {
        id: folder_id,
        path: temp.path().to_path_buf(),
        status: FolderStatus::Idle,
        created_at: Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 1. Submit job -> Status must be PENDING
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    let tracker_job = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracker_job.status, JobStatus::Pending);
    assert_eq!(tracker_job.files_total, 0);
    assert_eq!(tracker_job.files_processed, 0);
    assert!(tracker_job.completed_at.is_none());

    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Pending);
    assert!(db_job.completed_at.is_none());

    // 2. Dispatch worker command -> Transitions to RUNNING, then finishes to COMPLETED
    let command = rx.recv().await.unwrap();
    orchestrator.dispatch(command).await.unwrap();

    // 3. Final summary verified in memory and DB
    let final_tracker = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(final_tracker.status, JobStatus::Completed);
    assert!(final_tracker.completed_at.is_some());

    let final_db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(final_db_job.status, JobStatus::Completed);
    assert!(final_db_job.completed_at.is_some());
    assert_eq!(final_db_job.error_summary, None);

    // Folder status restored to IDLE
    let final_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(final_folder.status, FolderStatus::Idle);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_lifecycle_pending_to_cancelled_before_dispatch() {
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

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Cancel while still pending
    orchestrator.submit_cancel_job(job_id).await.unwrap();

    let tracker_job = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracker_job.status, JobStatus::Cancelled);
    assert!(tracker_job.completed_at.is_some());

    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Cancelled);
    assert!(db_job.completed_at.is_some());

    // Worker receives and dispatches cancelled command -> Must be safe no-op
    let command = rx.recv().await.unwrap();
    orchestrator.dispatch(command).await.unwrap();

    // State remains Cancelled
    let after_dispatch = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(after_dispatch.status, JobStatus::Cancelled);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_lifecycle_running_to_cancelled_mid_execution() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/cancel_running".into(),
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

    let mut reporter = orchestrator.create_progress_reporter(job_id, Some(10));
    reporter.mark_running(Some(200)).await.unwrap();

    // Process some files
    reporter.record_indexed(30).await.unwrap();
    reporter.record_skipped(10).await.unwrap();

    assert!(!reporter.is_cancelled());

    // Trigger mid-run cancellation
    orchestrator.submit_cancel_job(job_id).await.unwrap();
    assert!(reporter.is_cancelled());

    // Finalize cancelled job
    reporter.finish_cancelled().await.unwrap();

    // Verify terminal summary
    let tracker_state = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracker_state.status, JobStatus::Cancelled);
    assert_eq!(tracker_state.files_total, 200);
    assert_eq!(tracker_state.files_indexed, 30);
    assert_eq!(tracker_state.files_skipped, 10);
    assert_eq!(tracker_state.files_processed, 40);
    assert!(tracker_state.completed_at.is_some());

    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Cancelled);
    assert_eq!(db_job.files_total, 200);
    assert_eq!(db_job.files_indexed, 30);
    assert_eq!(db_job.files_skipped, 10);
    assert_eq!(db_job.files_processed, 40);
    assert!(db_job.completed_at.is_some());
}

#[tokio::test]
async fn test_lifecycle_running_to_failed_with_error_summary() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/failed_job".into(),
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

    let mut reporter = orchestrator.create_progress_reporter(job_id, Some(50));
    reporter.mark_running(Some(150)).await.unwrap();

    reporter.record_indexed(20).await.unwrap();
    reporter.record_failed(5).await.unwrap();

    let fatal_msg = "Disk read I/O error on file /workspace/failed_job/corrupt.bin";
    reporter.finish_failed(fatal_msg).await.unwrap();

    // Verify FAILED state & persisted error_summary
    let tracker_state = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(tracker_state.status, JobStatus::Failed);
    assert_eq!(tracker_state.error_summary.as_deref(), Some(fatal_msg));
    assert_eq!(tracker_state.files_processed, 25);
    assert!(tracker_state.completed_at.is_some());

    let db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_job.status, JobStatus::Failed);
    assert_eq!(db_job.error_summary.as_deref(), Some(fatal_msg));
    assert_eq!(db_job.files_processed, 25);
    assert!(db_job.completed_at.is_some());
}

#[tokio::test]
async fn test_progress_counters_batch_db_flush_and_final_summary() {
    let (state, _rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    state
        .repositories
        .folder
        .create_folder(&Folder {
            id: folder_id,
            path: "/workspace/batch_flush".into(),
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

    // Batch size of 100
    let mut reporter = orchestrator.create_progress_reporter(job_id, Some(100));

    // Planning result sets files_total = 250
    let planned_total = 250;
    reporter.mark_running(Some(planned_total)).await.unwrap();

    // Step 1: Record 60 indexed, 30 skipped (total 90 processed < batch size 100)
    reporter.record_indexed(60).await.unwrap();
    reporter.record_skipped(30).await.unwrap();

    // In-memory has real-time updates (90 processed)
    let memory_step1 = reporter.current_progress().unwrap();
    assert_eq!(memory_step1.files_processed, 90);
    assert_eq!(memory_step1.files_indexed, 60);
    assert_eq!(memory_step1.files_skipped, 30);

    // DB has not yet flushed counter increments (still 0 in DB)
    let db_step1 = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_step1.files_processed, 0);

    // Step 2: Record 20 more indexed (accumulated 110 >= batch_size 100 -> auto flush triggered!)
    reporter.record_indexed(20).await.unwrap();

    // DB now has flushed at least 110
    let db_step2 = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_step2.files_processed, 110);
    assert_eq!(db_step2.files_indexed, 80);
    assert_eq!(db_step2.files_skipped, 30);

    // Step 3: Record remaining items: 70 indexed, 60 skipped, 10 failed (total 250)
    reporter.record_indexed(70).await.unwrap();
    reporter.record_skipped(60).await.unwrap();
    reporter.record_failed(10).await.unwrap();

    // Step 4: Finish COMPLETED -> forces final flush of remaining deltas
    reporter.finish_completed().await.unwrap();

    let memory_final = reporter.current_progress().unwrap();
    assert_eq!(memory_final.status, JobStatus::Completed);
    assert_eq!(memory_final.files_total, planned_total);
    assert_eq!(memory_final.files_processed, 250);
    assert_eq!(memory_final.files_indexed, 150);
    assert_eq!(memory_final.files_skipped, 90);
    assert_eq!(memory_final.files_failed, 10);
    assert!(memory_final.completed_at.is_some());

    let db_final = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_final.status, JobStatus::Completed);
    assert_eq!(db_final.files_total, planned_total);
    assert_eq!(db_final.files_processed, 250);
    assert_eq!(db_final.files_indexed, 150);
    assert_eq!(db_final.files_skipped, 90);
    assert_eq!(db_final.files_failed, 10);
    assert!(db_final.completed_at.is_some());
    assert_eq!(db_final.files_total, db_final.files_processed);
}

#[tokio::test]
async fn test_terminal_jobs_cannot_regress_status() {
    let (state, _rx) = AppState::test_state();
    let job_id = JobId::new();

    let job = IndexingJob {
        id: job_id,
        folder_id: None,
        job_type: JobType::Rebuild,
        status: JobStatus::Pending,
        files_total: 10,
        files_processed: 0,
        files_indexed: 0,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    state.repositories.job.create_job(&job).await.unwrap();
    state
        .job_tracker
        .register_job(job_id, None, CancellationToken::new());

    let mut reporter = state.orchestrator().create_progress_reporter(job_id, None);
    reporter.mark_running(Some(10)).await.unwrap();
    reporter.record_indexed(10).await.unwrap();
    reporter.finish_completed().await.unwrap();

    // Verify completed
    assert_eq!(
        state.job_tracker.get_progress(&job_id).unwrap().status,
        JobStatus::Completed
    );

    // Attempt to regress status back to Running or Pending
    state.job_tracker.update_progress(
        &job_id,
        JobProgressUpdate {
            status: Some(JobStatus::Running),
            ..Default::default()
        },
    );
    state
        .repositories
        .job
        .update_progress(
            &job_id,
            &JobProgressUpdate {
                status: Some(JobStatus::Running),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Verify status remained Completed
    assert_eq!(
        state.job_tracker.get_progress(&job_id).unwrap().status,
        JobStatus::Completed
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
        JobStatus::Completed
    );
}
