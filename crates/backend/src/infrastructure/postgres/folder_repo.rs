use crate::domain::models::{Folder, FolderId, FolderStatus};
use crate::domain::ports::FolderRepository;
use crate::error::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct PgFolderRepository {
    pool: PgPool,
}

impl PgFolderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FolderRepository for PgFolderRepository {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError> {
        let root_path_str = folder.path.to_string_lossy().to_string();
        let status_str = folder.status.as_str();

        sqlx::query(
            "INSERT INTO folders (id, root_path, created_at, last_scanned_at, status) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(folder.id.into_inner())
        .bind(&root_path_str)
        .bind(folder.created_at)
        .bind(folder.last_scanned_at)
        .bind(status_str)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, root_path, created_at, last_scanned_at, status \
             FROM folders WHERE id = $1",
        )
        .bind(id.into_inner())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let status_str: String = row.try_get("status").map_err(AppError::Database)?;
        let status = FolderStatus::from_str(&status_str)
            .map_err(|e| AppError::Internal(format!("Invalid folder status in DB: {e}")))?;
        let root_path_str: String = row.try_get("root_path").map_err(AppError::Database)?;

        Ok(Some(Folder {
            id: *id,
            path: PathBuf::from(root_path_str),
            status,
            created_at: row.try_get("created_at").map_err(AppError::Database)?,
            last_scanned_at: row.try_get("last_scanned_at").map_err(AppError::Database)?,
        }))
    }

    async fn find_by_path(&self, path: &std::path::Path) -> Result<Option<Folder>, AppError> {
        let root_path_str = path.to_string_lossy().to_string();
        let row_opt = sqlx::query(
            "SELECT id, root_path, created_at, last_scanned_at, status \
             FROM folders WHERE root_path = $1",
        )
        .bind(&root_path_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
        let status_str: String = row.try_get("status").map_err(AppError::Database)?;
        let status = FolderStatus::from_str(&status_str)
            .map_err(|e| AppError::Internal(format!("Invalid folder status in DB: {e}")))?;
        let root_path_str: String = row.try_get("root_path").map_err(AppError::Database)?;

        Ok(Some(Folder {
            id: FolderId::from_uuid(id_uuid),
            path: PathBuf::from(root_path_str),
            status,
            created_at: row.try_get("created_at").map_err(AppError::Database)?,
            last_scanned_at: row.try_get("last_scanned_at").map_err(AppError::Database)?,
        }))
    }

    async fn list_folders(&self) -> Result<Vec<Folder>, AppError> {
        let rows = sqlx::query(
            "SELECT id, root_path, created_at, last_scanned_at, status \
             FROM folders ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut folders = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
            let root_path_str: String = row.try_get("root_path").map_err(AppError::Database)?;
            let status_str: String = row.try_get("status").map_err(AppError::Database)?;
            let status = FolderStatus::from_str(&status_str)
                .map_err(|e| AppError::Internal(format!("Invalid folder status in DB: {e}")))?;

            folders.push(Folder {
                id: FolderId::from_uuid(id_uuid),
                path: PathBuf::from(root_path_str),
                status,
                created_at: row.try_get("created_at").map_err(AppError::Database)?,
                last_scanned_at: row.try_get("last_scanned_at").map_err(AppError::Database)?,
            });
        }

        Ok(folders)
    }

    async fn list_folders_with_counts(&self) -> Result<Vec<(Folder, u64)>, AppError> {
        let rows = sqlx::query(
            "SELECT f.id, f.root_path, f.created_at, f.last_scanned_at, f.status, \
                    COALESCE(COUNT(d.id) FILTER (WHERE d.status = 'INDEXED'), 0)::BIGINT AS document_count \
             FROM folders f \
             LEFT JOIN document_registry d ON d.folder_id = f.id \
             GROUP BY f.id, f.root_path, f.created_at, f.last_scanned_at, f.status \
             ORDER BY f.created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
            let root_path_str: String = row.try_get("root_path").map_err(AppError::Database)?;
            let status_str: String = row.try_get("status").map_err(AppError::Database)?;
            let status = FolderStatus::from_str(&status_str)
                .map_err(|e| AppError::Internal(format!("Invalid folder status in DB: {e}")))?;
            let doc_count: i64 = row.try_get("document_count").map_err(AppError::Database)?;

            let folder = Folder {
                id: FolderId::from_uuid(id_uuid),
                path: PathBuf::from(root_path_str),
                status,
                created_at: row.try_get("created_at").map_err(AppError::Database)?,
                last_scanned_at: row.try_get("last_scanned_at").map_err(AppError::Database)?,
            };

            results.push((folder, doc_count.max(0) as u64));
        }

        Ok(results)
    }

    async fn delete_folder(&self, id: &FolderId) -> Result<(), AppError> {
        sqlx::query("DELETE FROM folders WHERE id = $1")
            .bind(id.into_inner())
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    async fn update_last_scanned(&self, id: &FolderId) -> Result<(), AppError> {
        sqlx::query("UPDATE folders SET last_scanned_at = NOW() WHERE id = $1")
            .bind(id.into_inner())
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    async fn update_status(&self, id: &FolderId, status: FolderStatus) -> Result<(), AppError> {
        sqlx::query("UPDATE folders SET status = $1 WHERE id = $2")
            .bind(status.as_str())
            .bind(id.into_inner())
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    async fn reset_scanning_folders(&self) -> Result<u64, AppError> {
        let result = sqlx::query("UPDATE folders SET status = 'IDLE' WHERE status = 'SCANNING'")
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(result.rows_affected())
    }
}
