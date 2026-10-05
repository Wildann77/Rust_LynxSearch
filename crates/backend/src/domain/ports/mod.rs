pub mod file_system;
pub mod stubs;

pub use file_system::{
    DiscoveredFile, FilePayload, FileReader, FileWalker, ReadOptions, WalkOptions, compute_sha256,
};

use crate::domain::models::{
    AppSettings, BulkIndexReport, DocumentId, DocumentStatus, Folder, FolderId, FolderStatus,
    IndexedDocument, IndexingJob, JobId, JobProgressUpdate, RegistryEntry, SearchRawResponse,
};
use crate::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait FolderRepository: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError>;
    async fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, AppError>;
    async fn find_by_path(&self, path: &std::path::Path) -> Result<Option<Folder>, AppError>;
    async fn list_folders(&self) -> Result<Vec<Folder>, AppError>;
    async fn list_folders_with_counts(&self) -> Result<Vec<(Folder, u64)>, AppError>;
    async fn delete_folder(&self, id: &FolderId) -> Result<(), AppError>;
    async fn update_last_scanned(&self, id: &FolderId) -> Result<(), AppError>;
    async fn update_status(&self, id: &FolderId, status: FolderStatus) -> Result<(), AppError>;
    async fn reset_scanning_folders(&self) -> Result<u64, AppError>;
}

#[async_trait]
pub trait DocumentRegistryRepository: Send + Sync {
    async fn get_entry(&self, doc_id: &DocumentId) -> Result<Option<RegistryEntry>, AppError>;
    async fn list_by_folder(&self, folder_id: &FolderId) -> Result<Vec<RegistryEntry>, AppError>;
    async fn list_by_status(&self, status: DocumentStatus) -> Result<Vec<RegistryEntry>, AppError>;
    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError>;
    async fn upsert_batch(&self, entries: &[RegistryEntry]) -> Result<u64, AppError>;
    async fn delete_entries(&self, ids: &[DocumentId]) -> Result<u64, AppError>;
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError>;
    async fn update_progress(&self, id: &JobId, update: &JobProgressUpdate)
    -> Result<(), AppError>;
    async fn get_job(&self, id: &JobId) -> Result<Option<IndexingJob>, AppError>;
    async fn mark_cancelled(&self, id: &JobId) -> Result<(), AppError>;
    async fn find_dangling_jobs(&self) -> Result<Vec<IndexingJob>, AppError>;
    async fn recover_dangling_jobs(&self, error_summary: &str) -> Result<u64, AppError>;
    async fn cancel_unfinished_jobs(&self, reason: &str) -> Result<u64, AppError>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_settings(&self) -> Result<AppSettings, AppError>;
    async fn update_settings(&self, settings: &AppSettings) -> Result<(), AppError>;
    async fn reset_settings(&self) -> Result<AppSettings, AppError>;
}

#[async_trait]
pub trait SearchRepository: Send + Sync {
    fn search_alias(&self) -> &str;
    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError>;
    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError>;
    async fn bulk_index_to_target(
        &self,
        target_index: &str,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError>;
    async fn delete_document(&self, id: &DocumentId) -> Result<(), AppError>;
    async fn delete_documents_by_folder(&self, folder_id: &FolderId) -> Result<u64, AppError>;
    async fn search(
        &self,
        query_dsl: &serde_json::value::RawValue,
    ) -> Result<SearchRawResponse, AppError>;
    async fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>, AppError>;
    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), AppError>;
    async fn ping(&self) -> Result<(), AppError>;
    async fn ensure_initial_index(&self) -> Result<String, AppError>;
    async fn get_active_physical_index(&self) -> Result<Option<String>, AppError>;
    async fn create_versioned_index(&self, version: u32) -> Result<String, AppError>;
    async fn get_alias_indices(&self, alias: &str) -> Result<Vec<String>, AppError>;
    async fn swap_alias(
        &self,
        alias: &str,
        old_indices: &[String],
        new_index: &str,
    ) -> Result<(), AppError>;
    async fn delete_index(&self, index_name: &str) -> Result<(), AppError>;
}
