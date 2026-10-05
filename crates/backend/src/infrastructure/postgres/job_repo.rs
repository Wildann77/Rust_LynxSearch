use crate::domain::models::{FolderId, IndexingJob, JobId, JobProgressUpdate, JobStatus, JobType};
use crate::domain::ports::JobRepository;
use crate::error::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct PgJobRepository {
    pool: PgPool,
}

impl PgJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobRepository for PgJobRepository {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError> {
        let folder_id_uuid = job.folder_id.map(|f| f.into_inner());

        sqlx::query(
            "INSERT INTO indexing_jobs (\
                 id, folder_id, job_type, status, files_total, files_added, files_updated, \
                 files_deleted, files_skipped, files_failed, started_at, completed_at, error_summary\
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(job.id.into_inner())
        .bind(folder_id_uuid)
        .bind(job.job_type.as_str())
        .bind(job.status.as_str())
        .bind(job.files_total)
        .bind(job.files_indexed)
        .bind(0i32) // files_updated
        .bind(0i32) // files_deleted
        .bind(job.files_skipped)
        .bind(job.files_failed)
        .bind(job.started_at)
        .bind(job.completed_at)
        .bind(&job.error_summary)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn update_progress(
        &self,
        id: &JobId,
        update: &JobProgressUpdate,
    ) -> Result<(), AppError> {
        let status_str = update.status.map(|s| s.as_str());

        sqlx::query(
            "UPDATE indexing_jobs \
             SET \
                 status = COALESCE($1, status), \
                 files_total = COALESCE($2, files_total), \
                 files_added = files_added + $3, \
                 files_skipped = files_skipped + $4, \
                 files_failed = files_failed + $5, \
                 error_summary = COALESCE($6, error_summary), \
                 completed_at = CASE \
                     WHEN COALESCE($1, status) IN ('COMPLETED', 'FAILED', 'CANCELLED') THEN COALESCE(completed_at, NOW()) \
                     ELSE completed_at \
                 END \
             WHERE id = $7",
        )
        .bind(status_str)
        .bind(update.files_total)
        .bind(update.indexed_delta)
        .bind(update.skipped_delta)
        .bind(update.failed_delta)
        .bind(update.error_summary.as_deref())
        .bind(id.into_inner())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<IndexingJob>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, folder_id, job_type, status, files_total, files_added, files_updated, \
                    files_skipped, files_failed, started_at, completed_at, error_summary \
             FROM indexing_jobs WHERE id = $1",
        )
        .bind(id.into_inner())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let folder_id_uuid: Option<uuid::Uuid> =
            row.try_get("folder_id").map_err(AppError::Database)?;
        let job_type_str: String = row.try_get("job_type").map_err(AppError::Database)?;
        let status_str: String = row.try_get("status").map_err(AppError::Database)?;

        let job_type = JobType::from_str(&job_type_str)
            .map_err(|e| AppError::Internal(format!("Invalid job_type in DB: {e}")))?;
        let status = JobStatus::from_str(&status_str)
            .map_err(|e| AppError::Internal(format!("Invalid job status in DB: {e}")))?;

        let files_total: i32 = row.try_get("files_total").map_err(AppError::Database)?;
        let files_added: i32 = row.try_get("files_added").map_err(AppError::Database)?;
        let files_updated: i32 = row.try_get("files_updated").map_err(AppError::Database)?;
        let files_skipped: i32 = row.try_get("files_skipped").map_err(AppError::Database)?;
        let files_failed: i32 = row.try_get("files_failed").map_err(AppError::Database)?;

        let files_indexed = files_added + files_updated;
        let files_processed = files_indexed + files_skipped + files_failed;

        Ok(Some(IndexingJob {
            id: *id,
            folder_id: folder_id_uuid.map(FolderId::from_uuid),
            job_type,
            status,
            files_total,
            files_processed,
            files_indexed,
            files_skipped,
            files_failed,
            error_summary: row.try_get("error_summary").map_err(AppError::Database)?,
            started_at: row.try_get("started_at").map_err(AppError::Database)?,
            completed_at: row.try_get("completed_at").map_err(AppError::Database)?,
        }))
    }

    async fn mark_cancelled(&self, id: &JobId) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE indexing_jobs \
             SET status = 'CANCELLED', completed_at = NOW() \
             WHERE id = $1 AND status IN ('PENDING', 'RUNNING')",
        )
        .bind(id.into_inner())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn find_dangling_jobs(&self) -> Result<Vec<IndexingJob>, AppError> {
        let rows = sqlx::query(
            "SELECT id, folder_id, job_type, status, files_total, files_added, files_updated, \
                    files_skipped, files_failed, started_at, completed_at, error_summary \
             FROM indexing_jobs WHERE status IN ('PENDING', 'RUNNING') ORDER BY started_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut jobs = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: uuid::Uuid = row.try_get("id").map_err(AppError::Database)?;
            let folder_id_uuid: Option<uuid::Uuid> =
                row.try_get("folder_id").map_err(AppError::Database)?;
            let job_type_str: String = row.try_get("job_type").map_err(AppError::Database)?;
            let status_str: String = row.try_get("status").map_err(AppError::Database)?;

            let job_type = JobType::from_str(&job_type_str)
                .map_err(|e| AppError::Internal(format!("Invalid job_type in DB: {e}")))?;
            let status = JobStatus::from_str(&status_str)
                .map_err(|e| AppError::Internal(format!("Invalid job status in DB: {e}")))?;

            let files_total: i32 = row.try_get("files_total").map_err(AppError::Database)?;
            let files_added: i32 = row.try_get("files_added").map_err(AppError::Database)?;
            let files_updated: i32 = row.try_get("files_updated").map_err(AppError::Database)?;
            let files_skipped: i32 = row.try_get("files_skipped").map_err(AppError::Database)?;
            let files_failed: i32 = row.try_get("files_failed").map_err(AppError::Database)?;

            let files_indexed = files_added + files_updated;
            let files_processed = files_indexed + files_skipped + files_failed;

            jobs.push(IndexingJob {
                id: JobId::from_uuid(id_uuid),
                folder_id: folder_id_uuid.map(FolderId::from_uuid),
                job_type,
                status,
                files_total,
                files_processed,
                files_indexed,
                files_skipped,
                files_failed,
                error_summary: row.try_get("error_summary").map_err(AppError::Database)?,
                started_at: row.try_get("started_at").map_err(AppError::Database)?,
                completed_at: row.try_get("completed_at").map_err(AppError::Database)?,
            });
        }

        Ok(jobs)
    }

    async fn recover_dangling_jobs(&self, error_summary: &str) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE indexing_jobs \
             SET status = 'FAILED', completed_at = NOW(), error_summary = $1 \
             WHERE status IN ('PENDING', 'RUNNING')",
        )
        .bind(error_summary)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected())
    }

    async fn cancel_unfinished_jobs(&self, reason: &str) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE indexing_jobs \
             SET status = 'CANCELLED', completed_at = NOW(), error_summary = $1 \
             WHERE status IN ('PENDING', 'RUNNING')",
        )
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected())
    }
}
