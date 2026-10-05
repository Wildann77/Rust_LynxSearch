use crate::application::orchestrator::{JobProgressReporter, JobTracker, WorkerCommand};
use crate::domain::events::{DomainEvent, DomainEventDispatcher, JobSummary};
use crate::domain::models::{
    DocumentId, DocumentStatus, Folder, FolderId, FolderStatus, IndexedDocument, IndexingJob,
    JobId, JobProgressUpdate, JobStatus, JobType, RegistryEntry,
};
use crate::domain::ports::file_system::{
    FileReader, FileWalker, ReadOptions, WalkOptions, compute_sha256,
};
use crate::domain::services::document_extractor::{
    DocumentExtractor, ExtractOptions, ExtractionResult,
};
use crate::domain::services::scan_planner::{PlanSkipReason, ScanPlanner, normalize_path_str};
use crate::error::AppError;
use crate::state::Repositories;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Central application service orchestrating background indexing jobs,
/// folder locks, and worker queue dispatching.
pub struct IndexOrchestrator {
    repositories: Repositories,
    job_tracker: Arc<JobTracker>,
    worker_sender: mpsc::Sender<WorkerCommand>,
    file_walker: Arc<dyn FileWalker>,
    file_reader: Arc<dyn FileReader>,
    event_dispatcher: Arc<DomainEventDispatcher>,
}

impl IndexOrchestrator {
    pub fn new(
        repositories: Repositories,
        job_tracker: Arc<JobTracker>,
        worker_sender: mpsc::Sender<WorkerCommand>,
        file_walker: Arc<dyn FileWalker>,
        file_reader: Arc<dyn FileReader>,
        event_dispatcher: Arc<DomainEventDispatcher>,
    ) -> Self {
        Self {
            repositories,
            job_tracker,
            worker_sender,
            file_walker,
            file_reader,
            event_dispatcher,
        }
    }

    pub fn with_default_fs(
        repositories: Repositories,
        job_tracker: Arc<JobTracker>,
        worker_sender: mpsc::Sender<WorkerCommand>,
    ) -> Self {
        let dispatcher = DomainEventDispatcher::new();
        dispatcher.register(Arc::new(
            crate::infrastructure::events::TracingDomainEventHandler::new(),
        ));
        Self::new(
            repositories,
            job_tracker,
            worker_sender,
            Arc::new(crate::infrastructure::fs::LocalFileWalker::new()),
            Arc::new(crate::infrastructure::fs::LocalFileReader::with_default_permits()),
            Arc::new(dispatcher),
        )
    }

    pub fn repositories(&self) -> &Repositories {
        &self.repositories
    }

    pub fn job_tracker(&self) -> &Arc<JobTracker> {
        &self.job_tracker
    }

    pub fn worker_sender(&self) -> &mpsc::Sender<WorkerCommand> {
        &self.worker_sender
    }

    pub fn file_walker(&self) -> &Arc<dyn FileWalker> {
        &self.file_walker
    }

    pub fn file_reader(&self) -> &Arc<dyn FileReader> {
        &self.file_reader
    }

    pub fn event_dispatcher(&self) -> &Arc<DomainEventDispatcher> {
        &self.event_dispatcher
    }

    pub fn create_progress_reporter(
        &self,
        job_id: JobId,
        batch_size: Option<i32>,
    ) -> JobProgressReporter {
        JobProgressReporter::new(
            job_id,
            self.job_tracker.clone(),
            self.repositories.job.clone(),
            batch_size,
        )
    }

