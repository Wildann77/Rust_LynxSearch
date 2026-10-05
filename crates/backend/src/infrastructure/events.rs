use crate::domain::events::{DomainEvent, DomainEventHandler};

/// Infrastructure adapter that logs domain events using the structured `tracing` ecosystem.
/// Keeps domain 100% pure without infra dependencies or I/O.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingDomainEventHandler;

impl TracingDomainEventHandler {
    pub fn new() -> Self {
        Self
    }
}

impl DomainEventHandler for TracingDomainEventHandler {
    fn handle(&self, event: &DomainEvent) {
        match event {
            DomainEvent::IndexingJobStarted(e) => {
                tracing::info!(
                    event = "IndexingJobStarted",
                    job_id = %e.job_id,
                    folder_id = ?e.folder_id.as_ref().map(|f| f.to_string()),
                    "Indexing job started"
                );
            }
            DomainEvent::DocumentIndexed(e) => {
                tracing::debug!(
                    event = "DocumentIndexed",
                    job_id = %e.job_id,
                    path = %e.path,
                    "Document indexed"
                );
            }
            DomainEvent::DocumentSkipped(e) => {
                tracing::info!(
                    event = "DocumentSkipped",
                    job_id = %e.job_id,
                    path = %e.path,
                    reason = %e.reason,
                    "Document skipped"
                );
            }
            DomainEvent::DocumentFailed(e) => {
                tracing::error!(
                    event = "DocumentFailed",
                    job_id = %e.job_id,
                    path = %e.path,
                    error = %e.error,
                    "Document indexing failed"
                );
            }
            DomainEvent::IndexingJobCompleted(e) => {
                tracing::info!(
                    event = "IndexingJobCompleted",
                    job_id = %e.job_id,
                    files_total = e.summary.files_total,
                    files_indexed = e.summary.files_indexed,
                    files_skipped = e.summary.files_skipped,
                    files_failed = e.summary.files_failed,
                    duration_ms = ?e.summary.duration_ms,
                    "Indexing job completed"
                );
            }
            DomainEvent::IndexRebuilt(e) => {
                tracing::info!(
                    event = "IndexRebuilt",
                    old_index = %e.old_index,
                    new_index = %e.new_index,
                    "Elasticsearch index rebuilt"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{DomainEventDispatcher, JobSummary};
    use crate::domain::models::{FolderId, JobId};
    use std::sync::Arc;

    #[test]
    fn test_tracing_handler_does_not_panic_on_any_event() {
        let handler = Arc::new(TracingDomainEventHandler::new());
        let dispatcher = DomainEventDispatcher::new();
        dispatcher.register(handler);

        let job_id = JobId::new();
        let folder_id = FolderId::new();

        dispatcher.dispatch(&DomainEvent::job_started(job_id, Some(folder_id)));
        dispatcher.dispatch(&DomainEvent::job_started(job_id, None));
        dispatcher.dispatch(&DomainEvent::document_indexed(job_id, "main.rs"));
        dispatcher.dispatch(&DomainEvent::document_skipped(
            job_id,
            "target/debug",
            "IgnoredDirectory",
        ));
        dispatcher.dispatch(&DomainEvent::document_failed(
            job_id,
            "broken.bin",
            "CorruptData",
        ));
        dispatcher.dispatch(&DomainEvent::job_completed(
            job_id,
            JobSummary::new(10, 8, 1, 1, Some(450)),
        ));
        dispatcher.dispatch(&DomainEvent::index_rebuilt("lynx_v1", "lynx_v2"));
    }
}
