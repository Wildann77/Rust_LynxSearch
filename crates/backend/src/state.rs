use crate::application::orchestrator::{IndexOrchestrator, JobTracker, WorkerCommand};
use crate::config::AppConfig;
use crate::domain::models::AppSettings;
use crate::domain::ports::stubs::{
    InMemoryDocumentRegistryRepository, InMemoryFolderRepository, InMemoryJobRepository,
    InMemorySearchRepository, InMemorySettingsRepository,
};
use crate::domain::ports::{
    DocumentRegistryRepository, FolderRepository, JobRepository, SearchRepository,
    SettingsRepository,
};
use crate::error::AppError;
use elasticsearch::Elasticsearch;
use sqlx::PgPool;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc};

pub const DEFAULT_WORKER_CHANNEL_CAPACITY: usize = 100;

#[derive(Clone)]
pub struct Repositories {
    pub folder: Arc<dyn FolderRepository>,
    pub registry: Arc<dyn DocumentRegistryRepository>,
    pub job: Arc<dyn JobRepository>,
    pub settings: Arc<dyn SettingsRepository>,
    pub search: Arc<dyn SearchRepository>,
}

impl Repositories {
    pub fn in_memory() -> Self {
        Self {
            folder: Arc::new(InMemoryFolderRepository::default()),
            registry: Arc::new(InMemoryDocumentRegistryRepository::default()),
            job: Arc::new(InMemoryJobRepository::default()),
            settings: Arc::new(InMemorySettingsRepository::default()),
            search: Arc::new(InMemorySearchRepository::default()),
        }
    }

    pub fn in_memory_with_settings(initial_settings: AppSettings) -> Self {
        Self {
            folder: Arc::new(InMemoryFolderRepository::default()),
            registry: Arc::new(InMemoryDocumentRegistryRepository::default()),
            job: Arc::new(InMemoryJobRepository::default()),
            settings: Arc::new(InMemorySettingsRepository::new(initial_settings)),
            search: Arc::new(InMemorySearchRepository::default()),
        }
    }

    pub fn from_postgres(pool: PgPool, search: Arc<dyn SearchRepository>) -> Self {
        Self {
            folder: Arc::new(crate::infrastructure::postgres::PgFolderRepository::new(
                pool.clone(),
            )),
            registry: Arc::new(
                crate::infrastructure::postgres::PgDocumentRegistryRepository::new(pool.clone()),
            ),
            job: Arc::new(crate::infrastructure::postgres::PgJobRepository::new(
                pool.clone(),
            )),
            settings: Arc::new(crate::infrastructure::postgres::PgSettingsRepository::new(
                pool,
            )),
            search,
        }
    }
}

pub struct AppStateInner {
    pub db_pool: PgPool,
    pub es_client: Elasticsearch,
    pub repositories: Repositories,
    pub job_tracker: Arc<JobTracker>,
    pub orchestrator: Arc<IndexOrchestrator>,
    pub worker_sender: mpsc::Sender<WorkerCommand>,
    pub file_io_semaphore: Arc<Semaphore>,
    pub config: Arc<AppConfig>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub event_recorder:
        std::sync::RwLock<Option<Arc<crate::domain::events::RecordingDomainEventHandler>>>,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    pub fn new(
        db_pool: PgPool,
        es_client: Elasticsearch,
        repositories: Repositories,
        job_tracker: Arc<JobTracker>,
        worker_sender: mpsc::Sender<WorkerCommand>,
        file_io_semaphore: Arc<Semaphore>,
        config: Arc<AppConfig>,
        settings: Arc<RwLock<AppSettings>>,
    ) -> Self {
        let file_reader = Arc::new(crate::infrastructure::fs::LocalFileReader::new(
            file_io_semaphore.clone(),
        ));
        let file_walker = Arc::new(crate::infrastructure::fs::LocalFileWalker::new());

        let dispatcher = crate::domain::events::DomainEventDispatcher::new();
        dispatcher.register(Arc::new(
            crate::infrastructure::events::TracingDomainEventHandler::new(),
        ));
        let event_dispatcher = Arc::new(dispatcher);

        let orchestrator = Arc::new(IndexOrchestrator::new(
            repositories.clone(),
            job_tracker.clone(),
            worker_sender.clone(),
            file_walker,
            file_reader,
            event_dispatcher,
        ));

        Self {
            inner: Arc::new(AppStateInner {
                db_pool,
                es_client,
                repositories,
                job_tracker,
                orchestrator,
                worker_sender,
                file_io_semaphore,
                config,
                settings,
                event_recorder: std::sync::RwLock::new(None),
            }),
        }
    }