    /// Submits a folder scan/rescan job to the queue.
    ///
    /// Validates folder existence, pre-checks folder lock to fail fast on conflict,
    /// persists `Pending` job in DB, registers in `JobTracker`, and enqueues `WorkerCommand`.
    pub async fn submit_index_folder(
        &self,
        folder_id: FolderId,
        rescan: bool,
    ) -> Result<JobId, AppError> {
        if self.job_tracker.is_shutting_down() {
            return Err(AppError::Internal("Server is shutting down".into()));
        }

        // 1. Verify folder exists in database
        let _folder = self
            .repositories
            .folder
            .get_folder(&folder_id)
            .await?
            .ok_or_else(|| AppError::FolderNotFound(*folder_id.as_uuid()))?;

        // 2. Acquire folder lock (fail fast if already scanning or rebuilding)
        self.job_tracker.try_lock_folder(&folder_id)?;

        // 3. Create and persist Pending job
        let job_id = JobId::new();
        let job = IndexingJob {
            id: job_id,
            folder_id: Some(folder_id),
            job_type: if rescan {
                JobType::Rescan
            } else {
                JobType::Import
            },
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
        if let Err(e) = self.repositories.job.create_job(&job).await {
            self.job_tracker.release_folder_lock(&folder_id);
            return Err(e);
        }

        // 4. Register in JobTracker
        let token = CancellationToken::new();
        self.job_tracker.register_job(job_id, folder_id, token);

        // 5. Enqueue command to worker channel
        if let Err(e) = self
            .worker_sender
            .send(WorkerCommand::IndexFolder {
                job_id,
                folder_id,
                rescan,
            })
            .await
        {
            self.job_tracker.release_folder_lock(&folder_id);
            return Err(AppError::Internal(format!(
                "Failed to enqueue index folder command: {e}"
            )));
        }

        tracing::info!(
            job_id = %job_id,
            folder_id = %folder_id,
            rescan,
            "Submitted folder indexing job to worker queue"
        );

        Ok(job_id)
    }

    /// Submits a job cancellation request.
    pub async fn submit_cancel_job(&self, job_id: JobId) -> Result<(), AppError> {
        let current_status = if let Some(progress) = self.job_tracker.get_progress(&job_id) {
            Some(progress.status)
        } else if let Some(db_job) = self.repositories.job.get_job(&job_id).await? {
            Some(db_job.status)
        } else {
            None
        };

        let status = current_status.ok_or_else(|| AppError::JobNotFound(*job_id.as_uuid()))?;

        if status.is_terminal() {
            return Err(AppError::JobNotActive {
                id: *job_id.as_uuid(),
                status: status.to_string(),
            });
        }

        let is_pending = status == JobStatus::Pending;

        self.job_tracker.cancel_job(&job_id);

        self.repositories.job.mark_cancelled(&job_id).await?;

        if is_pending {
            let folder_id = if let Some(fid) = self.job_tracker.get_folder_for_job(&job_id) {
                Some(fid)
            } else if let Ok(Some(job)) = self.repositories.job.get_job(&job_id).await {
                job.folder_id
            } else {
                None
            };

            if let Some(folder_id) = folder_id {
                self.job_tracker.release_folder_lock(&folder_id);
                let _ = self
                    .repositories
                    .folder
                    .update_status(&folder_id, FolderStatus::Idle)
                    .await;
            } else {
                self.job_tracker.release_rebuild_lock();
            }
        }

        // Forward cancellation command to worker loop
        let _ = self
            .worker_sender
            .send(WorkerCommand::CancelJob { job_id })
            .await;

        tracing::info!(job_id = %job_id, "Submitted job cancellation");
        Ok(())
    }

    /// Submits an index rebuild job to the queue.
    pub async fn submit_rebuild_index(&self) -> Result<(JobId, String), AppError> {
        if self.job_tracker.is_shutting_down() {
            return Err(AppError::Internal("Server is shutting down".into()));
        }

        // 1. Acquire rebuild lock (fail fast if scan or another rebuild is running)
        self.job_tracker.try_lock_rebuild()?;

        // 2. Determine target version & physical index name
        let alias = self.repositories.search.search_alias();
        let existing_indices = self
            .repositories
            .search
            .get_alias_indices(alias)
            .await
            .unwrap_or_default();
        let max_ver = existing_indices
            .iter()
            .filter_map(|name| {
                crate::infrastructure::elasticsearch::parse_index_version(name, alias)
            })
            .max()
            .unwrap_or(0);
        let target_version = max_ver + 1;
        let target_index =
            crate::infrastructure::elasticsearch::physical_index_name(alias, target_version);

        // 3. Create and persist Pending job
        let job_id = JobId::new();
        let job = IndexingJob {
            id: job_id,
            folder_id: None,
            job_type: JobType::Rebuild,
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
        if let Err(e) = self.repositories.job.create_job(&job).await {
            self.job_tracker.release_rebuild_lock();
            return Err(e);
        }

        // 4. Register in JobTracker
        let token = CancellationToken::new();
        self.job_tracker.register_job(job_id, None, token);

        // 5. Send command to worker
        if let Err(e) = self
            .worker_sender
            .send(WorkerCommand::RebuildIndex {
                job_id,
                target_index: target_index.clone(),
            })
            .await
        {
            self.job_tracker.release_rebuild_lock();
            return Err(AppError::Internal(format!(
                "Failed to enqueue rebuild command: {e}"
            )));
        }

        tracing::info!(job_id = %job_id, target_index = %target_index, "Submitted index rebuild job to worker queue");
        Ok((job_id, target_index))
    }

    /// Dispatches a single `WorkerCommand` received from the background queue.
    pub async fn dispatch(&self, command: WorkerCommand) -> Result<(), AppError> {
        match command {
            WorkerCommand::IndexFolder {
                job_id,
                folder_id,
                rescan,
            } => self.dispatch_index_folder(job_id, folder_id, rescan).await,
            WorkerCommand::CancelJob { job_id } => {
                self.dispatch_cancel_job(job_id).await;
                Ok(())
            }
            WorkerCommand::RebuildIndex {
                job_id,
                target_index,
            } => self.dispatch_rebuild_index(job_id, target_index).await,
        }
    }

    async fn dispatch_index_folder(
        &self,
        job_id: JobId,
        folder_id: FolderId,
        _rescan: bool,
    ) -> Result<(), AppError> {
        // 1. Check if job was cancelled while pending
        if matches!(self.job_tracker.get_progress(&job_id), Some(state) if state.status == JobStatus::Cancelled)
        {
            tracing::info!(job_id = %job_id, "Job was cancelled before execution");
            self.job_tracker.release_folder_lock(&folder_id);
            return Ok(());
        }

        // 2. Ensure folder lock is held (if directly dispatched without submit_index_folder)
        let _fallback_guard = if !self.job_tracker.is_folder_locked(&folder_id) {
            match self.job_tracker.try_acquire_folder_lock(&folder_id) {
                Ok(guard) => Some(guard),
                Err(err) => {
                    tracing::warn!(job_id = %job_id, folder_id = %folder_id, "Lock conflict during dispatch");
                    let err_msg = "Job conflict: folder already locked or rebuild in progress";
                    self.job_tracker.update_progress(
                        &job_id,
                        JobProgressUpdate {
                            status: Some(JobStatus::Failed),
                            error_summary: Some(err_msg.into()),
                            ..Default::default()
                        },
                    );
                    let _ = self
                        .repositories
                        .job
                        .update_progress(
                            &job_id,
                            &JobProgressUpdate {
                                status: Some(JobStatus::Failed),
                                error_summary: Some(err_msg.into()),
                                ..Default::default()
                            },
                        )
                        .await;
                    return Err(err);
                }
            }
        } else {
            None
        };

        // 3. Mark job RUNNING and folder SCANNING
        self.job_tracker.update_progress(
            &job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Running),
                ..Default::default()
            },
        );
        let _ = self
            .repositories
            .job
            .update_progress(
                &job_id,
                &JobProgressUpdate {
                    status: Some(JobStatus::Running),
                    ..Default::default()
                },
            )
            .await;
        let _ = self
            .repositories
            .folder
            .update_status(&folder_id, FolderStatus::Scanning)
            .await;

        let event = DomainEvent::job_started(job_id, Some(folder_id));
        self.event_dispatcher.dispatch(&event);
        tracing::info!(job_id = %job_id, folder_id = %folder_id, "Started executing folder indexing job");

        // 4. Execute scan pipeline (in Task 5.10 sets up lifecycle completion; Task 5.12 integrates full file walker & extractor)
        let scan_result = self.execute_folder_scan(&job_id, &folder_id).await;

        // 5. Finalize status and release folder lock
        if self.job_tracker.is_job_cancelled(&job_id) {
            let cancel_update = JobProgressUpdate {
                status: Some(JobStatus::Cancelled),
                error_summary: Some("Cancelled by user".to_string()),
                ..Default::default()
            };
            self.job_tracker
                .update_progress(&job_id, cancel_update.clone());
            let _ = self
                .repositories
                .job
                .update_progress(&job_id, &cancel_update)
                .await;
            let _ = self
                .repositories
                .folder
                .update_status(&folder_id, FolderStatus::Idle)
                .await;
            tracing::info!(job_id = %job_id, "Folder indexing job was cancelled during execution");
            self.job_tracker.release_folder_lock(&folder_id);
            drop(_fallback_guard);
            return Ok(());
        }

        match scan_result {
            Ok(()) => {
                self.job_tracker.update_progress(
                    &job_id,
                    JobProgressUpdate {
                        status: Some(JobStatus::Completed),
                        ..Default::default()
                    },
                );
                let _ = self
                    .repositories
                    .job
                    .update_progress(
                        &job_id,
                        &JobProgressUpdate {
                            status: Some(JobStatus::Completed),
                            ..Default::default()
                        },
                    )
                    .await;
                let _ = self
                    .repositories
                    .folder
                    .update_status(&folder_id, FolderStatus::Idle)
                    .await;

                let summary = self
                    .job_tracker
                    .get_progress(&job_id)
                    .map(|p| JobSummary {
                        files_total: p.files_total,
                        files_indexed: p.files_indexed,
                        files_skipped: p.files_skipped,
                        files_failed: p.files_failed,
                        duration_ms: None,
                    })
                    .unwrap_or_else(|| JobSummary::new(0, 0, 0, 0, None));
                self.event_dispatcher
                    .dispatch(&DomainEvent::job_completed(job_id, summary));

                tracing::info!(job_id = %job_id, "Folder indexing job completed successfully");
            }
            Err(err) => {
                let err_str = err.to_string();
                self.job_tracker.update_progress(
                    &job_id,
                    JobProgressUpdate {
                        status: Some(JobStatus::Failed),
                        error_summary: Some(err_str.clone()),
                        ..Default::default()
                    },
                );
                let _ = self
                    .repositories
                    .job
                    .update_progress(
                        &job_id,
                        &JobProgressUpdate {
                            status: Some(JobStatus::Failed),
                            error_summary: Some(err_str),
                            ..Default::default()
                        },
                    )
                    .await;
                let _ = self
                    .repositories
                    .folder
                    .update_status(&folder_id, FolderStatus::Idle)
                    .await;
                tracing::error!(job_id = %job_id, error = %err, "Folder indexing job failed");
            }
        }

        self.job_tracker.release_folder_lock(&folder_id);
        drop(_fallback_guard);
        Ok(())
    }

