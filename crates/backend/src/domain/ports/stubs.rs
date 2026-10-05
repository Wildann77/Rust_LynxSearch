use super::{
    DocumentRegistryRepository, FolderRepository, JobRepository, SearchRepository,
    SettingsRepository,
    file_system::{DiscoveredFile, FilePayload, FileReader, FileWalker, ReadOptions, WalkOptions},
};
use crate::config::Bm25Weights;
use crate::domain::models::{
    AppSettings, BulkIndexReport, DocumentId, DocumentStatus, Folder, FolderId, FolderStatus,
    IndexedDocument, IndexingJob, JobId, JobProgressUpdate, JobStatus, RegistryEntry,
    SearchRawResponse,
};
use crate::error::AppError;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct InMemoryFolderRepository {
    folders: Arc<RwLock<HashMap<FolderId, Folder>>>,
    document_counts: Arc<RwLock<HashMap<FolderId, u64>>>,
}

impl InMemoryFolderRepository {
    pub fn set_document_count(&self, id: FolderId, count: u64) {
        self.document_counts.write().insert(id, count);
    }
}

#[async_trait]
impl FolderRepository for InMemoryFolderRepository {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError> {
        self.folders.write().insert(folder.id, folder.clone());
        Ok(())
    }

    async fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, AppError> {
        Ok(self.folders.read().get(id).cloned())
    }

    async fn find_by_path(&self, path: &Path) -> Result<Option<Folder>, AppError> {
        Ok(self
            .folders
            .read()
            .values()
            .find(|f| f.path == path)
            .cloned())
    }

    async fn list_folders(&self) -> Result<Vec<Folder>, AppError> {
        let mut list: Vec<Folder> = self.folders.read().values().cloned().collect();
        list.sort_by_key(|f| f.created_at);
        Ok(list)
    }

    async fn list_folders_with_counts(&self) -> Result<Vec<(Folder, u64)>, AppError> {
        let folders = self.folders.read();
        let counts = self.document_counts.read();
        let mut list: Vec<(Folder, u64)> = folders
            .values()
            .map(|f| {
                let count = counts.get(&f.id).copied().unwrap_or(0);
                (f.clone(), count)
            })
            .collect();
        list.sort_by_key(|(f, _)| f.created_at);
        Ok(list)
    }

    async fn delete_folder(&self, id: &FolderId) -> Result<(), AppError> {
        self.folders.write().remove(id);
        self.document_counts.write().remove(id);
        Ok(())
    }

