use crate::error::AppError;
use crate::state::Repositories;
use serde::{Deserialize, Serialize};

pub const STARTUP_CRASH_RECOVERY_ERROR_MSG: &str =
    "Server di-restart mendadak saat proses job berjalan";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub jobs_recovered: u64,
    pub folders_reset: u64,
}

pub async fn recover_on_startup(repositories: &Repositories) -> Result<RecoveryReport, AppError> {
    // 1. Query dangling jobs (status RUNNING or PENDING)
    let dangling_jobs = repositories.job.find_dangling_jobs().await?;
    if !dangling_jobs.is_empty() {
        tracing::warn!(
            count = dangling_jobs.len(),
            job_ids = ?dangling_jobs.iter().map(|j| j.id).collect::<Vec<_>>(),
            "Found dangling indexing jobs from previous run or ungraceful shutdown"
        );
    }

    // 2. Mark dangling jobs FAILED, set completed_at & error_summary
    let jobs_recovered = repositories
        .job
        .recover_dangling_jobs(STARTUP_CRASH_RECOVERY_ERROR_MSG)
        .await?;

    // 3. Reset folder status SCANNING -> IDLE
    let folders_reset = repositories.folder.reset_scanning_folders().await?;
    if folders_reset > 0 {
        tracing::warn!(
            count = folders_reset,
            "Reset dangling folder scanning statuses to IDLE"
        );
    }

    Ok(RecoveryReport {
        jobs_recovered,
        folders_reset,
    })
}