    pub fn from_config(
        config: AppConfig,
    ) -> Result<(Self, mpsc::Receiver<WorkerCommand>), AppError> {
        let db_pool =
            crate::infrastructure::postgres::connection::create_pg_pool_lazy(&config.database_url)?;

        let es_client =
            crate::infrastructure::elasticsearch::create_es_client(&config.elasticsearch_url)?;

        let initial_settings = AppSettings::from_config(&config);
        let search_repo = Arc::new(
            crate::infrastructure::elasticsearch::EsSearchRepository::new(
                es_client.clone(),
                config.elasticsearch_index_alias.clone(),
            ),
        );
        let repositories = Repositories::from_postgres(db_pool.clone(), search_repo);
        let job_tracker = Arc::new(JobTracker::new());
        let (worker_sender, worker_receiver) = mpsc::channel(DEFAULT_WORKER_CHANNEL_CAPACITY);
        let file_io_semaphore = Arc::new(Semaphore::new(config.file_read_concurrency_limit));
        let config_arc = Arc::new(config);
        let settings_arc = Arc::new(RwLock::new(initial_settings));

        let state = Self::new(
            db_pool,
            es_client,
            repositories,
            job_tracker,
            worker_sender,
            file_io_semaphore,
            config_arc,
            settings_arc,
        );

        Ok((state, worker_receiver))
    }

    pub fn test_state() -> (Self, mpsc::Receiver<WorkerCommand>) {
        let config = AppConfig {
            database_url: "postgres://postgres:postgres@127.0.0.1:5432/lynx_search_test".into(),
            elasticsearch_url: "http://127.0.0.1:9200".into(),
            elasticsearch_index_alias: "lynx_documents_test".into(),
            backend_bind_addr: "127.0.0.1:3001".into(),
            backend_host: "127.0.0.1".into(),
            backend_port: 3001,
            rust_log: "info".into(),
            default_max_file_size_bytes: 2 * 1024 * 1024,
            default_ignore_patterns: vec![
                ".git".into(),
                "node_modules".into(),
                "target".into(),
                "dist".into(),
                "build".into(),
            ],
            default_bm25_weights: crate::config::Bm25Weights::default(),
            file_read_concurrency_limit: 50,
        };

        let db_pool =
            crate::infrastructure::postgres::connection::create_pg_pool_lazy(&config.database_url)
                .expect("Failed to initialize test lazy pg pool");
        let es_client =
            crate::infrastructure::elasticsearch::create_es_client(&config.elasticsearch_url)
                .expect("Failed to initialize test es client");
        let initial_settings = AppSettings::from_config(&config);
        let repositories = Repositories::in_memory_with_settings(initial_settings.clone());
        let job_tracker = Arc::new(JobTracker::new());
        let (worker_sender, rx) = mpsc::channel(DEFAULT_WORKER_CHANNEL_CAPACITY);
        let file_io_semaphore = Arc::new(Semaphore::new(config.file_read_concurrency_limit));
        let config_arc = Arc::new(config);
        let settings_arc = Arc::new(RwLock::new(initial_settings));

        let state = Self::new(
            db_pool,
            es_client,
            repositories,
            job_tracker,
            worker_sender,
            file_io_semaphore,
            config_arc,
            settings_arc,
        );

        let recorder = Arc::new(crate::domain::events::RecordingDomainEventHandler::new());
        state
            .orchestrator
            .event_dispatcher()
            .register(recorder.clone());
        if let Ok(mut lock) = state.inner.event_recorder.write() {
            *lock = Some(recorder);
        }
        (state, rx)
    }

    pub fn recorded_events(&self) -> Vec<crate::domain::events::DomainEvent> {
        self.inner
            .event_recorder
            .read()
            .ok()
            .and_then(|r| r.as_ref().map(|rec| rec.recorded_events()))
            .unwrap_or_default()
    }