    async fn update_last_scanned(&self, id: &FolderId) -> Result<(), AppError> {
        if let Some(folder) = self.folders.write().get_mut(id) {
            folder.last_scanned_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn update_status(&self, id: &FolderId, status: FolderStatus) -> Result<(), AppError> {
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
    entries: Arc<RwLock<HashMap<DocumentId, RegistryEntry>>>,
}

#[async_trait]
impl DocumentRegistryRepository for InMemoryDocumentRegistryRepository {
    async fn get_entry(&self, doc_id: &DocumentId) -> Result<Option<RegistryEntry>, AppError> {
        Ok(self.entries.read().get(doc_id).cloned())
    }

    async fn list_by_folder(&self, folder_id: &FolderId) -> Result<Vec<RegistryEntry>, AppError> {
        let entries = self
            .entries
            .read()
            .values()
            .filter(|e| e.folder_id == *folder_id)
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn list_by_status(&self, status: DocumentStatus) -> Result<Vec<RegistryEntry>, AppError> {
        let entries = self
            .entries
            .read()
            .values()
            .filter(|e| e.status == status)
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError> {
        self.entries.write().insert(entry.id, entry.clone());
        Ok(())
    }

    async fn upsert_batch(&self, entries: &[RegistryEntry]) -> Result<u64, AppError> {
        let mut count = 0;
        let mut store = self.entries.write();
        for entry in entries {
            store.insert(entry.id, entry.clone());
            count += 1;
        }
        Ok(count)
    }

    async fn delete_entries(&self, ids: &[DocumentId]) -> Result<u64, AppError> {
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
    jobs: Arc<RwLock<HashMap<JobId, IndexingJob>>>,
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError> {
        self.jobs.write().insert(job.id, job.clone());
        Ok(())
    }

    async fn update_progress(
        &self,
        id: &JobId,
        update: &JobProgressUpdate,
    ) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id) {
            if let Some(status) = update.status
                && (!job.status.is_terminal() || job.status == status)
            {
                job.status = status;
            }
            if let Some(total) = update.files_total {
                job.files_total = total;
            }
            job.files_indexed += update.indexed_delta;
            job.files_skipped += update.skipped_delta;
            job.files_failed += update.failed_delta;
            job.files_processed += update.processed_delta;
            let sum_sub = job.files_indexed + job.files_skipped + job.files_failed;
            if sum_sub > job.files_processed {
                job.files_processed = sum_sub;
            }
            if update.error_summary.is_some() {
                job.error_summary = update.error_summary.clone();
            }
            if job.status.is_terminal() && job.completed_at.is_none() {
                job.completed_at = Some(chrono::Utc::now());
            }
        }
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<IndexingJob>, AppError> {
        Ok(self.jobs.read().get(id).cloned())
    }

    async fn mark_cancelled(&self, id: &JobId) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id)
            && !job.status.is_terminal()
        {
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

    async fn reset_settings(&self) -> Result<AppSettings, AppError> {
        let default_settings = AppSettings::default();
        *self.settings.write() = default_settings.clone();
        Ok(default_settings)
    }
}

#[derive(Default)]
pub struct InMemorySearchRepository {
    documents: Arc<RwLock<HashMap<DocumentId, IndexedDocument>>>,
    active_index: Arc<RwLock<Option<String>>>,
    indices: Arc<RwLock<HashSet<String>>>,
}

#[async_trait]
impl SearchRepository for InMemorySearchRepository {
    fn search_alias(&self) -> &str {
        "lynx_documents"
    }

    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError> {
        self.documents.write().insert(doc.id, doc.clone());
        Ok(())
    }

    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        self.bulk_index_to_target("lynx_documents", docs).await
    }

    async fn bulk_index_to_target(
        &self,
        target_index: &str,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        let mut write = self.documents.write();
        for doc in docs {
            write.insert(doc.id, doc.clone());
        }
        self.indices.write().insert(target_index.to_string());
        Ok(BulkIndexReport {
            indexed: docs.len(),
            failed: 0,
            errors: Vec::new(),
        })
    }

    async fn delete_document(&self, id: &DocumentId) -> Result<(), AppError> {
        self.documents.write().remove(id);
        Ok(())
    }

    async fn delete_documents_by_folder(&self, folder_id: &FolderId) -> Result<u64, AppError> {
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

    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), AppError> {
        let old_indices = self.get_alias_indices(alias).await?;
        self.swap_alias(alias, &old_indices, new_index).await?;
        let current_indices = self.get_alias_indices(alias).await?;
        if !current_indices.contains(&new_index.to_string()) {
            return Err(AppError::SearchEngine(format!(
                "Alias swap verification failed: alias '{alias}' does not point to '{new_index}'"
            )));
        }
        for old in &old_indices {
            if old != new_index {
                self.delete_index(old).await?;
            }
        }
        Ok(())
    }

    async fn ping(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn ensure_initial_index(&self) -> Result<String, AppError> {
        let mut active = self.active_index.write();
        if let Some(ref current) = *active {
            Ok(current.clone())
        } else {
            let initial = "lynx_documents_v1".to_string();
            *active = Some(initial.clone());
            self.indices.write().insert(initial.clone());
            Ok(initial)
        }
    }

    async fn get_active_physical_index(&self) -> Result<Option<String>, AppError> {
        Ok(self.active_index.read().clone())
    }

    async fn create_versioned_index(&self, version: u32) -> Result<String, AppError> {
        let name = format!("lynx_documents_v{version}");
        self.indices.write().insert(name.clone());
        Ok(name)
    }

    async fn get_alias_indices(&self, _alias: &str) -> Result<Vec<String>, AppError> {
        Ok(self.active_index.read().clone().into_iter().collect())
    }

    async fn swap_alias(
        &self,
        _alias: &str,
        _old_indices: &[String],
        new_index: &str,
    ) -> Result<(), AppError> {
        self.indices.write().insert(new_index.to_string());
        *self.active_index.write() = Some(new_index.to_string());
        Ok(())
    }

    async fn delete_index(&self, index_name: &str) -> Result<(), AppError> {
        self.indices.write().remove(index_name);
        let mut active = self.active_index.write();
        if active.as_deref() == Some(index_name) {
            *active = None;
        }
        Ok(())
    }
}

/// Stub in-memory untuk FileWalker untuk keperluan domain & usecase testing tanpa akses filesystem fisik.
#[derive(Default, Clone)]
pub struct InMemoryFileWalker {
    files: Arc<RwLock<Vec<DiscoveredFile>>>,
}

impl InMemoryFileWalker {
    pub fn new(files: Vec<DiscoveredFile>) -> Self {
        Self {
            files: Arc::new(RwLock::new(files)),
        }
    }

    pub fn add_file(&self, file: DiscoveredFile) {
        self.files.write().push(file);
    }
}

impl FileWalker for InMemoryFileWalker {
    fn walk<'a>(
        &'a self,
        _root: &'a Path,
        _options: &'a WalkOptions,
    ) -> Result<Box<dyn Iterator<Item = Result<DiscoveredFile, AppError>> + Send + 'a>, AppError>
    {
        let files = self.files.read().clone();
        Ok(Box::new(files.into_iter().map(Ok)))
    }
}

/// Stub in-memory untuk FileReader untuk keperluan domain & usecase testing tanpa akses filesystem fisik.
#[derive(Default, Clone)]
pub struct InMemoryFileReader {
    files: Arc<RwLock<HashMap<std::path::PathBuf, FilePayload>>>,
}

impl InMemoryFileReader {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_payload(&self, payload: FilePayload) {
        self.files
            .write()
            .insert(payload.absolute_path.clone(), payload);
    }

    pub fn add_file(
        &self,
        absolute_path: impl Into<std::path::PathBuf>,
        relative_path: Option<impl Into<std::path::PathBuf>>,
        bytes: Vec<u8>,
    ) {
        let abs = absolute_path.into();
        let rel = relative_path.map(|r| r.into());
        let payload = FilePayload::new(abs.clone(), rel, bytes, Some(chrono::Utc::now()));
        self.files.write().insert(abs, payload);
    }
}

#[async_trait]
impl FileReader for InMemoryFileReader {
    async fn read_file(&self, path: &Path, options: &ReadOptions) -> Result<FilePayload, AppError> {
        let files = self.files.read();
        let payload = files.get(path).cloned().ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found in in-memory reader: {}", path.display()),
            ))
        })?;

        if let Some(limit) = options.max_file_size_bytes
            && payload.size > limit
        {
            return Err(AppError::ValidationFailed(format!(
                "File '{}' exceeds maximum allowed size: {} > {} bytes",
                path.display(),
                payload.size,
                limit
            )));
        }

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{DocumentId, DocumentStatus, FolderId, RegistryEntry};
    use crate::domain::ports::compute_sha256;
    use chrono::Utc;

    #[tokio::test]
    async fn test_in_memory_registry_upsert_batch() {
        let repo = InMemoryDocumentRegistryRepository::default();
        let folder_id = FolderId::new();
        let doc1 = RegistryEntry {
            id: DocumentId::from_relative_path(folder_id, "file1.rs"),
            folder_id,
            relative_path: "file1.rs".into(),
            content_hash: "hash1".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let doc2 = RegistryEntry {
            id: DocumentId::from_relative_path(folder_id, "file2.rs"),
            folder_id,
            relative_path: "file2.rs".into(),
            content_hash: "hash2".into(),
            file_size: 200,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let inserted = repo
            .upsert_batch(&[doc1.clone(), doc2.clone()])
            .await
            .unwrap();
        assert_eq!(inserted, 2);

        let list = repo.list_by_folder(&folder_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_settings_reset() {
        let repo = InMemorySettingsRepository::default();
        let mut modified = repo.get_settings().await.unwrap();
        modified.max_file_size_bytes = 999999;
        repo.update_settings(&modified).await.unwrap();

        let current = repo.get_settings().await.unwrap();
        assert_eq!(current.max_file_size_bytes, 999999);

        let reset = repo.reset_settings().await.unwrap();
        assert_eq!(reset.max_file_size_bytes, 2 * 1024 * 1024);

        let after_reset = repo.get_settings().await.unwrap();
        assert_eq!(after_reset.max_file_size_bytes, 2 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_in_memory_search_repo_initial_index_and_active_index() {
        let repo = InMemorySearchRepository::default();
        assert_eq!(repo.get_active_physical_index().await.unwrap(), None);

        let initial = repo.ensure_initial_index().await.unwrap();
        assert_eq!(initial, "lynx_documents_v1");
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v1".to_string())
        );

        // Subsequent ensure call should return the already active index (idempotent)
        let second = repo.ensure_initial_index().await.unwrap();
        assert_eq!(second, "lynx_documents_v1");

        // Simulating rebuild swap
        repo.rebuild_index_with_alias("lynx_documents_v2", "lynx_documents")
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v2".to_string())
        );
    }

    #[tokio::test]
    async fn test_in_memory_search_repo_reindex_alias_contract() {
        let repo = InMemorySearchRepository::default();

        // 1. Initial index creation
        let v1 = repo.ensure_initial_index().await.unwrap();
        assert_eq!(v1, "lynx_documents_v1");

        // 2. Create versioned index
        let v2 = repo.create_versioned_index(2).await.unwrap();
        assert_eq!(v2, "lynx_documents_v2");

        // 3. Inspect alias
        let indices = repo.get_alias_indices("lynx_documents").await.unwrap();
        assert_eq!(indices, vec!["lynx_documents_v1".to_string()]);

        // 4. Swap alias
        repo.swap_alias("lynx_documents", &indices, &v2)
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v2".to_string())
        );

        // 5. Cleanup old index
        repo.delete_index(&v1).await.unwrap();

        // 6. Test rebuild_index_with_alias helper
        let v3 = repo.create_versioned_index(3).await.unwrap();
        repo.rebuild_index_with_alias(&v3, "lynx_documents")
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v3".to_string())
        );
    }

    #[test]
    fn test_in_memory_file_walker() {
        let walker = InMemoryFileWalker::default();
        walker.add_file(DiscoveredFile {
            relative_path: "src/main.rs".into(),
            absolute_path: "/project/src/main.rs".into(),
            file_size: 1024,
            modified_at: Some(Utc::now()),
        });

        let files = walker
            .walk_all(Path::new("/project"), &WalkOptions::default())
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].relative_path,
            std::path::PathBuf::from("src/main.rs")
        );
    }

    #[tokio::test]
    async fn test_in_memory_file_reader() {
        let reader = InMemoryFileReader::new();
        let path = Path::new("/project/README.md");
        let content = b"# LynxSearch".to_vec();

        reader.add_file(path, Some("README.md"), content.clone());

        // 1. Read file successfully
        let payload = reader
            .read_file(path, &ReadOptions::default())
            .await
            .unwrap();
        assert_eq!(payload.bytes, content);
        assert_eq!(payload.size, content.len() as u64);
        assert_eq!(payload.hash, compute_sha256(&content));
        assert_eq!(
            payload.relative_path,
            Some(std::path::PathBuf::from("README.md"))
        );

        // 2. Read file with max size limit exceeded
        let strict_opts = ReadOptions::new(Some(5));
        let err = reader.read_file(path, &strict_opts).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationFailed(_)));

        // 3. File not found
        let not_found = reader
            .read_file(Path::new("/unknown.md"), &ReadOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(not_found, AppError::Io(_)));
    }
}
