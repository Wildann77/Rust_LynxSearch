pub mod document;
pub mod folder;
pub mod id;
pub mod job;
pub mod registry;
pub mod search;
pub mod settings;
pub mod types;

pub use crate::domain::events::JobSummary;
pub use document::{BulkIndexReport, IndexedDocument, SearchRawResponse};
pub use folder::{Folder, FolderStatus};
pub use id::{DocumentId, FolderId, JobId};
pub use job::{IndexingJob, JobProgressUpdate, JobStatus, JobType};
pub use registry::{DocumentStatus, RegistryEntry};
pub use search::{
    FacetBucket, SearchExecutionResult, SearchFacets, SearchHighlight, SearchHit, SearchQuery,
    extract_line_number, parse_search_execution_result,
};
pub use settings::AppSettings;
pub use types::{DocumentType, FilterKey, Language};