    pub async fn get_settings(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    pub async fn update_settings(&self, new_settings: AppSettings) -> Result<(), AppError> {
        self.repositories
            .settings
            .update_settings(&new_settings)
            .await?;
        let mut settings_lock = self.settings.write().await;
        *settings_lock = new_settings;
        Ok(())
    }

    pub async fn recover_on_startup(
        &self,
    ) -> Result<crate::application::orchestrator::RecoveryReport, AppError> {
        crate::application::orchestrator::recover_on_startup(&self.repositories).await
    }

    pub fn shutdown_token(&self) -> tokio_util::sync::CancellationToken {
        self.job_tracker.shutdown_token()
    }

    pub fn orchestrator(&self) -> &Arc<IndexOrchestrator> {
        &self.orchestrator
    }

    pub async fn graceful_shutdown<T: Send + 'static>(
        &self,
        worker_handle: Option<tokio::task::JoinHandle<T>>,
        grace_period: std::time::Duration,
    ) -> Result<crate::application::orchestrator::ShutdownReport, AppError> {
        tracing::info!("Executing graceful shutdown sequence...");

        // 1. Signal cancellation to job tracker & workers
        self.job_tracker.cancel_all();

        // 2. Wait for worker drain within grace period
        let timed_out = if let Some(handle) = worker_handle {
            crate::application::orchestrator::drain_worker_with_grace_period(handle, grace_period)
                .await
        } else {
            false
        };

        // 3. Mark unfinished jobs CANCELLED in repository & reset scanning folders
        let (jobs_cancelled, folders_reset) =
            crate::application::orchestrator::shutdown_in_flight_jobs(
                &self.repositories,
                &self.job_tracker,
            )
            .await?;

        // 4. Close database connection pool
        self.db_pool.close().await;
        tracing::info!("Database connection pool closed");

        // 5. Close HTTP clients
        tracing::info!("HTTP clients and search transport closed");

        Ok(crate::application::orchestrator::ShutdownReport {
            jobs_cancelled,
            folders_reset,
            grace_period_timed_out: timed_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_creation_and_clone_semantics() {
        let (state1, _rx) = AppState::test_state();
        let state2 = state1.clone();

        assert!(Arc::ptr_eq(&state1.inner, &state2.inner));
        assert_eq!(state1.config.backend_port, 3001);
        assert_eq!(state2.config.backend_port, 3001);
    }

    #[tokio::test]
    async fn test_app_state_job_tracker_and_folder_lock() {
        use crate::domain::models::FolderId;

        let (state, _rx) = AppState::test_state();
        let folder_id = FolderId::new();

        let lock1 = state.job_tracker.try_acquire_folder_lock(&folder_id);
        assert!(lock1.is_ok());

        let lock2 = state.job_tracker.try_acquire_folder_lock(&folder_id);
        assert!(lock2.is_err());
        match lock2.unwrap_err() {
            AppError::JobConflict(id) => assert_eq!(id, *folder_id),
            other => panic!("Expected JobConflict error, got: {other:?}"),
        }

        drop(lock1);

        let lock3 = state.job_tracker.try_acquire_folder_lock(&folder_id);
        assert!(lock3.is_ok());
    }

    #[tokio::test]
    async fn test_app_state_worker_sender() {
        use crate::domain::models::{FolderId, JobId};

        let (state, mut rx) = AppState::test_state();
        let job_id = JobId::new();
        let folder_id = FolderId::new();

        let send_result = state
            .worker_sender
            .send(WorkerCommand::IndexFolder {
                job_id,
                folder_id,
                rescan: true,
            })
            .await;
        assert!(send_result.is_ok());

        let received = rx.recv().await.expect("Failed to receive command");
        match received {
            WorkerCommand::IndexFolder {
                job_id: r_job_id,
                folder_id: r_folder_id,
                rescan,
            } => {
                assert_eq!(r_job_id, job_id);
                assert_eq!(r_folder_id, folder_id);
                assert!(rescan);
            }
            _ => panic!("Unexpected worker command"),
        }
    }

    #[tokio::test]
    async fn test_app_state_file_io_semaphore() {
        let (state, _rx) = AppState::test_state();
        assert_eq!(state.file_io_semaphore.available_permits(), 50);

        let permit = state.file_io_semaphore.clone().acquire_owned().await;
        assert!(permit.is_ok());
        assert_eq!(state.file_io_semaphore.available_permits(), 49);

        drop(permit);
        assert_eq!(state.file_io_semaphore.available_permits(), 50);
    }

    #[tokio::test]
    async fn test_app_state_settings_update() {
        let (state, _rx) = AppState::test_state();
        let initial_settings = state.get_settings().await;
        assert_eq!(initial_settings.max_file_size_bytes, 2 * 1024 * 1024);

        let mut updated = initial_settings.clone();
        updated.max_file_size_bytes = 4 * 1024 * 1024;
        let update_res = state.update_settings(updated.clone()).await;
        assert!(update_res.is_ok());

        let current = state.get_settings().await;
        assert_eq!(current.max_file_size_bytes, 4 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_app_state_recover_on_startup() {
        use crate::domain::models::{Folder, FolderStatus, IndexingJob, JobStatus};
        use std::path::PathBuf;

        let (state, _rx) = AppState::test_state();

        let mut folder = Folder::new(PathBuf::from("/test/folder"));
        folder.status = FolderStatus::Scanning;
        state
            .repositories
            .folder
            .create_folder(&folder)
            .await
            .unwrap();

        use crate::domain::models::JobId;

        let job = IndexingJob {
            id: JobId::new(),
            folder_id: Some(folder.id),
            job_type: Default::default(),
            status: JobStatus::Running,
            files_total: 10,
            files_processed: 3,
            files_indexed: 3,
            files_skipped: 0,
            files_failed: 0,
            error_summary: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
        };
        state.repositories.job.create_job(&job).await.unwrap();

        let report = state.recover_on_startup().await.unwrap();
        assert_eq!(report.jobs_recovered, 1);
        assert_eq!(report.folders_reset, 1);

        let recovered_job = state
            .repositories
            .job
            .get_job(&job.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered_job.status, JobStatus::Failed);
        assert!(recovered_job.completed_at.is_some());
        assert_eq!(
            recovered_job.error_summary.as_deref(),
            Some(crate::application::orchestrator::STARTUP_CRASH_RECOVERY_ERROR_MSG)
        );

        let recovered_folder = state
            .repositories
            .folder
            .get_folder(&folder.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered_folder.status, FolderStatus::Idle);

        // Idempotency check:
        let second_report = state.recover_on_startup().await.unwrap();
        assert_eq!(second_report.jobs_recovered, 0);
        assert_eq!(second_report.folders_reset, 0);
    }

    #[tokio::test]
    async fn test_app_state_graceful_shutdown() {
        use crate::domain::models::{Folder, FolderStatus, IndexingJob, JobId, JobStatus};
        use std::path::PathBuf;
        use std::time::Duration;

        let (state, mut rx) = AppState::test_state();

        let mut folder = Folder::new(PathBuf::from("/test/shutdown_state"));
        folder.status = FolderStatus::Scanning;
        state
            .repositories
            .folder
            .create_folder(&folder)
            .await
            .unwrap();

        let job = IndexingJob {
            id: JobId::new(),
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
        state.repositories.job.create_job(&job).await.unwrap();

        let shutdown_token = state.shutdown_token();
        let worker_handle = tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_token.cancelled() => {}
                _ = rx.recv() => {}
            }
        });

        let report = state
            .graceful_shutdown(Some(worker_handle), Duration::from_millis(500))
            .await
            .unwrap();

        assert_eq!(report.jobs_cancelled, 1);
        assert_eq!(report.folders_reset, 1);
        assert!(!report.grace_period_timed_out);

        let cancelled_job = state
            .repositories
            .job
            .get_job(&job.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancelled_job.status, JobStatus::Cancelled);
        assert!(cancelled_job.completed_at.is_some());

        let reset_folder = state
            .repositories
            .folder
            .get_folder(&folder.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reset_folder.status, FolderStatus::Idle);

        assert!(state.job_tracker.is_shutting_down());
        assert!(state.db_pool.is_closed());
    }

    #[tokio::test]
    async fn test_repositories_from_postgres_creation() {
        let pool = crate::infrastructure::postgres::connection::create_pg_pool_lazy(
            "postgres://postgres:postgres@127.0.0.1:5432/lynx_search_test",
        )
        .unwrap();
        let search = Arc::new(InMemorySearchRepository::default());
        let repos = Repositories::from_postgres(pool, search);

        // Verify that calling through trait object dispatch works cleanly and fails gracefully on unconnectable DB
        assert!(repos.folder.list_folders().await.is_err());
    }
}
