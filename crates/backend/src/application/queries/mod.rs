pub mod document;
pub mod search;
pub mod suggest_queries;

pub use document::get_document_detail;
pub use search::execute_search;
pub use suggest_queries::execute_suggest;
