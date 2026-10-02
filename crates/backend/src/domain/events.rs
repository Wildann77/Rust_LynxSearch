use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DomainEvent {
    IndexingJobStarted {
        job_id: Uuid,
        folder_id: Uuid,
    },
    DocumentIndexed {
        job_id: Uuid,
        path: String,
    },
    DocumentSkipped {
        job_id: Uuid,
        path: String,
        reason: String,
    },
    DocumentFailed {
        job_id: Uuid,
        path: String,
        error: String,
    },
    IndexingJobCompleted {
        job_id: Uuid,
    },
    IndexRebuilt {
        old_index: String,
        new_index: String,
    },
}
