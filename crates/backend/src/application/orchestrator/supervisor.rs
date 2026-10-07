use crate::application::orchestrator::{IndexOrchestrator, JobTracker, WorkerCommand};
use crate::error::AppError;
use crate::state::Repositories;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

pub const WORKER_PANIC_ERROR_MSG: &str = "Internal Worker Panic: Process terminated unexpectedly";
pub const DEFAULT_SUPERVISOR_BACKOFF: Duration = Duration::from_secs(1);
pub const MAX_CONCURRENT_FOLDER_SCANS: usize = 2;

/// Recovers database and in-memory state after a background worker crash/panic.
pub async fn recover_panicked_worker(
    repositories: &Repositories,
    job_tracker: &JobTracker,
) -> Result<(u64, u64), AppError> {
    tracing::error!(
        "CRITICAL: Background worker panicked! Executing state recovery and releasing folder locks..."
    );

    // 1. Mark in-memory active jobs as FAILED and cancel tokens
    job_tracker.fail_running_jobs(WORKER_PANIC_ERROR_MSG);

    // 2. Release any folder locks and rebuild locks held in memory
    job_tracker.clear_folder_locks();
    job_tracker.clear_rebuild_lock();

    // 3. Mark database active jobs as FAILED
    let jobs_failed = repositories
        .job
        .recover_dangling_jobs(WORKER_PANIC_ERROR_MSG)
        .await?;

    // 4. Reset folder scanning status in database to IDLE
    let folders_reset = repositories.folder.reset_scanning_folders().await?;

    tracing::info!(
        jobs_failed,
        folders_reset,
        "Panicked worker state recovery completed"
    );

    Ok((jobs_failed, folders_reset))
}

/// Generic supervisor loop that runs a worker factory, catches panics, recovers state,
/// applies backoff, and restarts until stopped by shutdown_token or clean worker termination.
pub async fn run_supervisor<F, Fut>(
    worker_factory: F,
    repositories: Repositories,
    job_tracker: Arc<JobTracker>,
    shutdown_token: CancellationToken,
    backoff: Duration,
) where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    loop {
        if shutdown_token.is_cancelled() {
            tracing::info!("Supervisor received shutdown signal before worker spawn, exiting");
            break;
        }

        let worker_fut = worker_factory();
        let handle = tokio::spawn(worker_fut);

        match handle.await {
            Ok(()) => {
                tracing::info!("Worker task completed cleanly");
                break;
            }
            Err(join_err) => {
                if join_err.is_panic() {
                    tracing::error!(
                        error = ?join_err,
                        "CRITICAL: Background worker task panicked! Initiating supervisor recovery"
                    );

                    if let Err(err) = recover_panicked_worker(&repositories, &job_tracker).await {
                        tracing::error!("Failed to recover panicked worker state: {err}");
                    }

                    if shutdown_token.is_cancelled() {
                        tracing::info!(
                            "Shutdown token cancelled after worker panic, stopping supervisor"
                        );
                        break;
                    }

                    tracing::warn!(
                        backoff_millis = backoff.as_millis(),
                        "Applying backoff delay before restarting worker..."
                    );

                    tokio::select! {
                        _ = shutdown_token.cancelled() => {
                            tracing::info!("Supervisor received shutdown signal during backoff delay");
                            break;
                        }
                        _ = tokio::time::sleep(backoff) => {
                            tracing::info!("Supervisor backoff elapsed, restarting worker task");
                        }
                    }
                } else {
                    tracing::warn!(
                        error = ?join_err,
                        "Worker task terminated without panic (cancelled/aborted)"
                    );
                    break;
                }
            }
        }
    }
}

