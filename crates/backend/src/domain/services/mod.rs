pub mod document_extractor;
pub mod scan_planner;
pub mod search_query_builder;

pub use crate::domain::query_parser::QueryParser;
pub use document_extractor::{
    DEFAULT_MAX_FILE_SIZE_BYTES, DocumentExtractor, ExtractOptions, ExtractedDoc, ExtractionResult,
    SkipReason, extract_project, is_secret_file,
};
pub use scan_planner::{
    PlanAdd, PlanDelete, PlanMove, PlanSkip, PlanSkipReason, PlanUpdate, ScanPlan, ScanPlanner,
    normalize_path_str,
};
pub use search_query_builder::{
    DEFAULT_PAGE, DEFAULT_PER_PAGE, DEFAULT_SOURCE_FIELDS, HighlightConfig, MAX_PER_PAGE,
    SearchQueryBuilder,
};
