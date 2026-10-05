use crate::domain::models::{FolderId, JobId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerCommand {
    IndexFolder {
        job_id: JobId,
        folder_id: FolderId,
        rescan: bool,
    },
    CancelJob {
        job_id: JobId,
    },
    RebuildIndex {
        job_id: JobId,
        target_index: String,
    },
}
