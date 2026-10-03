use std::ops::{Deref, DerefMut};

use axum::{extract::FromRequestParts, http::request::Parts};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::validation_helper::validation_errors_to_json;
use crate::error::AppError;

/// Custom Axum extractor untuk query string parameter dengan validasi deklaratif via `validator`.
/// Menghasilkan HTTP 422 `VALIDATION_FAILED` jika parsing deserialisasi atau validasi gagal.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T> ValidatedQuery<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) =
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|err| {
                    let mut details = serde_json::Map::new();
                    details.insert(
                        "_".to_string(),
                        serde_json::Value::Array(vec![serde_json::Value::String(err.to_string())]),
                    );
                    AppError::ValidationErrors {
                        message: "Query parameter tidak valid.".to_string(),
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
