use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

use crate::api::dtos::SuggestRequestDto;
use crate::api::extractors::ValidatedQuery;
use crate::state::AppState;

pub async fn suggest(
    _state: State<AppState>,
    ValidatedQuery(_query): ValidatedQuery<SuggestRequestDto>,
) -> impl IntoResponse {
    Json(json!({
        "suggestions": []
    }))
}
