use axum::Json;
use axum::extract::State;

use crate::api::dtos::{SearchRequestDto, SearchResponseDto};
use crate::api::extractors::ValidatedQuery;
use crate::application::queries::execute_search;
use crate::error::AppError;
use crate::state::AppState;

/// Handler untuk endpoint `GET /api/search`.
/// Mendelegasikan parsing query, query DSL builder, dan search execution ke application layer.
pub async fn search(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<SearchRequestDto>,
) -> Result<Json<SearchResponseDto>, AppError> {
    let settings = state.get_settings().await;
    let response = execute_search(&state.repositories, &settings, &query).await?;
    Ok(Json(response))
}
