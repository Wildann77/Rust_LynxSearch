use super::{
    DocumentRegistryRepository, FolderRepository, JobRepository, SearchRepository,
    SettingsRepository,
};
use crate::config::Bm25Weights;
use crate::domain::models::{
    AppSettings, BulkIndexReport, Folder, FolderStatus, IndexedDocument, IndexingJob,
    JobProgressUpdate, JobStatus, RegistryEntry, SearchRawResponse,
};
use crate::error::AppError;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Default)]
pub struct InMemoryFolderRepository {
    folders: Arc<RwLock<HashMap<Uuid, Folder>>>,
}

#[async_trait]
impl FolderRepository for InMemoryFolderRepository {
    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError> {
        self.folders.write().insert(folder.id, folder.clone());
        Ok(())
    }

    async fn get_folder(&self, id: &Uuid) -> Result<Option<Folder>, AppError> {
        Ok(self.folders.read().get(id).cloned())
    }

    async fn list_folders(&self) -> Result<Vec<Folder>, AppError> {
        Ok(self.folders.read().values().cloned().collect())
    }

    async fn delete_folder(&self, id: &Uuid) -> Result<(), AppError> {
        self.folders.write().remove(id);
        Ok(())
    }

    async fn update_last_scanned(&self, id: &Uuid) -> Result<(), AppError> {
        if let Some(folder) = self.folders.write().get_mut(id) {
            folder.last_scanned_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn update_status(&self, id: &Uuid, status: FolderStatus) -> Result<(), AppError> {
        if let Some(folder) = self.folders.write().get_mut(id) {
            folder.status = status;
        }
        Ok(())
    }

    async fn reset_scanning_folders(&self) -> Result<u64, AppError> {
        let mut count = 0;
        let mut folders = self.folders.write();
        for folder in folders.values_mut() {
            if folder.status == FolderStatus::Scanning {
                folder.status = FolderStatus::Idle;
                count += 1;
            }
        }
        Ok(count)
    }
}

#[derive(Default)]
pub struct InMemoryDocumentRegistryRepository {
    entries: Arc<RwLock<HashMap<Uuid, RegistryEntry>>>,
}

#[async_trait]
impl DocumentRegistryRepository for InMemoryDocumentRegistryRepository {
    async fn get_entry(&self, doc_id: &Uuid) -> Result<Option<RegistryEntry>, AppError> {
        Ok(self.entries.read().get(doc_id).cloned())
    }

    async fn list_by_folder(&self, folder_id: &Uuid) -> Result<Vec<RegistryEntry>, AppError> {
        let entries = self
            .entries
            .read()
            .values()
            .filter(|e| e.folder_id == *folder_id)
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError> {
        self.entries.write().insert(entry.id, entry.clone());
        Ok(())
    }

    async fn delete_entries(&self, ids: &[Uuid]) -> Result<u64, AppError> {
        let mut count = 0;
        let mut entries = self.entries.write();
        for id in ids {
            if entries.remove(id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }
}

#[derive(Default)]
pub struct InMemoryJobRepository {
    jobs: Arc<RwLock<HashMap<Uuid, IndexingJob>>>,
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError> {
        self.jobs.write().insert(job.id, job.clone());
        Ok(())
    }

    async fn update_progress(&self, id: &Uuid, update: &JobProgressUpdate) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id) {
            if let Some(status) = update.status {
                job.status = status;
            }
            if let Some(total) = update.files_total {
                job.files_total = total;
            }
            job.files_processed += update.processed_delta;
            job.files_indexed += update.indexed_delta;
            job.files_skipped += update.skipped_delta;
            job.files_failed += update.failed_delta;
            if update.error_summary.is_some() {
                job.error_summary = update.error_summary.clone();
            }
            if job.status == JobStatus::Completed
                || job.status == JobStatus::Failed
                || job.status == JobStatus::Cancelled
            {
                job.completed_at = Some(chrono::Utc::now());
            }
        }
        Ok(())
    }

    async fn get_job(&self, id: &Uuid) -> Result<Option<IndexingJob>, AppError> {
        Ok(self.jobs.read().get(id).cloned())
    }

    async fn mark_cancelled(&self, id: &Uuid) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id) {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn find_dangling_jobs(&self) -> Result<Vec<IndexingJob>, AppError> {
        let dangling = self
            .jobs
            .read()
            .values()
            .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Pending)
            .cloned()
            .collect();
        Ok(dangling)
    }

    async fn recover_dangling_jobs(&self, error_summary: &str) -> Result<u64, AppError> {
        let mut count = 0;
        let mut jobs = self.jobs.write();
        let now = chrono::Utc::now();
        for job in jobs.values_mut() {
            if job.status == JobStatus::Running || job.status == JobStatus::Pending {
                job.status = JobStatus::Failed;
                job.completed_at = Some(now);
                job.error_summary = Some(error_summary.to_string());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn cancel_unfinished_jobs(&self, reason: &str) -> Result<u64, AppError> {
        let mut count = 0;
        let mut jobs = self.jobs.write();
        let now = chrono::Utc::now();
        for job in jobs.values_mut() {
            if job.status == JobStatus::Running || job.status == JobStatus::Pending {
                job.status = JobStatus::Cancelled;
                job.completed_at = Some(now);
                job.error_summary = Some(reason.to_string());
                count += 1;
            }
        }
        Ok(count)
    }
}

pub struct InMemorySettingsRepository {
    settings: Arc<RwLock<AppSettings>>,
}

impl Default for InMemorySettingsRepository {
    fn default() -> Self {
        Self {
            settings: Arc::new(RwLock::new(AppSettings {
                max_file_size_bytes: 2 * 1024 * 1024,
                weights: Bm25Weights {
                    title: 3.0,
                    tags: 2.0,
                    content: 1.0,
                },
                ignore_patterns: vec![
                    ".git".into(),
                    "node_modules".into(),
                    "target".into(),
                    "dist".into(),
                    "build".into(),
                ],
            })),
        }
    }
}

impl InMemorySettingsRepository {
    pub fn new(initial: AppSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(initial)),
        }
    }
}

#[async_trait]
impl SettingsRepository for InMemorySettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings, AppError> {
        Ok(self.settings.read().clone())
    }

    async fn update_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
        *self.settings.write() = settings.clone();
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemorySearchRepository {
    documents: Arc<RwLock<HashMap<Uuid, IndexedDocument>>>,
}

#[async_trait]
impl SearchRepository for InMemorySearchRepository {
    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError> {
        self.documents.write().insert(doc.id, doc.clone());
        Ok(())
    }

    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        let mut write = self.documents.write();
        for doc in docs {
            write.insert(doc.id, doc.clone());
        }
        Ok(BulkIndexReport {
            indexed: docs.len(),
            failed: 0,
            errors: Vec::new(),
        })
    }

    async fn delete_document(&self, id: &Uuid) -> Result<(), AppError> {
        self.documents.write().remove(id);
        Ok(())
    }

    async fn delete_documents_by_folder(&self, folder_id: &Uuid) -> Result<u64, AppError> {
        let mut write = self.documents.write();
        let initial_len = write.len();
        write.retain(|_, doc| doc.folder_id != *folder_id);
        Ok((initial_len - write.len()) as u64)
    }

    async fn search(
        &self,
        _query_dsl: &serde_json::value::RawValue,
    ) -> Result<SearchRawResponse, AppError> {
        Ok(SearchRawResponse {
            raw_json: "{\"hits\":{\"total\":{\"value\":0},\"hits\":[]}}".into(),
        })
    }

    async fn suggest(&self, _prefix: &str, _limit: usize) -> Result<Vec<String>, AppError> {
        Ok(Vec::new())
    }

    async fn rebuild_index_with_alias(
        &self,
        _new_index: &str,
        _alias: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn ping(&self) -> Result<(), AppError> {
        Ok(())
    }
}
