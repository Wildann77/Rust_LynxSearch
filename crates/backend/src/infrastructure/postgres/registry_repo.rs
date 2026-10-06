use crate::domain::models::{DocumentId, DocumentStatus, FolderId, RegistryEntry};
use crate::domain::ports::DocumentRegistryRepository;
use crate::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct PgDocumentRegistryRepository {
    pool: PgPool,
}

impl PgDocumentRegistryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DocumentRegistryRepository for PgDocumentRegistryRepository {
    async fn get_entry(&self, doc_id: &DocumentId) -> Result<Option<RegistryEntry>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at \
             FROM document_registry WHERE id = $1",
        )
        .bind(doc_id.into_inner())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let status_str: String = row.try_get("status").map_err(AppError::Database)?;
        let status = DocumentStatus::from_str(&status_str)
            .map_err(|e| AppError::Internal(format!("Invalid document status in DB: {e}")))?;
        let folder_id_uuid: uuid::Uuid = row.try_get("folder_id").map_err(AppError::Database)?;
        let content_hash: Option<String> =
            row.try_get("content_hash").map_err(AppError::Database)?;
        let status_reason: Option<String> =
            row.try_get("status_reason").map_err(AppError::Database)?;

        Ok(Some(RegistryEntry {
            id: *doc_id,
            folder_id: FolderId::from_uuid(folder_id_uuid),
            relative_path: row.try_get("relative_path").map_err(AppError::Database)?,
            content_hash: content_hash.unwrap_or_default(),
            file_size: row.try_get("file_size_bytes").map_err(AppError::Database)?,
            status,
            status_reason,
            indexed_at: row.try_get("last_indexed_at").map_err(AppError::Database)?,
            updated_at: row.try_get("modified_at").map_err(AppError::Database)?,
        }))
    }

    async fn list_by_folder(&self, folder_id: &FolderId) -> Result<Vec<RegistryEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at \
             FROM document_registry WHERE folder_id = $1 ORDER BY relative_path ASC",
        )
        .bind(folder_id.into_inner())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
            let status_str: String = row.try_get("status").map_err(AppError::Database)?;
            let status = DocumentStatus::from_str(&status_str)
                .map_err(|e| AppError::Internal(format!("Invalid document status in DB: {e}")))?;
            let content_hash: Option<String> =
                row.try_get("content_hash").map_err(AppError::Database)?;
            let status_reason: Option<String> =
                row.try_get("status_reason").map_err(AppError::Database)?;

            entries.push(RegistryEntry {
                id: DocumentId::from_uuid(id_uuid),
                folder_id: *folder_id,
                relative_path: row.try_get("relative_path").map_err(AppError::Database)?,
                content_hash: content_hash.unwrap_or_default(),
                file_size: row.try_get("file_size_bytes").map_err(AppError::Database)?,
                status,
                status_reason,
                indexed_at: row.try_get("last_indexed_at").map_err(AppError::Database)?,
                updated_at: row.try_get("modified_at").map_err(AppError::Database)?,
            });
        }

        Ok(entries)
    }

    async fn list_by_status(&self, status: DocumentStatus) -> Result<Vec<RegistryEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at \
             FROM document_registry WHERE status = $1 ORDER BY relative_path ASC",
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
            let status_str: String = row.try_get("status").map_err(AppError::Database)?;
            let status = DocumentStatus::from_str(&status_str)
                .map_err(|e| AppError::Internal(format!("Invalid document status in DB: {e}")))?;
            let folder_id_uuid: uuid::Uuid =
                row.try_get("folder_id").map_err(AppError::Database)?;
            let content_hash: Option<String> =
                row.try_get("content_hash").map_err(AppError::Database)?;
            let status_reason: Option<String> =
                row.try_get("status_reason").map_err(AppError::Database)?;

            entries.push(RegistryEntry {
                id: DocumentId::from_uuid(id_uuid),
                folder_id: FolderId::from_uuid(folder_id_uuid),
                relative_path: row.try_get("relative_path").map_err(AppError::Database)?,
                content_hash: content_hash.unwrap_or_default(),
                file_size: row.try_get("file_size_bytes").map_err(AppError::Database)?,
                status,
                status_reason,
                indexed_at: row.try_get("last_indexed_at").map_err(AppError::Database)?,
                updated_at: row.try_get("modified_at").map_err(AppError::Database)?,
            });
        }

        Ok(entries)
    }

    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO document_registry (\
                 id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at\
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (id) DO UPDATE SET \
                 relative_path = EXCLUDED.relative_path, \
                 content_hash = EXCLUDED.content_hash, \
                 file_size_bytes = EXCLUDED.file_size_bytes, \
                 modified_at = EXCLUDED.modified_at, \
                 status = EXCLUDED.status, \
                 status_reason = EXCLUDED.status_reason, \
                 last_indexed_at = EXCLUDED.last_indexed_at",
        )
        .bind(entry.id.into_inner())
        .bind(entry.folder_id.into_inner())
        .bind(&entry.relative_path)
        .bind(&entry.content_hash)
        .bind(entry.file_size)
        .bind(entry.updated_at)
        .bind(entry.status.as_str())
        .bind(&entry.status_reason)
        .bind(entry.indexed_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn upsert_batch(&self, entries: &[RegistryEntry]) -> Result<u64, AppError> {
        if entries.is_empty() {
            return Ok(0);
        }

        let ids: Vec<uuid::Uuid> = entries.iter().map(|e| e.id.into_inner()).collect();
        let folder_ids: Vec<uuid::Uuid> =
            entries.iter().map(|e| e.folder_id.into_inner()).collect();
        let relative_paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        let content_hashes: Vec<&str> = entries.iter().map(|e| e.content_hash.as_str()).collect();
        let file_sizes: Vec<i64> = entries.iter().map(|e| e.file_size).collect();
        let modified_ats: Vec<DateTime<Utc>> = entries.iter().map(|e| e.updated_at).collect();
        let statuses: Vec<&str> = entries.iter().map(|e| e.status.as_str()).collect();
        let status_reasons: Vec<Option<&str>> =
            entries.iter().map(|e| e.status_reason.as_deref()).collect();
        let indexed_ats: Vec<DateTime<Utc>> = entries.iter().map(|e| e.indexed_at).collect();

        let result = sqlx::query(
            "INSERT INTO document_registry (\
                 id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at\
             ) \
             SELECT u.id, u.folder_id, u.relative_path, u.content_hash, u.file_size_bytes, u.modified_at, u.status, u.status_reason, u.last_indexed_at \
             FROM UNNEST(\
                 $1::uuid[], $2::uuid[], $3::text[], $4::varchar[], $5::bigint[], $6::timestamptz[], $7::varchar[], $8::text[], $9::timestamptz[]\
             ) AS u(id, folder_id, relative_path, content_hash, file_size_bytes, modified_at, status, status_reason, last_indexed_at) \
             ON CONFLICT (id) DO UPDATE SET \
                 relative_path = EXCLUDED.relative_path, \
                 content_hash = EXCLUDED.content_hash, \
                 file_size_bytes = EXCLUDED.file_size_bytes, \
                 modified_at = EXCLUDED.modified_at, \
                 status = EXCLUDED.status, \
                 status_reason = EXCLUDED.status_reason, \
                 last_indexed_at = EXCLUDED.last_indexed_at",
        )
        .bind(&ids[..])
        .bind(&folder_ids[..])
        .bind(&relative_paths[..])
        .bind(&content_hashes[..])
        .bind(&file_sizes[..])
        .bind(&modified_ats[..])
        .bind(&statuses[..])
        .bind(&status_reasons[..])
        .bind(&indexed_ats[..])
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected())
    }

    async fn delete_entries(&self, ids: &[DocumentId]) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }

        let id_uuids: Vec<uuid::Uuid> = ids.iter().map(|id| id.into_inner()).collect();
        let result = sqlx::query("DELETE FROM document_registry WHERE id = ANY($1)")
            .bind(&id_uuids[..])
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(result.rows_affected())
    }
}