    async fn execute_folder_scan(
        &self,
        job_id: &JobId,
        folder_id: &FolderId,
    ) -> Result<(), AppError> {
        let mut reporter = self.create_progress_reporter(*job_id, None);

        // 1. Ambil folder dari database
        let folder = self
            .repositories
            .folder
            .get_folder(folder_id)
            .await?
            .ok_or_else(|| AppError::FolderNotFound(*folder_id.as_uuid()))?;

        // 2. Validasi direktori folder pada filesystem lokal
        if !folder.path.exists() || !folder.path.is_dir() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Folder root directory does not exist or is not a directory: {}",
                    folder.path.display()
                ),
            )));
        }

        // 3. Ambil konfigurasi settings aplikasi
        let settings = self.repositories.settings.get_settings().await?;

        // Periksa pembatalan sebelum pemindaian berat
        if reporter.is_cancelled() {
            return Ok(());
        }

        // 4. Traversal berkas menggunakan FileWalker
        let walk_options = WalkOptions {
            custom_ignore_patterns: settings.ignore_patterns.clone(),
            skip_hidden: true,
            respect_gitignore: true,
            max_depth: None,
        };

        let walker = self.file_walker.clone();
        let root_path = folder.path.clone();
        let inventory =
            tokio::task::spawn_blocking(move || walker.walk_all(&root_path, &walk_options))
                .await
                .map_err(|e| AppError::Internal(format!("File walker worker task panicked: {e}")))?
                .map_err(|e| {
                    AppError::Internal(format!(
                        "Failed to traverse directory '{}': {e}",
                        folder.path.display()
                    ))
                })?;

        if reporter.is_cancelled() {
            return Ok(());
        }

        // 5. Ambil data entri registri database untuk perbandingan ScanPlan
        let db_entries = self.repositories.registry.list_by_folder(folder_id).await?;

        // 6. Buat ScanPlan via pure domain service ScanPlanner
        let inventory_for_plan = inventory.clone();
        let db_entries_for_plan = db_entries.clone();
        let settings_for_plan = settings.clone();
        let fid = *folder_id;

        let plan = tokio::task::spawn_blocking(move || {
            ScanPlanner::plan(
                fid,
                &inventory_for_plan,
                &db_entries_for_plan,
                &settings_for_plan,
                |discovered| {
                    let bytes = std::fs::read(&discovered.absolute_path)?;
                    Ok(compute_sha256(&bytes))
                },
            )
        })
        .await
        .map_err(|e| AppError::Internal(format!("Scan planner task panicked: {e}")))?;

        // 7. Inisialisasi total counter progres (total files on disk)
        let total_files = inventory.len() as i32;
        reporter.set_files_total(total_files).await?;

        // 8. Catat file yang diskip atau gagal dibaca pada fase planning
        let now = Utc::now();
        let mut skips_count = 0;
        let mut read_failed_entries = Vec::new();

        for skip in &plan.to_skip {
            if skip.reason == PlanSkipReason::PreservedExcluded {
                continue;
            }
            if skip.reason == PlanSkipReason::ReadFailed {
                let err_msg = "Failed to read or hash file during scan planning".to_string();
                read_failed_entries.push(RegistryEntry {
                    id: skip.id,
                    folder_id: *folder_id,
                    relative_path: skip.relative_path.clone(),
                    content_hash: String::new(),
                    file_size: 0,
                    status: DocumentStatus::Failed,
                    status_reason: Some(err_msg.clone()),
                    indexed_at: now,
                    updated_at: now,
                });
                self.event_dispatcher
                    .dispatch(&DomainEvent::document_failed(
                        *job_id,
                        skip.relative_path.clone(),
                        err_msg,
                    ));
            } else {
                skips_count += 1;
            }
        }

        if skips_count > 0 {
            reporter.record_skipped(skips_count).await?;
        }
        if !read_failed_entries.is_empty() {
            self.repositories
                .registry
                .upsert_batch(&read_failed_entries)
                .await?;
            reporter
                .record_failed(read_failed_entries.len() as i32)
                .await?;
        }

        if reporter.is_cancelled() {
            return Ok(());
        }

        // 9. Hapus missing documents (PlanDelete) dari Elasticsearch dan PostgreSQL registry
        if !plan.to_delete.is_empty() {
            let delete_ids: Vec<DocumentId> = plan.to_delete.iter().map(|d| d.id).collect();
            for del_id in &delete_ids {
                if reporter.is_cancelled() {
                    return Ok(());
                }
                let _ = self.repositories.search.delete_document(del_id).await;
            }
            self.repositories
                .registry
                .delete_entries(&delete_ids)
                .await?;
        }

        // 10. Kumpulkan seluruh target yang perlu diekstrak dan diindeks (to_add, to_update, to_move)
        struct IndexTarget {
            doc_id: DocumentId,
            relative_path: std::path::PathBuf,
            expected_hash: Option<String>,
        }

        let mut targets: Vec<IndexTarget> =
            Vec::with_capacity(plan.to_add.len() + plan.to_update.len() + plan.to_move.len());

        for add in &plan.to_add {
            targets.push(IndexTarget {
                doc_id: add.id,
                relative_path: add.relative_path.clone(),
                expected_hash: Some(add.content_hash.clone()),
            });
        }
        for upd in &plan.to_update {
            targets.push(IndexTarget {
                doc_id: upd.id,
                relative_path: upd.relative_path.clone(),
                expected_hash: Some(upd.new_hash.clone()),
            });
        }
        for mv in &plan.to_move {
            targets.push(IndexTarget {
                doc_id: mv.new_id,
                relative_path: mv.new_path.clone(),
                expected_hash: Some(mv.content_hash.clone()),
            });
        }

        let extract_options = ExtractOptions {
            max_file_size_bytes: Some(settings.max_file_size_bytes),
        };
        let extractor = DocumentExtractor::new();
        let read_options = ReadOptions::new(Some(settings.max_file_size_bytes));
        let now = Utc::now();

        const BULK_CHUNK_SIZE: usize = 100;
        let mut docs_to_index: Vec<IndexedDocument> = Vec::with_capacity(BULK_CHUNK_SIZE);
        let mut registry_to_upsert: Vec<RegistryEntry> = Vec::with_capacity(BULK_CHUNK_SIZE);

        for target in targets {
            if reporter.is_cancelled() {
                tracing::info!(job_id = %job_id, "Cancellation signaled: stopping new file acquisition");
                break;
            }

            let abs_path = folder.path.join(&target.relative_path);
            let norm_rel_path = normalize_path_str(&target.relative_path);

            // Baca konten berkas melalui FileReader port (isolasi kegagalan per berkas)
            let payload = match self.file_reader.read_file(&abs_path, &read_options).await {
                Ok(p) => p,
                Err(err) => {
                    let err_msg = format!("Failed to read file: {err}");
                    tracing::warn!(
                        path = %abs_path.display(),
                        error = %err,
                        "Failed to read file during scan execution"
                    );
                    let failed_entry = RegistryEntry {
                        id: target.doc_id,
                        folder_id: *folder_id,
                        relative_path: norm_rel_path.clone(),
                        content_hash: target.expected_hash.unwrap_or_default(),
                        file_size: 0,
                        status: DocumentStatus::Failed,
                        status_reason: Some(err_msg.clone()),
                        indexed_at: now,
                        updated_at: now,
                    };
                    let _ = self.repositories.registry.upsert_entry(&failed_entry).await;
                    self.event_dispatcher
                        .dispatch(&DomainEvent::document_failed(
                            *job_id,
                            norm_rel_path,
                            err_msg,
                        ));
                    reporter.record_failed(1).await?;
                    continue;
                }
            };

            // Ekstraksi konten dokumen dengan isolasi panic & failure
            let extraction_outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extractor.extract(
                    &target.relative_path,
                    &payload.bytes,
                    payload.size,
                    payload.modified_at,
                    &extract_options,
                )
            }));

            let extraction_result = match extraction_outcome {
                Ok(res) => res,
                Err(panic_payload) => {
                    let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic in document extractor".to_string()
                    };
                    let err_msg = format!("Extractor panicked: {panic_msg}");
                    tracing::error!(
                        path = %abs_path.display(),
                        error = %err_msg,
                        "Document extractor panicked at file boundary"
                    );
                    let failed_entry = RegistryEntry {
                        id: target.doc_id,
                        folder_id: *folder_id,
                        relative_path: norm_rel_path.clone(),
                        content_hash: payload.hash,
                        file_size: payload.size as i64,
                        status: DocumentStatus::Failed,
                        status_reason: Some(err_msg.clone()),
                        indexed_at: now,
                        updated_at: payload.modified_at.unwrap_or(now),
                    };
                    let _ = self.repositories.registry.upsert_entry(&failed_entry).await;
                    self.event_dispatcher
                        .dispatch(&DomainEvent::document_failed(
                            *job_id,
                            norm_rel_path,
                            err_msg,
                        ));
                    reporter.record_failed(1).await?;
                    continue;
                }
            };

            match extraction_result {
                ExtractionResult::Skipped(reason) => {
                    let reason_str = format!("{reason:?}");
                    tracing::debug!(
                        path = %target.relative_path.display(),
                        reason = ?reason,
                        "Document skipped by extractor"
                    );
                    let skipped_entry = RegistryEntry {
                        id: target.doc_id,
                        folder_id: *folder_id,
                        relative_path: norm_rel_path.clone(),
                        content_hash: payload.hash,
                        file_size: payload.size as i64,
                        status: DocumentStatus::Skipped,
                        status_reason: Some(reason_str.clone()),
                        indexed_at: now,
                        updated_at: payload.modified_at.unwrap_or(now),
                    };
                    let _ = self
                        .repositories
                        .registry
                        .upsert_entry(&skipped_entry)
                        .await;
                    self.event_dispatcher
                        .dispatch(&DomainEvent::document_skipped(
                            *job_id,
                            norm_rel_path,
                            reason_str,
                        ));
                    reporter.record_skipped(1).await?;
                }
                ExtractionResult::Extracted(extracted) => {
                    let indexed_doc = extracted.into_indexed_doc(
                        target.doc_id,
                        *folder_id,
                        norm_rel_path.clone(),
                        abs_path.to_string_lossy().into_owned(),
                        Some(payload.hash.clone()),
                        now,
                    );

                    let entry = RegistryEntry {
                        id: target.doc_id,
                        folder_id: *folder_id,
                        relative_path: norm_rel_path,
                        content_hash: payload.hash,
                        file_size: payload.size as i64,
                        status: DocumentStatus::Indexed,
                        status_reason: None,
                        indexed_at: now,
                        updated_at: payload.modified_at.unwrap_or(now),
                    };

                    docs_to_index.push(indexed_doc);
                    registry_to_upsert.push(entry);

                    if docs_to_index.len() >= BULK_CHUNK_SIZE {
                        self.flush_index_batch(
                            *job_id,
                            &docs_to_index,
                            &registry_to_upsert,
                            &mut reporter,
                        )
                        .await?;
                        docs_to_index.clear();
                        registry_to_upsert.clear();
                    }
                }
            }
        }

        // Flush active batch within grace policy
        if !docs_to_index.is_empty() {
            let flush_res = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                self.flush_index_batch(*job_id, &docs_to_index, &registry_to_upsert, &mut reporter),
            )
            .await;
            match flush_res {
                Ok(Ok(())) => {
                    tracing::info!(job_id = %job_id, count = docs_to_index.len(), "Active batch flushed successfully within grace period");
                }
                Ok(Err(e)) => {
                    tracing::warn!(job_id = %job_id, error = %e, "Failed to flush active batch during cancellation");
                }
                Err(_) => {
                    tracing::warn!(job_id = %job_id, "Active batch flush timed out under grace period");
                }
            }
            docs_to_index.clear();
            registry_to_upsert.clear();
        }

        if reporter.is_cancelled() {
            tracing::info!(job_id = %job_id, "Job cancelled: stopping remaining scan operations");
            reporter.finish_cancelled().await?;
            return Ok(());
        }

        // 11. Hapus old_id untuk file yang dipindahkan/direname (PlanMove) setelah path baru berhasil diindeks
        if !plan.to_move.is_empty() {
            let old_move_ids: Vec<DocumentId> = plan.to_move.iter().map(|m| m.old_id).collect();
            for old_id in &old_move_ids {
                if reporter.is_cancelled() {
                    return Ok(());
                }
                let _ = self.repositories.search.delete_document(old_id).await;
            }
            self.repositories
                .registry
                .delete_entries(&old_move_ids)
                .await?;
        }

        // 12. Update last_scanned_at pada folder
        self.repositories
            .folder
            .update_last_scanned(folder_id)
            .await?;

        // 13. Selesaikan job dengan status Completed
        reporter.finish_completed().await?;

        Ok(())
    }

    async fn flush_index_batch(
        &self,
        job_id: JobId,
        docs: &[IndexedDocument],
        registry_entries: &[RegistryEntry],
        reporter: &mut JobProgressReporter,
    ) -> Result<(), AppError> {
        if docs.is_empty() {
            return Ok(());
        }

        let report = self.repositories.search.bulk_index_documents(docs).await?;

        // Upsert ke PostgreSQL document registry
        self.repositories
            .registry
            .upsert_batch(registry_entries)
            .await?;

        // Update progress counters (auto-flushes ke DB setiap 100 dokumen)
        if report.indexed > 0 {
            reporter.record_indexed(report.indexed as i32).await?;
            for doc in docs.iter().take(report.indexed) {
                self.event_dispatcher
                    .dispatch(&DomainEvent::document_indexed(
                        job_id,
                        doc.relative_path.clone(),
                    ));
            }
        }
        if report.failed > 0 {
            reporter.record_failed(report.failed as i32).await?;
            for err in &report.errors {
                self.event_dispatcher
                    .dispatch(&DomainEvent::document_failed(
                        job_id,
                        "bulk_index_item".to_string(),
                        err.clone(),
                    ));
            }
        }

        Ok(())
    }

    async fn dispatch_cancel_job(&self, job_id: JobId) {
        self.job_tracker.cancel_job(&job_id);
        let _ = self.repositories.job.mark_cancelled(&job_id).await;
        tracing::info!(job_id = %job_id, "Dispatched job cancellation");
    }

    async fn dispatch_rebuild_index(
        &self,
        job_id: JobId,
        target_index: String,
    ) -> Result<(), AppError> {
        if matches!(self.job_tracker.get_progress(&job_id), Some(state) if state.status == JobStatus::Cancelled)
        {
            tracing::info!(job_id = %job_id, "Rebuild job was cancelled before execution");
            self.job_tracker.release_rebuild_lock();
            return Ok(());
        }

        let _fallback_guard = if !self.job_tracker.is_rebuilding() {
            match self.job_tracker.try_acquire_rebuild_lock() {
                Ok(guard) => Some(guard),
                Err(err) => {
                    let err_msg = "Job conflict: folder scan or rebuild in progress";
                    self.job_tracker.update_progress(
                        &job_id,
                        JobProgressUpdate {
                            status: Some(JobStatus::Failed),
                            error_summary: Some(err_msg.into()),
                            ..Default::default()
                        },
                    );
                    let _ = self
                        .repositories
                        .job
                        .update_progress(
                            &job_id,
                            &JobProgressUpdate {
                                status: Some(JobStatus::Failed),
                                error_summary: Some(err_msg.into()),
                                ..Default::default()
                            },
                        )
                        .await;
                    return Err(err);
                }
            }
        } else {
            None
        };

        let mut reporter = self.create_progress_reporter(job_id, Some(50));
        reporter.mark_running(None).await?;

        tracing::info!(job_id = %job_id, target_index = %target_index, "Executing index rebuild");

        let alias = self.repositories.search.search_alias();
        let target_version = crate::infrastructure::elasticsearch::parse_index_version(&target_index, alias)
            .unwrap_or(1);

        let created_index = match self.repositories.search.create_versioned_index(target_version).await {
            Ok(idx) => idx,
            Err(err) => {
                let err_msg = format!("Failed to create physical index '{target_index}': {err}");
                tracing::error!(job_id = %job_id, error = %err_msg, "Rebuild failed at index creation");
                let _ = reporter.finish_failed(&err_msg).await;
                self.job_tracker.release_rebuild_lock();
                drop(_fallback_guard);
                return Err(err);
            }
        };

        let settings = self.repositories.settings.get_settings().await.unwrap_or_default();
        let read_options = ReadOptions::new(Some(settings.max_file_size_bytes));
        let extract_options = ExtractOptions {
            max_file_size_bytes: Some(settings.max_file_size_bytes),
        };
        let extractor = DocumentExtractor::new();

        let folders = self.repositories.folder.list_folders().await.unwrap_or_default();
        let folder_map: std::collections::HashMap<FolderId, Folder> =
            folders.into_iter().map(|f| (f.id, f)).collect();

        let entries = match self
            .repositories
            .registry
            .list_by_status(DocumentStatus::Indexed)
            .await
        {
            Ok(ents) => ents,
            Err(err) => {
                let err_msg = format!("Failed to query registry for eligible documents: {err}");
                tracing::error!(job_id = %job_id, error = %err_msg, "Rebuild failed querying registry");
                let _ = self.repositories.search.delete_index(&created_index).await;
                let _ = reporter.finish_failed(&err_msg).await;
                self.job_tracker.release_rebuild_lock();
                drop(_fallback_guard);
                return Err(err);
            }
        };

        let total_docs = entries.len() as i32;
        reporter.set_files_total(total_docs).await?;

        const REBUILD_CHUNK_SIZE: usize = 200;
        let mut docs_to_index: Vec<IndexedDocument> = Vec::with_capacity(REBUILD_CHUNK_SIZE);
        let mut rebuild_failed = false;
        let mut failure_reason = String::new();
        let now = Utc::now();

        for entry in entries {
            if reporter.is_cancelled() {
                tracing::info!(job_id = %job_id, "Rebuild cancelled during streaming");
                break;
            }

            let folder = match folder_map.get(&entry.folder_id) {
                Some(f) => f,
                None => {
                    let _ = reporter.record_skipped(1).await;
                    continue;
                }
            };

            let abs_path = folder.path.join(&entry.relative_path);
            let payload = match self.file_reader.read_file(&abs_path, &read_options).await {
                Ok(p) => p,
                Err(err) => {
                    tracing::warn!(
                        path = %abs_path.display(),
                        error = %err,
                        "Failed to read file during rebuild"
                    );
                    let _ = reporter.record_failed(1).await;
                    continue;
                }
            };

            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extractor.extract(
                    std::path::Path::new(&entry.relative_path),
                    &payload.bytes,
                    payload.size,
                    payload.modified_at,
                    &extract_options,
                )
            }));

            match outcome {
                Ok(ExtractionResult::Extracted(extracted)) => {
                    let norm_rel_path = normalize_path_str(&entry.relative_path);
                    let indexed_doc = extracted.into_indexed_doc(
                        entry.id,
                        entry.folder_id,
                        norm_rel_path,
                        abs_path.to_string_lossy().into_owned(),
                        Some(payload.hash),
                        now,
                    );
                    docs_to_index.push(indexed_doc);
                }
                Ok(ExtractionResult::Skipped(_)) => {
                    let _ = reporter.record_skipped(1).await;
                }
                Err(_) => {
                    let _ = reporter.record_failed(1).await;
                }
            }

            if docs_to_index.len() >= REBUILD_CHUNK_SIZE {
                match self
                    .repositories
                    .search
                    .bulk_index_to_target(&created_index, &docs_to_index)
                    .await
                {
                    Ok(report) => {
                        let _ = reporter.record_indexed(report.indexed as i32).await;
                        if report.failed > 0 {
                            let _ = reporter.record_failed(report.failed as i32).await;
                        }
                        docs_to_index.clear();
                    }
                    Err(err) => {
                        failure_reason =
                            format!("Bulk index error into '{created_index}': {err}");
                        rebuild_failed = true;
                        break;
                    }
                }
            }
        }

        if !rebuild_failed && !reporter.is_cancelled() && !docs_to_index.is_empty() {
            match self
                .repositories
                .search
                .bulk_index_to_target(&created_index, &docs_to_index)
                .await
            {
                Ok(report) => {
                    let _ = reporter.record_indexed(report.indexed as i32).await;
                    if report.failed > 0 {
                        let _ = reporter.record_failed(report.failed as i32).await;
                    }
                    docs_to_index.clear();
                }
                Err(err) => {
                    failure_reason =
                        format!("Bulk index error into '{created_index}': {err}");
                    rebuild_failed = true;
                }
            }
        }

        if reporter.is_cancelled() {
            tracing::warn!(
                job_id = %job_id,
                "Rebuild was cancelled. Keeping old alias active and removing incomplete index"
            );
            let _ = self.repositories.search.delete_index(&created_index).await;
            let _ = reporter.finish_cancelled().await;
            self.job_tracker.release_rebuild_lock();
            drop(_fallback_guard);
            return Ok(());
        }

        if rebuild_failed {
            tracing::error!(
                job_id = %job_id,
                reason = %failure_reason,
                "Rebuild failed during bulk indexing. Keeping old alias active"
            );
            let _ = self.repositories.search.delete_index(&created_index).await;
            let _ = reporter.finish_failed(&failure_reason).await;
            self.job_tracker.release_rebuild_lock();
            drop(_fallback_guard);
            return Err(AppError::SearchEngine(failure_reason));
        }

        match self
            .repositories
            .search
            .rebuild_index_with_alias(&created_index, alias)
            .await
        {
            Ok(()) => {
                tracing::info!(
                    job_id = %job_id,
                    target_index = %created_index,
                    "Zero-downtime atomic alias swap completed successfully"
                );
                reporter.finish_completed().await?;
                self.job_tracker.release_rebuild_lock();
                drop(_fallback_guard);
                Ok(())
            }
            Err(err) => {
                let err_msg = format!("Atomic alias swap failed: {err}");
                tracing::error!(
                    job_id = %job_id,
                    error = %err_msg,
                    "Rebuild swap failed. Keeping old alias active"
                );
                let _ = self.repositories.search.delete_index(&created_index).await;
                let _ = reporter.finish_failed(&err_msg).await;
                self.job_tracker.release_rebuild_lock();
                drop(_fallback_guard);
                Err(err)
            }
        }
    }
}
