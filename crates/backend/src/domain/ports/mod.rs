pub mod stubs;

use crate::domain::models::{
    AppSettings, BulkIndexReport, Folder, FolderStatus, IndexedDocument, IndexingJob,
    JobProgressUpdate, RegistryEntry, SearchRawResponse,
};
use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait FolderRepository: Send + Sync {
    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError>;
    async fn get_folder(&self, id: &Uuid) -> Result<Option<Folder>, AppError>;
    async fn list_folders(&self) -> Result<Vec<Folder>, AppError>;
    async fn delete_folder(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_last_scanned(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_status(&self, id: &Uuid, status: FolderStatus) -> Result<(), AppError>;
    async fn reset_scanning_folders(&self) -> Result<u64, AppError>;
}

#[async_trait]
pub trait DocumentRegistryRepository: Send + Sync {
    async fn get_entry(&self, doc_id: &Uuid) -> Result<Option<RegistryEntry>, AppError>;
    async fn list_by_folder(&self, folder_id: &Uuid) -> Result<Vec<RegistryEntry>, AppError>;
    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError>;
    async fn delete_entries(&self, ids: &[Uuid]) -> Result<u64, AppError>;
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError>;
    async fn update_progress(&self, id: &Uuid, update: &JobProgressUpdate) -> Result<(), AppError>;
    async fn get_job(&self, id: &Uuid) -> Result<Option<IndexingJob>, AppError>;
    async fn mark_cancelled(&self, id: &Uuid) -> Result<(), AppError>;
    async fn find_dangling_jobs(&self) -> Result<Vec<IndexingJob>, AppError>;
    async fn recover_dangling_jobs(&self, error_summary: &str) -> Result<u64, AppError>;
    async fn cancel_unfinished_jobs(&self, reason: &str) -> Result<u64, AppError>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_settings(&self) -> Result<AppSettings, AppError>;
    async fn update_settings(&self, settings: &AppSettings) -> Result<(), AppError>;
}

#[async_trait]
pub trait SearchRepository: Send + Sync {
    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError>;
    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError>;
    async fn delete_document(&self, id: &Uuid) -> Result<(), AppError>;
    async fn delete_documents_by_folder(&self, folder_id: &Uuid) -> Result<u64, AppError>;
    async fn search(
        &self,
        query_dsl: &serde_json::value::RawValue,
    ) -> Result<SearchRawResponse, AppError>;
    async fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>, AppError>;
    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), AppError>;
    async fn ping(&self) -> Result<(), AppError>;
}
