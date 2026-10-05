use crate::application::orchestrator::{JobProgressState, JobTracker};
use crate::domain::models::{JobId, JobProgressUpdate, JobStatus};
use crate::domain::ports::JobRepository;
use crate::error::AppError;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub const DEFAULT_PROGRESS_BATCH_SIZE: i32 = 100;

/// Stateful progress reporter for an active indexing job.
/// Maintains real-time in-memory progress in `JobTracker`,
/// and batches periodic progress persistence to `JobRepository` (every `batch_size` files).
/// Ensures terminal states (`Completed`, `Failed`, `Cancelled`) and final summaries
/// are strictly flushed and persisted.
pub struct JobProgressReporter {
    job_id: JobId,
    job_tracker: Arc<JobTracker>,
    job_repo: Arc<dyn JobRepository>,
    batch_size: i32,
    unpersisted_indexed: i32,
    unpersisted_skipped: i32,
    unpersisted_failed: i32,
    cancellation_token: CancellationToken,
}

impl JobProgressReporter {
    pub fn new(
        job_id: JobId,
        job_tracker: Arc<JobTracker>,
        job_repo: Arc<dyn JobRepository>,
        batch_size: Option<i32>,
    ) -> Self {
        let cancellation_token = job_tracker
            .get_cancellation_token(&job_id)
            .unwrap_or_default();

        Self {
            job_id,
            job_tracker,
            job_repo,
            batch_size: batch_size.unwrap_or(DEFAULT_PROGRESS_BATCH_SIZE).max(1),
            unpersisted_indexed: 0,
            unpersisted_skipped: 0,
            unpersisted_failed: 0,
            cancellation_token,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    pub fn current_progress(&self) -> Option<JobProgressState> {
        self.job_tracker.get_progress(&self.job_id)
    }

    pub async fn mark_running(&mut self, files_total: Option<i32>) -> Result<(), AppError> {
        let update = JobProgressUpdate {
            status: Some(JobStatus::Running),
            files_total,
            ..Default::default()
        };
        self.job_tracker
            .update_progress(&self.job_id, update.clone());
        self.job_repo.update_progress(&self.job_id, &update).await?;
        Ok(())
    }

    pub async fn set_files_total(&mut self, total: i32) -> Result<(), AppError> {
        let update = JobProgressUpdate {
            files_total: Some(total),
            ..Default::default()
        };
        self.job_tracker
            .update_progress(&self.job_id, update.clone());
        self.job_repo.update_progress(&self.job_id, &update).await?;
        Ok(())
    }

    pub async fn record_indexed(&mut self, count: i32) -> Result<(), AppError> {
        self.apply_delta(count, 0, 0).await
    }

    pub async fn record_skipped(&mut self, count: i32) -> Result<(), AppError> {
        self.apply_delta(0, count, 0).await
    }

    pub async fn record_failed(&mut self, count: i32) -> Result<(), AppError> {
        self.apply_delta(0, 0, count).await
    }

    pub async fn record_batch(
        &mut self,
        indexed: i32,
        skipped: i32,
        failed: i32,
    ) -> Result<(), AppError> {
        self.apply_delta(indexed, skipped, failed).await
    }

    async fn apply_delta(
        &mut self,
        indexed: i32,
        skipped: i32,
        failed: i32,
    ) -> Result<(), AppError> {
        let processed = indexed + skipped + failed;
        if processed <= 0 {
            return Ok(());
        }

        let update = JobProgressUpdate {
            processed_delta: processed,
            indexed_delta: indexed,
            skipped_delta: skipped,
            failed_delta: failed,
            ..Default::default()
        };

        // Real-time update in memory
        self.job_tracker.update_progress(&self.job_id, update);
        self.unpersisted_indexed += indexed;
        self.unpersisted_skipped += skipped;
        self.unpersisted_failed += failed;

        let total_unpersisted =
            self.unpersisted_indexed + self.unpersisted_skipped + self.unpersisted_failed;
        if total_unpersisted >= self.batch_size {
            self.flush_to_db().await?;
        }

        Ok(())
    }

    pub async fn flush_to_db(&mut self) -> Result<(), AppError> {
        let total_unpersisted =
            self.unpersisted_indexed + self.unpersisted_skipped + self.unpersisted_failed;
        if total_unpersisted == 0 {
            return Ok(());
        }

        let state = match self.job_tracker.get_progress(&self.job_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        let db_update = JobProgressUpdate {
            status: Some(state.status),
            files_total: Some(state.files_total),
            processed_delta: total_unpersisted,
            indexed_delta: self.unpersisted_indexed,
            skipped_delta: self.unpersisted_skipped,
            failed_delta: self.unpersisted_failed,
            error_summary: state.error_summary,
        };

        self.job_repo
            .update_progress(&self.job_id, &db_update)
            .await?;
        self.unpersisted_indexed = 0;
        self.unpersisted_skipped = 0;
        self.unpersisted_failed = 0;

        Ok(())
    }

    pub async fn finish_completed(&mut self) -> Result<(), AppError> {
        if self.is_cancelled() {
            return self.finish_cancelled().await;
        }

        let total_unpersisted =
            self.unpersisted_indexed + self.unpersisted_skipped + self.unpersisted_failed;

        // In-memory JobTracker only needs terminal status transition
        self.job_tracker.update_progress(
            &self.job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Completed),
                ..Default::default()
            },
        );

        // DB receives final status plus remaining unpersisted deltas
        let db_update = JobProgressUpdate {
            status: Some(JobStatus::Completed),
            processed_delta: total_unpersisted,
            indexed_delta: self.unpersisted_indexed,
            skipped_delta: self.unpersisted_skipped,
            failed_delta: self.unpersisted_failed,
            ..Default::default()
        };
        self.job_repo
            .update_progress(&self.job_id, &db_update)
            .await?;

        self.unpersisted_indexed = 0;
        self.unpersisted_skipped = 0;
        self.unpersisted_failed = 0;

        Ok(())
    }

    pub async fn finish_failed(&mut self, error_summary: &str) -> Result<(), AppError> {
        let total_unpersisted =
            self.unpersisted_indexed + self.unpersisted_skipped + self.unpersisted_failed;

        self.job_tracker.update_progress(
            &self.job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Failed),
                error_summary: Some(error_summary.to_string()),
                ..Default::default()
            },
        );

