use std::borrow::Cow;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

pub fn validate_relative_path(path: &str) -> Result<(), ValidationError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        let mut err = ValidationError::new("empty");
        err.message = Some(Cow::Borrowed("Path relatif tidak boleh kosong"));
        return Err(err);
    }

    if trimmed.contains('\0') {
        let mut err = ValidationError::new("null_byte");
        err.message = Some(Cow::Borrowed(
            "Path relatif tidak boleh mengandung null byte",
        ));
        return Err(err);
    }

    let p = Path::new(trimmed);
    if p.is_absolute() {
        let mut err = ValidationError::new("not_relative");
        err.message = Some(Cow::Borrowed(
            "Path relatif tidak boleh berupa path absolut",
        ));
        return Err(err);
    }

    for component in p.components() {
        if matches!(component, Component::ParentDir) {
            let mut err = ValidationError::new("path_traversal");
            err.message = Some(Cow::Borrowed(
                "Path relatif tidak boleh mengandung '..' (path traversal)",
            ));
            return Err(err);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct IndexDocumentRequestDto {
    pub folder_id: Uuid,

    #[validate(custom(function = "validate_relative_path"))]
    pub relative_path: String,
}

impl IndexDocumentRequestDto {
    pub fn new(folder_id: Uuid, relative_path: impl Into<String>) -> Self {
        Self {
            folder_id,
            relative_path: relative_path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "src/main.rs");
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_empty_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "   ");
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_traversal_relative_path() {
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), "../secret.rs");
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_absolute_relative_path() {
        #[cfg(unix)]
        let path = "/etc/passwd";
        #[cfg(windows)]
        let path = "C:\\secret.rs";
        let dto = IndexDocumentRequestDto::new(Uuid::new_v4(), path);
        assert!(dto.validate().is_err());
    }
}
