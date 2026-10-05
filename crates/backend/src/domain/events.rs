use crate::domain::models::{FolderId, JobId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

/// Summary of an indexing job's final counters and duration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct JobSummary {
    pub files_total: i32,
    pub files_indexed: i32,
    pub files_skipped: i32,
    pub files_failed: i32,
    pub duration_ms: Option<u64>,
}

impl JobSummary {
    pub fn new(
        files_total: i32,
        files_indexed: i32,
        files_skipped: i32,
        files_failed: i32,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            files_total,
            files_indexed,
            files_skipped,
            files_failed,
            duration_ms,
        }
    }
}

/// Emitted when an indexing job is started by the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexingJobStarted {
    pub job_id: JobId,
    pub folder_id: Option<FolderId>,
}

/// Emitted when a document has been successfully indexed in Elasticsearch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentIndexed {
    pub job_id: JobId,
    pub path: String,
}

/// Emitted when a document is skipped during scanning or extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSkipped {
    pub job_id: JobId,
    pub path: String,
    pub reason: String,
}

/// Emitted when a document fails during extraction or indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentFailed {
    pub job_id: JobId,
    pub path: String,
    pub error: String,
}

/// Emitted when an indexing job reaches terminal completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexingJobCompleted {
    pub job_id: JobId,
    pub summary: JobSummary,
}

/// Emitted when Elasticsearch index has been rebuilt and alias swapped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexRebuilt {
    pub old_index: String,
    pub new_index: String,
}

/// Strongly-typed domain event envelope. Pure data, no I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DomainEvent {
    IndexingJobStarted(IndexingJobStarted),
    DocumentIndexed(DocumentIndexed),
    DocumentSkipped(DocumentSkipped),
    DocumentFailed(DocumentFailed),
    IndexingJobCompleted(IndexingJobCompleted),
    IndexRebuilt(IndexRebuilt),
}

impl DomainEvent {
    pub fn job_started(job_id: JobId, folder_id: Option<FolderId>) -> Self {
        Self::IndexingJobStarted(IndexingJobStarted { job_id, folder_id })
    }

    pub fn document_indexed(job_id: JobId, path: impl Into<String>) -> Self {
        Self::DocumentIndexed(DocumentIndexed {
            job_id,
            path: path.into(),
        })
    }

    pub fn document_skipped(
        job_id: JobId,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::DocumentSkipped(DocumentSkipped {
            job_id,
            path: path.into(),
            reason: reason.into(),
        })
    }

    pub fn document_failed(
        job_id: JobId,
        path: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self::DocumentFailed(DocumentFailed {
            job_id,
            path: path.into(),
            error: error.into(),
        })
    }

    pub fn job_completed(job_id: JobId, summary: JobSummary) -> Self {
        Self::IndexingJobCompleted(IndexingJobCompleted { job_id, summary })
    }

    pub fn index_rebuilt(old_index: impl Into<String>, new_index: impl Into<String>) -> Self {
        Self::IndexRebuilt(IndexRebuilt {
            old_index: old_index.into(),
            new_index: new_index.into(),
        })
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::IndexingJobStarted(_) => "INDEXING_JOB_STARTED",
            Self::DocumentIndexed(_) => "DOCUMENT_INDEXED",
            Self::DocumentSkipped(_) => "DOCUMENT_SKIPPED",
            Self::DocumentFailed(_) => "DOCUMENT_FAILED",
            Self::IndexingJobCompleted(_) => "INDEXING_JOB_COMPLETED",
            Self::IndexRebuilt(_) => "INDEX_REBUILT",
        }
    }

    pub fn job_id(&self) -> Option<&JobId> {
        match self {
            Self::IndexingJobStarted(e) => Some(&e.job_id),
            Self::DocumentIndexed(e) => Some(&e.job_id),
            Self::DocumentSkipped(e) => Some(&e.job_id),
            Self::DocumentFailed(e) => Some(&e.job_id),
            Self::IndexingJobCompleted(e) => Some(&e.job_id),
            Self::IndexRebuilt(_) => None,
        }
    }
}

