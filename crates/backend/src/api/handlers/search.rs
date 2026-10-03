use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

use crate::api::dtos::SearchRequestDto;
use crate::api::extractors::ValidatedQuery;
use crate::state::AppState;

pub async fn search(
    _state: State<AppState>,
    ValidatedQuery(query): ValidatedQuery<SearchRequestDto>,
) -> impl IntoResponse {
    Json(json!({
        "query": query.normalized_query().unwrap_or(""),
        "page": query.page(),
        "size": query.size(),
        "total": 0,
        "took_ms": 0,
        "items": [],
        "facets": {
            "types": [],
            "languages": [],
            "tags": [],
            "projects": []
        },
        "warnings": []
    }))
}
