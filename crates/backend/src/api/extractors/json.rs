use std::ops::{Deref, DerefMut};

use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::validation_helper::validation_errors_to_json;
use crate::error::AppError;

/// Custom Axum extractor untuk JSON request body dengan validasi deklaratif via `validator`.
/// Menghasilkan HTTP 422 `VALIDATION_FAILED` jika parsing deserialisasi atau validasi gagal.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|err| {
                let mut details = serde_json::Map::new();
                details.insert(
                    "_".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::String(err.to_string())]),
                );
                AppError::ValidationErrors {
                    message: "Payload JSON tidak valid.".to_string(),
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
