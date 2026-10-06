use std::str::FromStr;

use crate::api::dtos::{SearchFacetsDto, SearchRequestDto, SearchResponseDto, SearchResultItemDto};
use crate::domain::models::AppSettings;
use crate::domain::models::types::{DocumentType, FilterKey, Language};
use crate::domain::query_parser::QueryParser;
use crate::domain::services::search_query_builder::SearchQueryBuilder;
use crate::error::AppError;
use crate::state::Repositories;

/// Orkestrasi use case pencarian dokumen (Query side pada CQS/Hexagonal Architecture).
/// Menggabungkan query parser domain, penggabungan filter URL eksplisit, query builder DSL,
/// eksekusi pada SearchRepository, dan transformasi ke respons DTO seragam.
pub async fn execute_search(
    repositories: &Repositories,
    settings: &AppSettings,
    request: &SearchRequestDto,
) -> Result<SearchResponseDto, AppError> {
    // 1. Parsing string query mentah (free terms, phrase terms, inline filters)
    let raw_q = request.q.as_deref().unwrap_or("");
    let mut parsed_query = QueryParser::parse(raw_q);

    // 2. Gabungkan filter dari query parameter URL eksplisit
    // Sesuai keputusan arsitektur: Parameter URL eksplisit menimpa atau melengkapi inline filter
    if let Some(doc_type_raw) = request.doc_type.as_deref() {
        let clean = doc_type_raw.trim().to_lowercase();
        if !clean.is_empty() {
            if DocumentType::from_str(&clean).is_ok() {
                parsed_query.filters.insert(FilterKey::Type, clean);
            } else {
                parsed_query.warnings.push(format!(
                    "Filter type '{clean}' tidak dikenali dan diabaikan."
                ));
            }
        }
    }

    if let Some(lang_raw) = request.language.as_deref() {
        let clean = lang_raw.trim().to_lowercase();
        if !clean.is_empty() {
            if Language::from_str(&clean).is_ok() {
                parsed_query.filters.insert(FilterKey::Language, clean);
            } else {
                parsed_query.warnings.push(format!(
                    "Filter language '{clean}' tidak dikenali dan diabaikan."
                ));
            }
        }
    }

    if let Some(tag_raw) = request.tag.as_deref() {
        let clean = tag_raw.trim();
        if !clean.is_empty() {
            parsed_query
                .filters
                .insert(FilterKey::Tag, clean.to_string());
        }
    }

    if let Some(proj_raw) = request.project.as_deref() {
        let clean = proj_raw.trim();
        if !clean.is_empty() {
            parsed_query
                .filters
                .insert(FilterKey::Project, clean.to_string());
        }
    }

    // 3. Bangun Elasticsearch query DSL dengan pagination, weights BM25, and sort
    let builder = SearchQueryBuilder::new(parsed_query.clone())
        .page(request.page() as usize)
        .per_page(request.size() as usize)
        .weights(settings.weights)
        .sort(request.sort.clone());

    let raw_dsl = builder.build_raw().map_err(|e| {
        AppError::Internal(format!("Failed to serialize query DSL to RawValue: {e}"))
    })?;

    // 4. Eksekusi pencarian ke SearchRepository port
    let raw_res = repositories.search.search(&raw_dsl).await?;

    // 5. Parse respons JSON Elasticsearch ke domain SearchExecutionResult
    let exec_result = raw_res.parse_execution_result()?;

    // 6. Petakan domain SearchHit ke SearchResultItemDto untuk presentasi UI
    let items: Vec<SearchResultItemDto> = exec_result
        .hits
        .into_iter()
        .map(|hit| SearchResultItemDto {
            id: hit.id,
            title: hit.title,
            relative_path: hit.relative_path,
            project: hit.project,
            doc_type: hit.doc_type,
            language: hit.language,
            tags: hit.tags,
            highlights: hit.highlights,
            score: hit.score,
            file_size: hit.file_size_bytes,
            updated_at: hit.modified_at,
        })
        .collect();

    // 7. Bentuk SearchResponseDto (items dan results disediakan untuk kompatibilitas ganda)
    Ok(SearchResponseDto {
        query: request.normalized_query().unwrap_or("").to_string(),
        page: request.page(),
        size: request.size(),
        total: exec_result.total,
        took_ms: exec_result.took_ms,
        results: items.clone(),
        items,
        facets: SearchFacetsDto::default(),
        warnings: parsed_query.warnings,
    })
}