impl fmt::Display for DomainEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexingJobStarted(e) => match &e.folder_id {
                Some(folder_id) => write!(
                    f,
                    "IndexingJobStarted(job: {}, folder: {})",
                    e.job_id, folder_id
                ),
                None => write!(f, "IndexingJobStarted(job: {}, global/rebuild)", e.job_id),
            },
            Self::DocumentIndexed(e) => {
                write!(f, "DocumentIndexed(job: {}, path: {})", e.job_id, e.path)
            }
            Self::DocumentSkipped(e) => {
                write!(
                    f,
                    "DocumentSkipped(job: {}, path: {}, reason: {})",
                    e.job_id, e.path, e.reason
                )
            }
            Self::DocumentFailed(e) => {
                write!(
                    f,
                    "DocumentFailed(job: {}, path: {}, error: {})",
                    e.job_id, e.path, e.error
                )
            }
            Self::IndexingJobCompleted(e) => {
                write!(
                    f,
                    "IndexingJobCompleted(job: {}, indexed: {}, skipped: {}, failed: {}, total: {})",
                    e.job_id,
                    e.summary.files_indexed,
                    e.summary.files_skipped,
                    e.summary.files_failed,
                    e.summary.files_total
                )
            }
            Self::IndexRebuilt(e) => {
                write!(
                    f,
                    "IndexRebuilt(old: {}, new: {})",
                    e.old_index, e.new_index
                )
            }
        }
    }
}

impl From<IndexingJobStarted> for DomainEvent {
    fn from(e: IndexingJobStarted) -> Self {
        Self::IndexingJobStarted(e)
    }
}

impl From<DocumentIndexed> for DomainEvent {
    fn from(e: DocumentIndexed) -> Self {
        Self::DocumentIndexed(e)
    }
}

impl From<DocumentSkipped> for DomainEvent {
    fn from(e: DocumentSkipped) -> Self {
        Self::DocumentSkipped(e)
    }
}

impl From<DocumentFailed> for DomainEvent {
    fn from(e: DocumentFailed) -> Self {
        Self::DocumentFailed(e)
    }
}

impl From<IndexingJobCompleted> for DomainEvent {
    fn from(e: IndexingJobCompleted) -> Self {
        Self::IndexingJobCompleted(e)
    }
}

impl From<IndexRebuilt> for DomainEvent {
    fn from(e: IndexRebuilt) -> Self {
        Self::IndexRebuilt(e)
    }
}

/// Pure in-memory domain event handler trait.
/// Strictly no I/O.
pub trait DomainEventHandler: Send + Sync {
    fn handle(&self, event: &DomainEvent);
}

