use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDocumentResponseDto {
    pub success: bool,
    pub id: Uuid,
    pub status: String,
    pub message: String,
}
