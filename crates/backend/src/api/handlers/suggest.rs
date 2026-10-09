use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::api::dtos::SuggestRequestDto;
use crate::api::extractors::ValidatedQuery;
use crate::application::queries::suggest_queries::execute_suggest;
use crate::state::AppState;

pub async fn suggest(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<SuggestRequestDto>,
) -> impl IntoResponse {
    let response = execute_suggest(&state.repositories, &query).await;
    Json(response)
}
