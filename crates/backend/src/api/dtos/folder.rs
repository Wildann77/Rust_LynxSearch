use std::borrow::Cow;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

/// Custom validator untuk memvalidasi keamanan sintaks path folder root:
/// - Tidak boleh kosong atau hanya whitespace.
/// - Tidak boleh mengandung null-byte `\0`.
/// - Tidak boleh mengandung komponen traversal `..`.
/// - Wajib berupa path absolut (`Path::is_absolute()`).
/// - Panjang path maksimal 4096 karakter.
pub fn validate_folder_root_path(root_path: &str) -> Result<(), ValidationError> {
    let trimmed = root_path.trim();
    if trimmed.is_empty() {
        let mut err = ValidationError::new("empty");
        err.message = Some(Cow::Borrowed("Path folder root tidak boleh kosong"));
        return Err(err);
    }

    if root_path.contains('\0') {
        let mut err = ValidationError::new("null_byte");
        err.message = Some(Cow::Borrowed(
            "Path folder root tidak boleh mengandung null byte",
        ));
        return Err(err);
    }

    if root_path.len() > 4096 {
        let mut err = ValidationError::new("length");
        err.message = Some(Cow::Borrowed(
            "Path folder root terlalu panjang (maksimal 4096 karakter)",
        ));
        return Err(err);
    }

    let path = Path::new(trimmed);

    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            let mut err = ValidationError::new("path_traversal");
            err.message = Some(Cow::Borrowed(
                "Path folder root tidak boleh mengandung '..' (path traversal)",
            ));
            return Err(err);
        }
    }

    if !path.is_absolute() {
        let mut err = ValidationError::new("not_absolute");
        err.message = Some(Cow::Borrowed("Path folder root harus berupa path absolut"));
        return Err(err);
    }

    Ok(())
}

/// DTO request body untuk endpoint `POST /api/index/folder`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct RegisterFolderRequestDto {
    #[validate(custom(function = "validate_folder_root_path"))]
    pub root_path: String,
}

impl RegisterFolderRequestDto {
    pub fn new(root_path: impl Into<String>) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }

    pub fn normalized_path(&self) -> &str {
        self.root_path.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_folder_root_path() {
        #[cfg(unix)]
        let valid_path = "/home/developer/projects/LynxSearch";
        #[cfg(windows)]
        let valid_path = "C:\\projects\\LynxSearch";

        let dto = RegisterFolderRequestDto::new(valid_path);
        assert!(dto.validate().is_ok());
        assert_eq!(dto.normalized_path(), valid_path);
    }

    #[test]
    fn test_empty_or_whitespace_folder_root_path() {
        let empty_dto = RegisterFolderRequestDto::new("");
        assert!(empty_dto.validate().is_err());

        let whitespace_dto = RegisterFolderRequestDto::new("   \t  ");
        assert!(whitespace_dto.validate().is_err());
    }

    #[test]
    fn test_null_byte_rejection() {
        #[cfg(unix)]
        let path = "/home/developer/\0evil";
        #[cfg(windows)]
        let path = "C:\\developer\\\0evil";

        let dto = RegisterFolderRequestDto::new(path);
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_path_traversal_rejection() {
        #[cfg(unix)]
        let path = "/home/developer/../etc/passwd";
        #[cfg(windows)]
        let path = "C:\\projects\\..\\windows\\system32";

        let dto = RegisterFolderRequestDto::new(path);
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_relative_path_rejection() {
        let dto = RegisterFolderRequestDto::new("relative/folder/path");
        assert!(dto.validate().is_err());
    }
}
