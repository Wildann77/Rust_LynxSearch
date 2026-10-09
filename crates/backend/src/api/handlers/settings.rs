use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::api::dtos::UpdateSettingsRequestDto;
use crate::api::extractors::ValidatedJson;
use crate::config::Bm25Weights;
use crate::domain::models::AppSettings;
use crate::error::AppError;
use crate::state::AppState;

pub async fn get_settings(State(state): State<AppState>) -> impl IntoResponse {
    let settings = state.get_settings().await;
    Json(settings)
}

pub async fn update_settings(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSettingsRequestDto>,
) -> Result<impl IntoResponse, AppError> {
    let current = state.get_settings().await;
    let new_settings = AppSettings {
        max_file_size_bytes: payload
            .max_file_size_bytes
            .unwrap_or(current.max_file_size_bytes),
        weights: match &payload.weights {
            Some(w) => Bm25Weights {
                title: w.title.unwrap_or(current.weights.title),
                tags: w.tags.unwrap_or(current.weights.tags),
                content: w.content.unwrap_or(current.weights.content),
            },
            None => current.weights,
        },
        ignore_patterns: payload
            .ignore_patterns
            .clone()
            .unwrap_or(current.ignore_patterns),
    };

    state.update_settings(new_settings.clone()).await?;
    Ok(Json(new_settings))
}

pub async fn reset_settings(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let settings = state.reset_settings().await?;
    Ok(Json(settings))
}