/// In-memory event recorder for testing and verification without I/O.
#[derive(Debug, Clone, Default)]
pub struct RecordingDomainEventHandler {
    events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl RecordingDomainEventHandler {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn recorded_events(&self) -> Vec<DomainEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    pub fn count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl DomainEventHandler for RecordingDomainEventHandler {
    fn handle(&self, event: &DomainEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

/// In-memory event dispatcher for domain events.
/// Dispatches synchronously to registered handlers without I/O.
#[derive(Default)]
pub struct DomainEventDispatcher {
    handlers: std::sync::RwLock<Vec<Arc<dyn DomainEventHandler>>>,
}

impl DomainEventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: std::sync::RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, handler: Arc<dyn DomainEventHandler>) {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.push(handler);
        }
    }

    pub fn dispatch(&self, event: &DomainEvent) {
        if let Ok(handlers) = self.handlers.read() {
            for handler in handlers.iter() {
                handler.handle(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_constructors_and_helpers() {
        let job_id = JobId::new();
        let folder_id = FolderId::new();

        let started = DomainEvent::job_started(job_id, Some(folder_id));
        assert_eq!(started.event_type(), "INDEXING_JOB_STARTED");
        assert_eq!(started.job_id(), Some(&job_id));
        assert!(format!("{started}").contains("IndexingJobStarted"));

        let started_global = DomainEvent::job_started(job_id, None);
        assert!(format!("{started_global}").contains("global/rebuild"));

        let indexed = DomainEvent::document_indexed(job_id, "src/main.rs");
        assert_eq!(indexed.event_type(), "DOCUMENT_INDEXED");
        assert_eq!(indexed.job_id(), Some(&job_id));
        assert!(format!("{indexed}").contains("src/main.rs"));

        let skipped = DomainEvent::document_skipped(job_id, "test.bin", "BinaryFileDetected");
        assert_eq!(skipped.event_type(), "DOCUMENT_SKIPPED");
        assert_eq!(skipped.job_id(), Some(&job_id));
        assert!(format!("{skipped}").contains("BinaryFileDetected"));

        let failed = DomainEvent::document_failed(job_id, "broken.rs", "Syntax error");
        assert_eq!(failed.event_type(), "DOCUMENT_FAILED");
        assert_eq!(failed.job_id(), Some(&job_id));
        assert!(format!("{failed}").contains("Syntax error"));

        let summary = JobSummary::new(100, 80, 15, 5, Some(1250));
        let completed = DomainEvent::job_completed(job_id, summary.clone());
        assert_eq!(completed.event_type(), "INDEXING_JOB_COMPLETED");
        assert_eq!(completed.job_id(), Some(&job_id));
        assert!(format!("{completed}").contains("indexed: 80"));

        let rebuilt = DomainEvent::index_rebuilt("lynx_v1", "lynx_v2");
        assert_eq!(rebuilt.event_type(), "INDEX_REBUILT");
        assert_eq!(rebuilt.job_id(), None);
        assert!(format!("{rebuilt}").contains("lynx_v1"));
    }

    #[test]
    fn test_domain_event_serde_roundtrip() {
        let job_id = JobId::new();
        let folder_id = FolderId::new();

        let events = vec![
            DomainEvent::job_started(job_id, Some(folder_id)),
            DomainEvent::job_started(job_id, None),
            DomainEvent::document_indexed(job_id, "docs/architecture.md"),
            DomainEvent::document_skipped(job_id, ".env", "SecretPatternBlacklist"),
            DomainEvent::document_failed(job_id, "corrupt.txt", "Utf8DecodingFailed"),
            DomainEvent::job_completed(
                job_id,
                JobSummary {
                    files_total: 10,
                    files_indexed: 8,
                    files_skipped: 1,
                    files_failed: 1,
                    duration_ms: Some(540),
                },
            ),
            DomainEvent::index_rebuilt("old_idx", "new_idx"),
        ];

        for event in events {
            let json = serde_json::to_string(&event).expect("Serialize event to JSON");
            assert!(
                json.contains("\"type\":\""),
                "JSON must contain type discriminator: {json}"
            );
            assert!(
                json.contains("\"payload\":"),
                "JSON must contain payload object: {json}"
            );
            let deserialized: DomainEvent =
                serde_json::from_str(&json).expect("Deserialize event from JSON");
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn test_domain_event_dispatcher_and_recording_handler() {
        let recorder = Arc::new(RecordingDomainEventHandler::new());
        let dispatcher = DomainEventDispatcher::new();
        dispatcher.register(recorder.clone());

        assert_eq!(recorder.count(), 0);

        let job_id = JobId::new();
        let ev1 = DomainEvent::job_started(job_id, None);
        let ev2 = DomainEvent::document_indexed(job_id, "lib.rs");

        dispatcher.dispatch(&ev1);
        dispatcher.dispatch(&ev2);

        assert_eq!(recorder.count(), 2);
        let recorded = recorder.recorded_events();
        assert_eq!(recorded[0], ev1);
        assert_eq!(recorded[1], ev2);

        recorder.clear();
        assert_eq!(recorder.count(), 0);
    }

    #[test]
    fn test_from_conversions() {
        let job_id = JobId::new();
        let started = IndexingJobStarted {
            job_id,
            folder_id: None,
        };
        let ev: DomainEvent = started.clone().into();
        assert_eq!(ev, DomainEvent::IndexingJobStarted(started));

        let indexed = DocumentIndexed {
            job_id,
            path: "test.md".into(),
        };
        let ev: DomainEvent = indexed.clone().into();
        assert_eq!(ev, DomainEvent::DocumentIndexed(indexed));

        let skipped = DocumentSkipped {
            job_id,
            path: "skip.me".into(),
            reason: "Ignored".into(),
        };
        let ev: DomainEvent = skipped.clone().into();
        assert_eq!(ev, DomainEvent::DocumentSkipped(skipped));

        let failed = DocumentFailed {
            job_id,
            path: "fail.me".into(),
            error: "Error".into(),
        };
        let ev: DomainEvent = failed.clone().into();
        assert_eq!(ev, DomainEvent::DocumentFailed(failed));

        let completed = IndexingJobCompleted {
            job_id,
            summary: JobSummary::default(),
        };
        let ev: DomainEvent = completed.clone().into();
        assert_eq!(ev, DomainEvent::IndexingJobCompleted(completed));

        let rebuilt = IndexRebuilt {
            old_index: "a".into(),
            new_index: "b".into(),
        };
        let ev: DomainEvent = rebuilt.clone().into();
        assert_eq!(ev, DomainEvent::IndexRebuilt(rebuilt));
    }
}
