pub mod common;
pub mod folder;
pub mod health;
pub mod index;
pub mod search;
pub mod settings;

pub use common::{PathUuid, ValidatedUuid};
pub use folder::RegisterFolderRequestDto;
pub use health::{
    ComponentHealthDto, HealthSummaryResponseDto, LivenessResponseDto, ReadinessResponseDto,
};
pub use index::IndexDocumentRequestDto;
pub use search::{SearchRequestDto, SuggestRequestDto};
pub use settings::{UpdateBm25WeightsDto, UpdateSettingsRequestDto};

pub use crate::error::{ErrorCode, ErrorResponse};
