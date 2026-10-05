pub mod common;
pub mod document;
pub mod folder;
pub mod health;
pub mod index;
pub mod search;
pub mod settings;

pub use common::{PathUuid, ValidatedUuid};
pub use document::DeleteDocumentResponseDto;
pub use folder::{
    DeleteFolderResponseDto, FolderResponseDto, IndexFolderResponseDto, RegisterFolderRequestDto,
};
pub use health::{
    ComponentHealthDto, HealthSummaryResponseDto, LivenessResponseDto, ReadinessResponseDto,
};
pub use index::{
    IndexDocumentRequestDto, IndexDocumentResponseDto, JobStatusResponseDto, RebuildIndexResponseDto,
};
pub use search::{SearchRequestDto, SuggestRequestDto};
pub use settings::{UpdateBm25WeightsDto, UpdateSettingsRequestDto};

pub use crate::error::{ErrorCode, ErrorResponse};
