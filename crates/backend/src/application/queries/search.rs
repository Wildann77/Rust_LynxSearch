use std::str::FromStr;

use crate::api::dtos::{
    FacetBucketDto, SearchFacetsDto, SearchRequestDto, SearchResponseDto, SearchResultItemDto,
};
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
        let clean = doc_type_raw.trim();
        if !clean.is_empty() {
            let mut valid_parts = Vec::new();
            for part in clean.split(',') {
                let trimmed = part.trim().to_lowercase();
                if DocumentType::from_str(&trimmed).is_ok() {
                    valid_parts.push(trimmed);
                } else if !trimmed.is_empty() {
                    parsed_query.warnings.push(format!(
                        "Filter type '{trimmed}' tidak dikenali dan diabaikan."
                    ));
                }
            }
            if !valid_parts.is_empty() {
                let joined = valid_parts.join(",");
                parsed_query
                    .filters
                    .entry(FilterKey::Type)
                    .and_modify(|existing| {
                        for p in &valid_parts {
                            if !existing.split(',').any(|x| x == p) {
                                existing.push(',');
                                existing.push_str(p);
                            }
                        }
                    })
                    .or_insert(joined);
            }
        }
    }

    if let Some(lang_raw) = request.language.as_deref() {
        let clean = lang_raw.trim();
        if !clean.is_empty() {
            let mut valid_parts = Vec::new();
            for part in clean.split(',') {
                let trimmed = part.trim().to_lowercase();
                if trimmed.is_empty() {
                    continue;
                }
                let lang = Language::from_str(&trimmed).expect("infallible");
                if lang.is_known() {
                    valid_parts.push(trimmed);
                } else {
                    parsed_query.warnings.push(format!(
                        "Filter language '{trimmed}' tidak dikenali dan diabaikan."
                    ));
                }
            }
            if !valid_parts.is_empty() {
                let joined = valid_parts.join(",");
                parsed_query
                    .filters
                    .entry(FilterKey::Language)
                    .and_modify(|existing| {
                        for p in &valid_parts {
                            if !existing.split(',').any(|x| x == p) {
                                existing.push(',');
                                existing.push_str(p);
                            }
                        }
                    })
                    .or_insert(joined);
            }
        }
    }

    if let Some(tag_raw) = request.tag.as_deref() {
        let clean = tag_raw.trim();
        if !clean.is_empty() {
            parsed_query
                .filters
                .entry(FilterKey::Tag)
                .and_modify(|existing| {
                    for p in clean.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        if !existing.split(',').any(|x| x == p) {
                            existing.push(',');
                            existing.push_str(p);
                        }
                    }
                })
                .or_insert_with(|| clean.to_string());
        }
    }

    if let Some(proj_raw) = request.project.as_deref() {
        let clean = proj_raw.trim();
        if !clean.is_empty() {
            parsed_query
                .filters
                .entry(FilterKey::Project)
                .and_modify(|existing| {
                    for p in clean.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        if !existing.split(',').any(|x| x == p) {
                            existing.push(',');
                            existing.push_str(p);
                        }
                    }
                })
                .or_insert_with(|| clean.to_string());
        }
    }

    if let Some(ext_raw) = request.extension.as_deref() {
        let clean = ext_raw.trim();
        if !clean.is_empty() {
            let mut normalized_parts = Vec::new();
            for part in clean.split(',') {
                let stripped = part.trim().trim_start_matches('.');
                if !stripped.is_empty() {
                    normalized_parts.push(stripped.to_ascii_lowercase());
                }
            }
            if !normalized_parts.is_empty() {
                let joined = normalized_parts.join(",");
                parsed_query
                    .filters
                    .entry(FilterKey::Extension)
                    .and_modify(|existing| {
                        for p in &normalized_parts {
                            if !existing.split(',').any(|x| x == p) {
                                existing.push(',');
                                existing.push_str(p);
                            }
                        }
                    })
                    .or_insert(joined);
            }
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

    let facets = SearchFacetsDto {
        extensions: exec_result
            .facets
            .extensions
            .into_iter()
            .map(|b| FacetBucketDto {
                key: b.key,
                doc_count: b.doc_count,
            })
            .collect(),
        types: exec_result
            .facets
            .types
            .into_iter()
            .map(|b| FacetBucketDto {
                key: b.key,
                doc_count: b.doc_count,
            })
            .collect(),
        languages: exec_result
            .facets
            .languages
            .into_iter()
            .map(|b| FacetBucketDto {
                key: b.key,
                doc_count: b.doc_count,
            })
            .collect(),
        tags: exec_result
            .facets
            .tags
            .into_iter()
            .map(|b| FacetBucketDto {
                key: b.key,
                doc_count: b.doc_count,
            })
            .collect(),
        projects: exec_result
            .facets
            .projects
            .into_iter()
            .map(|b| FacetBucketDto {
                key: b.key,
                doc_count: b.doc_count,
            })
            .collect(),
    };

    // 7. Bentuk SearchResponseDto (items dan results disediakan untuk kompatibilitas ganda)
    Ok(SearchResponseDto {
        query: request.normalized_query().unwrap_or("").to_string(),
        page: request.page(),
        size: request.size(),
        total: exec_result.total,
        took_ms: exec_result.took_ms,
        results: items.clone(),
        items,
        facets,
        warnings: parsed_query.warnings,
    })
}
