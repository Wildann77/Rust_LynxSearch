use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerCommand {
    IndexFolder {
        job_id: Uuid,
        folder_id: Uuid,
        rescan: bool,
    },
    CancelJob {
        job_id: Uuid,
    },
    RebuildIndex {
        job_id: Uuid,
    },
}
