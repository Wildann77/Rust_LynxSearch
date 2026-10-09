use crate::api::dtos::{SuggestRequestDto, SuggestResponseDto};
use crate::state::Repositories;

/// Orkestrasi use case autocomplete saran query (Read query CQS/Hexagonal).
/// Mengeksekusi pencarian prefix pada SearchRepository dengan fallback graceful.
pub async fn execute_suggest(
    repositories: &Repositories,
    request: &SuggestRequestDto,
) -> SuggestResponseDto {
    let limit = request.limit() as usize;
    let suggestions = match repositories.search.suggest(&request.q, limit).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Autocomplete suggest query failed gracefully: {e}");
            Vec::new()
        }
    };

    SuggestResponseDto { suggestions }
}