        let db_update = JobProgressUpdate {
            status: Some(JobStatus::Failed),
            processed_delta: total_unpersisted,
            indexed_delta: self.unpersisted_indexed,
            skipped_delta: self.unpersisted_skipped,
            failed_delta: self.unpersisted_failed,
            error_summary: Some(error_summary.to_string()),
            ..Default::default()
        };
        self.job_repo
            .update_progress(&self.job_id, &db_update)
            .await?;

        self.unpersisted_indexed = 0;
        self.unpersisted_skipped = 0;
        self.unpersisted_failed = 0;

        Ok(())
    }

    pub async fn finish_cancelled(&mut self) -> Result<(), AppError> {
        let total_unpersisted =
            self.unpersisted_indexed + self.unpersisted_skipped + self.unpersisted_failed;

        self.job_tracker.update_progress(
            &self.job_id,
            JobProgressUpdate {
                status: Some(JobStatus::Cancelled),
                error_summary: Some("Cancelled by user".to_string()),
                ..Default::default()
            },
        );

        let db_update = JobProgressUpdate {
            status: Some(JobStatus::Cancelled),
            processed_delta: total_unpersisted,
            indexed_delta: self.unpersisted_indexed,
            skipped_delta: self.unpersisted_skipped,
            failed_delta: self.unpersisted_failed,
            error_summary: Some("Cancelled by user".to_string()),
            ..Default::default()
        };
        self.job_repo
            .update_progress(&self.job_id, &db_update)
            .await?;

        self.unpersisted_indexed = 0;
        self.unpersisted_skipped = 0;
        self.unpersisted_failed = 0;

        Ok(())
    }
}
