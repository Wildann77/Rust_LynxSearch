use std::ops::{Deref, DerefMut};

use axum::{extract::FromRequestParts, http::request::Parts};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::validation_helper::validation_errors_to_json;
use crate::error::AppError;

/// Custom Axum extractor untuk path parameters dengan validasi deklaratif via `validator`.
/// Menghasilkan HTTP 422 `VALIDATION_FAILED` jika parsing deserialisasi path atau validasi gagal.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedPath<T>(pub T);

impl<T> ValidatedPath<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    T: DeserializeOwned + Validate + Send + Sync,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(value) = axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|err| {
                let mut details = serde_json::Map::new();
                details.insert(
                    "path".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::String(err.to_string())]),
                );
                AppError::ValidationErrors {
                    message: "Parameter path URL tidak valid.".to_string(),
                    details: serde_json::Value::Object(details),
                }
            })?;

        value
            .validate()
            .map_err(|errs| AppError::ValidationErrors {
                message: "Parameter input tidak valid.".to_string(),
                details: validation_errors_to_json(&errs),
            })?;

        Ok(Self(value))
    }
}