/// Worker loop processing incoming `WorkerCommand`s and dispatching via `IndexOrchestrator`.
/// Supports up to `MAX_CONCURRENT_FOLDER_SCANS` concurrent folder scans while maintaining
/// immediate cancellation responsiveness and exclusive index rebuilding.
pub async fn run_orchestrator_worker_loop(
    orchestrator: Arc<IndexOrchestrator>,
    receiver: Arc<Mutex<mpsc::Receiver<WorkerCommand>>>,
    shutdown_token: CancellationToken,
) {
    let scan_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FOLDER_SCANS));
    let mut tasks = JoinSet::new();

    loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => {
                tracing::info!("Worker loop received shutdown signal, stopped accepting new tasks");
                break;
            }
            Some(res) = tasks.join_next(), if !tasks.is_empty() => {
                if let Err(join_err) = res {
                    if join_err.is_panic() {
                        tracing::error!("A spawned worker task panicked: {:?}", join_err);
                        std::panic::resume_unwind(join_err.into_panic());
                    } else {
                        tracing::warn!("A spawned worker task was cancelled/aborted: {:?}", join_err);
                    }
                }
            }
            cmd_opt = async {
                let mut rx = receiver.lock().await;
                rx.recv().await
            } => {
                match cmd_opt {
                    Some(cmd) => {
                        tracing::debug!("Worker received command: {cmd:?}");
                        match cmd {
                            WorkerCommand::CancelJob { job_id } => {
                                tracing::debug!(job_id = %job_id, "Executing immediate cancellation in worker loop");
                                if let Err(err) = orchestrator.dispatch(WorkerCommand::CancelJob { job_id }).await {
                                    tracing::error!("Error dispatching cancel job command: {err}");
                                }
                            }
                            WorkerCommand::IndexFolder {
                                job_id,
                                folder_id,
                                rescan,
                            } => {
                                let sem = scan_semaphore.clone();
                                let orch = orchestrator.clone();
                                let token = shutdown_token.clone();
                                tasks.spawn(async move {
                                    let _permit = tokio::select! {
                                        _ = token.cancelled() => {
                                            tracing::info!(job_id = %job_id, "Shutdown cancelled before acquiring scan permit");
                                            return;
                                        }
                                        permit_res = sem.acquire_owned() => {
                                            match permit_res {
                                                Ok(p) => p,
                                                Err(_) => {
                                                    tracing::warn!("Scan semaphore closed for job {job_id}");
                                                    return;
                                                }
                                            }
                                        }
                                    };

                                    tracing::debug!(job_id = %job_id, folder_id = %folder_id, "Worker task starting IndexFolder execution");
                                    if let Err(err) = orch.dispatch(WorkerCommand::IndexFolder { job_id, folder_id, rescan }).await {
                                        tracing::error!("Error dispatching IndexFolder command: {err}");
                                    }
                                });
                            }
                            WorkerCommand::RebuildIndex {
                                job_id,
                                target_index,
                            } => {
                                let sem = scan_semaphore.clone();
                                let orch = orchestrator.clone();
                                let token = shutdown_token.clone();
                                tasks.spawn(async move {
                                    let _permits = tokio::select! {
                                        _ = token.cancelled() => {
                                            tracing::info!(job_id = %job_id, "Shutdown cancelled before acquiring rebuild permits");
                                            return;
                                        }
                                        permits_res = sem.acquire_many_owned(MAX_CONCURRENT_FOLDER_SCANS as u32) => {
                                            match permits_res {
                                                Ok(p) => p,
                                                Err(_) => {
                                                    tracing::warn!("Scan semaphore closed for rebuild job {job_id}");
                                                    return;
                                                }
                                            }
                                        }
                                    };

                                    tracing::debug!(job_id = %job_id, "Worker task starting RebuildIndex execution (exclusive)");
                                    if let Err(err) = orch.dispatch(WorkerCommand::RebuildIndex { job_id, target_index }).await {
                                        tracing::error!("Error dispatching RebuildIndex command: {err}");
                                    }
                                });
                            }
                        }
                    }
                    None => {
                        tracing::debug!("Worker command channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Await any remaining background tasks before finishing the loop
    while let Some(res) = tasks.join_next().await {
        if let Err(join_err) = res
            && join_err.is_panic()
        {
            tracing::error!(
                "Worker background task panicked during shutdown drain: {:?}",
                join_err
            );
            std::panic::resume_unwind(join_err.into_panic());
        }
    }
}

/// Fallback worker loop for test scenarios without an orchestrator instance.
pub async fn run_worker_loop(
    receiver: Arc<Mutex<mpsc::Receiver<WorkerCommand>>>,
    shutdown_token: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => {
                tracing::info!("Worker loop received shutdown signal, stopped accepting new tasks");
                break;
            }
            cmd_opt = async {
                let mut rx = receiver.lock().await;
                rx.recv().await
            } => {
                match cmd_opt {
                    Some(cmd) => {
                        tracing::debug!("Worker received command: {cmd:?}");
                    }
                    None => {
                        tracing::debug!("Worker command channel closed");
                        break;
                    }
                }
            }
        }
    }
}

/// Spawns a supervised worker loop task using an `IndexOrchestrator`.
pub fn spawn_orchestrator_supervisor(
    orchestrator: Arc<IndexOrchestrator>,
    worker_rx: mpsc::Receiver<WorkerCommand>,
    shutdown_token: CancellationToken,
    backoff: Duration,
) -> JoinHandle<()> {
    let rx_arc = Arc::new(Mutex::new(worker_rx));
    let token_clone = shutdown_token.clone();
    let repositories = orchestrator.repositories().clone();
    let job_tracker = orchestrator.job_tracker().clone();

    tokio::spawn(async move {
        let rx_for_factory = rx_arc.clone();
        let token_for_factory = token_clone.clone();
        let orch_for_factory = orchestrator.clone();

        let factory = move || {
            let rx = rx_for_factory.clone();
            let token = token_for_factory.clone();
            let orch = orch_for_factory.clone();
            async move {
                run_orchestrator_worker_loop(orch, rx, token).await;
            }
        };

        run_supervisor(factory, repositories, job_tracker, token_clone, backoff).await;
    })
}

/// Spawns a supervised worker loop task.
///
/// Wraps `worker_rx` in an `Arc<Mutex>` so that receiver and buffered messages
/// survive task crashes and can be consumed by the restarted worker.
/// Returns the supervisor `JoinHandle<()>`.
pub fn spawn_worker_supervisor(
    repositories: Repositories,
    job_tracker: Arc<JobTracker>,
    worker_rx: mpsc::Receiver<WorkerCommand>,
    shutdown_token: CancellationToken,
    backoff: Duration,
) -> JoinHandle<()> {
    let (tx, _) = mpsc::channel(1);
    let orchestrator = Arc::new(IndexOrchestrator::with_default_fs(
        repositories,
        job_tracker,
        tx,
    ));
    spawn_orchestrator_supervisor(orchestrator, worker_rx, shutdown_token, backoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Folder, FolderId, FolderStatus, IndexingJob, JobId, JobStatus};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_recover_panicked_worker_resets_jobs_and_folders() {
        let repositories = Repositories::in_memory();
        let job_tracker = Arc::new(JobTracker::new());

        let folder = Folder {
            id: FolderId::new(),
            path: "/path/to/project".into(),
            status: FolderStatus::Scanning,
            created_at: chrono::Utc::now(),
            last_scanned_at: None,
        };
        repositories.folder.create_folder(&folder).await.unwrap();

        let job_id = JobId::new();
        let token = CancellationToken::new();
        job_tracker.register_job(job_id, folder.id, token);
        let job_model = IndexingJob {
            id: job_id,
            folder_id: Some(folder.id),
            job_type: Default::default(),
            status: JobStatus::Running,
            files_total: 10,
            files_processed: 2,
            files_indexed: 2,
            files_skipped: 0,
            files_failed: 0,
            error_summary: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
        };
        repositories.job.create_job(&job_model).await.unwrap();

        let (jobs_failed, folders_reset) = recover_panicked_worker(&repositories, &job_tracker)
            .await
            .unwrap();

        assert_eq!(jobs_failed, 1);
        assert_eq!(folders_reset, 1);

        let in_memory_job = job_tracker.get_progress(&job_id).unwrap();
        assert_eq!(in_memory_job.status, JobStatus::Failed);
        assert_eq!(
            in_memory_job.error_summary,
            Some(WORKER_PANIC_ERROR_MSG.to_string())
        );

        let db_job = repositories.job.get_job(&job_id).await.unwrap().unwrap();
        assert_eq!(db_job.status, JobStatus::Failed);
        assert_eq!(
            db_job.error_summary,
            Some(WORKER_PANIC_ERROR_MSG.to_string())
        );

        let db_folder = repositories
            .folder
            .get_folder(&folder.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(db_folder.status, FolderStatus::Idle);
    }

    #[tokio::test]
    async fn test_supervisor_catches_panic_and_restarts_worker() {
        let repositories = Repositories::in_memory();
        let job_tracker = Arc::new(JobTracker::new());
        let shutdown_token = CancellationToken::new();
        let run_count = Arc::new(AtomicU32::new(0));

        let run_count_clone = run_count.clone();
        let shutdown_token_clone = shutdown_token.clone();

        let factory = move || {
            let run_count = run_count_clone.clone();
            let shutdown_token = shutdown_token_clone.clone();
            async move {
                let current = run_count.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    panic!("Simulated worker panic on run 0!");
                } else {
                    // Second run: run cleanly until shutdown
                    shutdown_token.cancel();
                }
            }
        };

        run_supervisor(
            factory,
            repositories,
            job_tracker,
            shutdown_token,
            Duration::from_millis(10),
        )
        .await;

        assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_supervisor_shutdown_token_cancels_during_backoff() {
        let repositories = Repositories::in_memory();
        let job_tracker = Arc::new(JobTracker::new());
        let shutdown_token = CancellationToken::new();

        let shutdown_token_clone = shutdown_token.clone();
        let factory = move || async move {
            panic!("Worker panic!");
        };

        let token_for_spawn = shutdown_token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            token_for_spawn.cancel();
        });

        let start = std::time::Instant::now();
        run_supervisor(
            factory,
            repositories,
            job_tracker,
            shutdown_token_clone,
            Duration::from_secs(5), // Long backoff
        )
        .await;

        // Verify supervisor aborted early due to cancellation rather than sleeping full 5s
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
