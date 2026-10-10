use super::id::{DocumentId, FolderId};
use super::types::{DocumentType, FilterKey, Language};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub raw_query: String,
    pub free_terms: Vec<String>,
    pub phrase_terms: Vec<String>,
    pub filters: HashMap<FilterKey, String>,
    pub warnings: Vec<String>,
}

impl SearchQuery {
    pub fn new(raw_query: impl Into<String>) -> Self {
        Self {
            raw_query: raw_query.into(),
            free_terms: Vec::new(),
            phrase_terms: Vec::new(),
            filters: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new(String::new())
    }

    pub fn is_empty(&self) -> bool {
        self.free_terms.is_empty() && self.phrase_terms.is_empty() && self.filters.is_empty()
    }

    pub fn has_filters(&self) -> bool {
        !self.filters.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Highlight fragment hasil pencarian dengan nomor baris kode/dokumen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHighlight {
    pub snippet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
}

/// Satu dokumen hit hasil pencarian teks penuh / kode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub title: String,
    pub relative_path: String,
    #[serde(default)]
    pub absolute_path: String,
    pub doc_type: DocumentType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default)]
    pub file_size_bytes: u64,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<SearchHighlight>,
}

/// Bucket agregasi facet kategori pencarian pada layer domain.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FacetBucket {
    pub key: String,
    pub doc_count: u64,
}

/// Kelompok facet pencarian lengkap (extensions, types, languages, tags, projects).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchFacets {
    #[serde(default)]
    pub extensions: Vec<FacetBucket>,
    #[serde(default)]
    pub types: Vec<FacetBucket>,
    #[serde(default)]
    pub languages: Vec<FacetBucket>,
    #[serde(default)]
    pub tags: Vec<FacetBucket>,
    #[serde(default)]
    pub projects: Vec<FacetBucket>,
}

/// Hasil eksekusi query pencarian terstruktur pada layer aplikasi / domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchExecutionResult {
    pub total: u64,
    pub took_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub es_took_ms: Option<u64>,
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    pub facets: SearchFacets,
}

/// Algoritma penghitungan nomor baris presisi berdasarkan kemunculan newline sebelum offset match.
/// Formula: 1 + count('\n' sebelum offset kecocokan pada raw content).
pub fn extract_line_number(content: &str, snippet: &str) -> Option<usize> {
    if content.is_empty() || snippet.is_empty() {
        return None;
    }

    // 1. Bersihkan marker highlight <em> dan </em> untuk mendapatkan plain text snippet
    let clean_snippet = snippet.replace("<em>", "").replace("</em>", "");
    let clean_trimmed = clean_snippet.trim();

    // Hitung offset keyword match pertama di dalam snippet jika ada tag <em>
    let match_offset = snippet
        .find("<em>")
        .map(|em_pos| {
            snippet[..em_pos]
                .replace("<em>", "")
                .replace("</em>", "")
                .len()
        })
        .unwrap_or(0);

    // 2. Coba cari kemunculan exact snippet yang telah dibersihkan
    if !clean_trimmed.is_empty()
        && let Some(pos) = content.find(clean_trimmed)
    {
        let target_pos = (pos + match_offset).min(content.len());
        let line = 1 + content[..target_pos].chars().filter(|&c| c == '\n').count();
        return Some(line);
    }

    // 3. Fallback jika snippet diawali/diakhiri ellipsis (...) dari fragmenter
    let clean_core = clean_trimmed
        .trim_matches(['.', '…', ' '].as_slice())
        .trim();
    if !clean_core.is_empty()
        && let Some(pos) = content.find(clean_core)
    {
        let target_pos = (pos + match_offset).min(content.len());
        let line = 1 + content[..target_pos].chars().filter(|&c| c == '\n').count();
        return Some(line);
    }

    // 4. Fallback jika snippet terpotong: cari keyword yang dibungkus <em>...</em>
    let mut search_idx = 0;
    while let Some(start_tag) = snippet[search_idx..].find("<em>") {
        let abs_start = search_idx + start_tag + 4;
        if let Some(end_tag) = snippet[abs_start..].find("</em>") {
            let keyword = &snippet[abs_start..abs_start + end_tag];
            if !keyword.is_empty()
                && let Some(pos) = content.find(keyword)
            {
                let line = 1 + content[..pos].chars().filter(|&c| c == '\n').count();
                return Some(line);
            }
            search_idx = abs_start + end_tag + 5;
        } else {
            break;
        }
    }

    None
}

/// Memulihkan dan mempertahankan indentasi awal cuplikan kode jika terpotong oleh Elasticsearch,
/// dengan mengacu pada baris mentah di `content` pada `line_number`.
pub fn preserve_and_restore_indentation(
    snippet: &str,
    content: &str,
    line_number: Option<usize>,
) -> String {
    let clean = snippet.trim_end();
    if let Some(num) = line_number
        && num > 0
        && let Some(raw_line) = content.lines().nth(num - 1)
    {
        let raw_indent_len = raw_line.len() - raw_line.trim_start().len();
        if raw_indent_len > 0 {
            let raw_indent = &raw_line[..raw_indent_len];
            let clean_indent_len = clean.len() - clean.trim_start().len();
            if clean_indent_len < raw_indent_len {
                return format!("{raw_indent}{}", clean.trim_start());
            }
        }
    }
    clean.to_string()
}

#[derive(Deserialize)]
struct RawEsResponse {
    took: Option<u64>,
    hits: Option<RawEsHitsBlock>,
    aggregations: Option<HashMap<String, RawEsAggregation>>,
}

#[derive(Deserialize)]
struct RawEsAggregation {
    buckets: Option<Vec<RawEsBucket>>,
}

#[derive(Deserialize)]
struct RawEsBucket {
    key: serde_json::Value,
    doc_count: u64,
}

#[derive(Deserialize)]
struct RawEsHitsBlock {
    total: Option<RawEsTotal>,
    hits: Option<Vec<RawEsHit>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEsTotal {
    Object { value: u64 },
    Integer(u64),
}

#[derive(Deserialize)]
struct RawEsHit {
    _id: Option<String>,
    _score: Option<f32>,
    _source: Option<serde_json::Value>,
    highlight: Option<HashMap<String, Vec<String>>>,
}

/// Parse respons mentah JSON dari Elasticsearch ke domain struct `SearchExecutionResult`.
pub fn parse_search_execution_result(
    raw_json: &str,
    client_took_ms: u64,
) -> Result<SearchExecutionResult, AppError> {
    let parsed: RawEsResponse = serde_json::from_str(raw_json).map_err(|e| {
        AppError::Internal(format!("Failed to parse Elasticsearch response JSON: {e}"))
    })?;

    let hits_block = parsed.hits.unwrap_or(RawEsHitsBlock {
        total: None,
        hits: None,
    });

    let raw_hits = hits_block.hits.unwrap_or_default();

    let total = match hits_block.total {
        Some(RawEsTotal::Object { value }) => value,
        Some(RawEsTotal::Integer(val)) => val,
        None => raw_hits.len() as u64,
    };

    let mut hits = Vec::with_capacity(raw_hits.len());

    for hit in raw_hits {
        let score = hit._score.unwrap_or(0.0);
        let source = hit._source.unwrap_or_default();

        let id = source
            .get("id")
            .and_then(|v| v.as_str())
            .or(hit._id.as_deref())
            .and_then(|s| Uuid::from_str(s).ok())
            .map(DocumentId::from_uuid)
            .unwrap_or_else(|| DocumentId::from_uuid(Uuid::new_v4()));

        let folder_id = source
            .get("folder_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::from_str(s).ok())
            .map(FolderId::from_uuid)
            .unwrap_or_default();

        let title = source
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let relative_path = source
            .get("relative_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let absolute_path = source
            .get("absolute_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let content = source.get("content").and_then(|v| v.as_str()).unwrap_or("");

        let doc_type = source
            .get("type")
            .and_then(|v| v.as_str())
            .and_then(|s| DocumentType::from_str(s).ok())
            .unwrap_or(DocumentType::Doc);

        let language = source
            .get("language")
            .and_then(|v| v.as_str())
            .and_then(|s| Language::from_str(s).ok());

        let tags: Vec<String> = source
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let project = source
            .get("project")
            .and_then(|v| v.as_str())
            .map(String::from);

        let file_size_bytes = source
            .get("file_size_bytes")
            .or_else(|| source.get("file_size"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mut highlights = Vec::new();
        if let Some(hl_map) = hit.highlight {
            let mut ordered_fields: Vec<String> = hl_map.keys().cloned().collect();
            ordered_fields.sort_by(|a, b| {
                let a_is_content = a == "content" || a == "content.code";
                let b_is_content = b == "content" || b == "content.code";
                if a_is_content && !b_is_content {
                    std::cmp::Ordering::Less
                } else if !a_is_content && b_is_content {
                    std::cmp::Ordering::Greater
                } else {
                    a.cmp(b)
                }
            });

            for field in ordered_fields {
                if let Some(snippets) = hl_map.get(&field) {
                    for snippet in snippets {
                        let is_content_field = field == "content" || field == "content.code";
                        if is_content_field && snippet.contains('\n') && snippet.contains("<em>") {
                            for line in snippet.lines() {
                                if line.contains("<em>") {
                                    let line_number = if !content.is_empty() {
                                        extract_line_number(content, line)
                                    } else {
                                        None
                                    };
                                    let line_str = preserve_and_restore_indentation(
                                        line,
                                        content,
                                        line_number,
                                    );
                                    let hl = SearchHighlight {
                                        snippet: line_str,
                                        line_number,
                                    };
                                    if !highlights.contains(&hl) {
                                        highlights.push(hl);
                                    }
                                }
                            }
                        } else {
                            let line_number = if is_content_field && !content.is_empty() {
                                extract_line_number(content, snippet)
                            } else {
                                None
                            };
                            let snippet_str = if is_content_field {
                                preserve_and_restore_indentation(snippet, content, line_number)
                            } else {
                                snippet.clone()
                            };
                            let hl = SearchHighlight {
                                snippet: snippet_str,
                                line_number,
                            };
                            if !highlights.contains(&hl) {
                                highlights.push(hl);
                            }
                        }
                    }
                }
            }
        }

        let modified_at = source
            .get("modified_at")
            .or_else(|| source.get("updated_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        hits.push(SearchHit {
            id,
            folder_id,
            title,
            relative_path,
            absolute_path,
            doc_type,
            language,
            tags,
            project,
            file_size_bytes,
            score,
            modified_at,
            highlights,
        });
    }

    let extract_buckets = |key: &str| -> Vec<FacetBucket> {
        parsed
            .aggregations
            .as_ref()
            .and_then(|aggs| aggs.get(key))
            .and_then(|agg| agg.buckets.as_ref())
            .map(|buckets| {
                buckets
                    .iter()
                    .filter_map(|b| {
                        let key_str = match &b.key {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(bv) => bv.to_string(),
                            _ => return None,
                        };
                        if key_str.is_empty() {
                            None
                        } else {
                            Some(FacetBucket {
                                key: key_str,
                                doc_count: b.doc_count,
                            })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let facets = SearchFacets {
        extensions: extract_buckets("extensions"),
        types: extract_buckets("types"),
        languages: extract_buckets("languages"),
        tags: extract_buckets("tags"),
        projects: extract_buckets("projects"),
    };

    Ok(SearchExecutionResult {
        total,
        took_ms: client_took_ms,
        es_took_ms: parsed.took,
        hits,
        facets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_empty_and_predicates() {
        let empty_query = SearchQuery::empty();
        assert!(empty_query.is_empty());
        assert!(!empty_query.has_filters());
        assert!(!empty_query.has_warnings());
        assert_eq!(empty_query.raw_query, "");

        let mut query = SearchQuery::new("rust ownership");
        assert!(query.is_empty());
        query.free_terms.push("rust".to_string());
        assert!(!query.is_empty());

        query
            .filters
            .insert(FilterKey::Language, "rust".to_string());
        assert!(query.has_filters());

        query
            .warnings
            .push("Filter 'foo' tidak dikenali dan diabaikan.".to_string());
        assert!(query.has_warnings());
    }

    #[test]
    fn test_search_query_serde_roundtrip() {
        let mut query = SearchQuery::new("ownership language:rust");
        query.free_terms.push("ownership".to_string());
        query.phrase_terms.push("memory safety".to_string());
        query
            .filters
            .insert(FilterKey::Language, "rust".to_string());
        query
            .warnings
            .push("Filter 'x' tidak dikenali dan diabaikan.".to_string());

        let json = serde_json::to_string(&query).expect("Serialize to JSON");
        let decoded: SearchQuery = serde_json::from_str(&json).expect("Deserialize from JSON");

        assert_eq!(decoded, query);
    }

    #[test]
    fn test_extract_line_number_cases() {
        let content = "line 1\nline 2: match here\nline 3\nline 4: final match";

        // First line
        let first_snippet = "match <em>line 1</em>";
        assert_eq!(extract_line_number(content, first_snippet), Some(1));

        // Middle line
        let mid_snippet = "line 2: <em>match</em> here";
        assert_eq!(extract_line_number(content, mid_snippet), Some(2));

        // Last line
        let last_snippet = "line 4: <em>final</em> match";
        assert_eq!(extract_line_number(content, last_snippet), Some(4));

        // Multiline snippet
        let multi_snippet = "line 2: match here\nline 3";
        assert_eq!(extract_line_number(content, multi_snippet), Some(2));

        // Empty content or snippet
        assert_eq!(extract_line_number("", "snippet"), None);
        assert_eq!(extract_line_number(content, ""), None);

        // Not found
        assert_eq!(
            extract_line_number(content, "non_existent <em>word</em>"),
            None
        );
    }

    #[test]
    fn test_parse_search_execution_result_full_es8() {
        let es_json = r#"{
            "took": 7,
            "timed_out": false,
            "_shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 },
            "hits": {
                "total": { "value": 1, "relation": "eq" },
                "max_score": 2.45,
                "hits": [
                    {
                        "_index": "lynx_documents_v1",
                        "_id": "550e8400-e29b-41d4-a716-446655440000",
                        "_score": 2.45,
                        "_source": {
                            "id": "550e8400-e29b-41d4-a716-446655440000",
                            "folder_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                            "title": "Main entry point",
                            "relative_path": "src/main.rs",
                            "absolute_path": "/home/user/code/src/main.rs",
                            "content": "use std::io;\nfn main() {\n    println!(\"ownership\");\n}\n",
                            "type": "code",
                            "language": "rust",
                            "tags": ["cli", "main"],
                            "project": "lynx",
                            "file_size_bytes": 1024
                        },
                        "highlight": {
                            "content": [
                                "println!(\"<em>ownership</em>\");"
                            ],
                            "title": [
                                "<em>Main</em> entry point"
                            ]
                        }
                    }
                ]
            }
        }"#;

        let result = parse_search_execution_result(es_json, 12).expect("valid parse");
        assert_eq!(result.total, 1);
        assert_eq!(result.took_ms, 12);
        assert_eq!(result.es_took_ms, Some(7));
        assert_eq!(result.hits.len(), 1);

        let hit = &result.hits[0];
        assert_eq!(hit.title, "Main entry point");
        assert_eq!(hit.relative_path, "src/main.rs");
        assert_eq!(hit.doc_type, DocumentType::Code);
        assert_eq!(hit.language, Some(Language::Rust));
        assert_eq!(hit.tags, vec!["cli".to_string(), "main".to_string()]);
        assert_eq!(hit.score, 2.45);
        assert_eq!(hit.highlights.len(), 2);

        // Content highlight must have line_number = 3 (line 3 has println!("ownership"))
        let content_hl = hit
            .highlights
            .iter()
            .find(|h| h.snippet.contains("ownership"))
            .expect("content highlight present");
        assert_eq!(content_hl.line_number, Some(3));
        assert_eq!(content_hl.snippet, "    println!(\"<em>ownership</em>\");");
    }

    #[test]
    fn test_preserve_and_restore_indentation_cases() {
        let content = "fn test() {\n    let val = 42;\n\t\tlet tab_indented = true;\n}";

        // Line 2: 4 spaces indent, snippet without indent
        let snippet1 = "let <em>val</em> = 42;";
        let restored1 = preserve_and_restore_indentation(snippet1, content, Some(2));
        assert_eq!(restored1, "    let <em>val</em> = 42;");

        // Line 2: 4 spaces indent, snippet already has 4 spaces indent
        let snippet2 = "    let <em>val</em> = 42;";
        let restored2 = preserve_and_restore_indentation(snippet2, content, Some(2));
        assert_eq!(restored2, "    let <em>val</em> = 42;");

        // Line 3: 2 tabs indent, snippet without indent
        let snippet3 = "let <em>tab_indented</em> = true;";
        let restored3 = preserve_and_restore_indentation(snippet3, content, Some(3));
        assert_eq!(restored3, "\t\tlet <em>tab_indented</em> = true;");

        // Line 1: no indent
        let snippet_top = "fn <em>test</em>() {";
        let restored_top = preserve_and_restore_indentation(snippet_top, content, Some(1));
        assert_eq!(restored_top, "fn <em>test</em>() {");

        // Invalid line number: retains snippet
        let restored_none = preserve_and_restore_indentation(snippet1, content, None);
        assert_eq!(restored_none, snippet1);
    }

    #[test]
    fn test_parse_search_execution_result_flat_total_fallback() {
        let es_json = r#"{
            "hits": {
                "total": 0,
                "hits": []
            }
        }"#;

        let result = parse_search_execution_result(es_json, 5).expect("valid parse");
        assert_eq!(result.total, 0);
        assert_eq!(result.took_ms, 5);
        assert_eq!(result.es_took_ms, None);
        assert!(result.hits.is_empty());
    }

    #[test]
    fn test_parse_search_execution_result_malformed_json() {
        let bad_json = "not json at all";
        let err = parse_search_execution_result(bad_json, 10).unwrap_err();
        match err {
            AppError::Internal(msg) => {
                assert!(msg.contains("Failed to parse Elasticsearch response JSON"));
            }
            other => panic!("expected AppError::Internal, got {other:?}"),
        }
    }

    #[test]
    fn test_extract_line_number_first_middle_last_multiline_and_ellipsis() {
        let content = "\
// Line 1: Header
fn alpha() {
    let a = 1;
}
// Line 5: Middle line
fn beta() {
    let target = 42;
}
// Line 9: Almost end
// Line 10: Tail line";

        // 1. First line match
        let hl_first = "// Line 1: <em>Header</em>";
        assert_eq!(extract_line_number(content, hl_first), Some(1));

        // 2. Middle line match
        let hl_mid = "<em>Middle</em> line";
        assert_eq!(extract_line_number(content, hl_mid), Some(5));

        // 3. Last line match
        let hl_last = "Tail <em>line</em>";
        assert_eq!(extract_line_number(content, hl_last), Some(10));

        // 4. Multiline snippet where match keyword is on the second line of snippet
        let hl_multi = "fn beta() {\n    let <em>target</em> = 42;";
        assert_eq!(extract_line_number(content, hl_multi), Some(7));

        // 5. Snippet with leading and trailing ellipses (...) from fragmenter
        let hl_ellipsis = "... let <em>target</em> = 42; ...";
        assert_eq!(extract_line_number(content, hl_ellipsis), Some(7));

        // 6. Non-matching snippet returns None or fallback
        assert_eq!(extract_line_number(content, "non_existent_token"), None);
        assert_eq!(extract_line_number("", hl_mid), None);
    }

    #[test]
    fn test_parse_search_execution_result_multiple_highlight_fragments_and_deduplication() {
        let es_json = r#"{
            "took": 15,
            "hits": {
                "total": { "value": 1 },
                "hits": [
                    {
                        "_id": "550e8400-e29b-41d4-a716-446655440000",
                        "_score": 5.2,
                        "_source": {
                            "folder_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                            "title": "Config Loader",
                            "relative_path": "src/config.rs",
                            "content": "pub struct Config;\n\nimpl Config {\n    pub fn load() -> Self {\n        Config\n    }\n}\n",
                            "type": "code",
                            "language": "rust",
                            "tags": ["config"]
                        },
                        "highlight": {
                            "content": [
                                "pub struct <em>Config</em>;",
                                "impl <em>Config</em> {",
                                "pub struct <em>Config</em>;"
                            ],
                            "title": [
                                "<em>Config</em> Loader"
                            ]
                        }
                    }
                ]
            }
        }"#;

        let result = parse_search_execution_result(es_json, 15).expect("valid parse");
        let hit = &result.hits[0];

        // Highlight should contain 2 distinct content highlights (1 deduplicated) + 1 title highlight = 3 highlights
        assert_eq!(hit.highlights.len(), 3);

        // Content highlight 1: line 1
        assert_eq!(hit.highlights[0].snippet, "pub struct <em>Config</em>;");
        assert_eq!(hit.highlights[0].line_number, Some(1));

        // Content highlight 2: line 3
        assert_eq!(hit.highlights[1].snippet, "impl <em>Config</em> {");
        assert_eq!(hit.highlights[1].line_number, Some(3));

        // Title highlight: no line number
        assert_eq!(hit.highlights[2].snippet, "<em>Config</em> Loader");
        assert_eq!(hit.highlights[2].line_number, None);
    }

    #[test]
    fn test_parse_search_execution_result_with_facets() {
        let es_json = r#"{
            "took": 7,
            "hits": { "total": { "value": 3 }, "hits": [] },
            "aggregations": {
                "extensions": {
                    "buckets": [
                        { "key": "rs", "doc_count": 2 },
                        { "key": "md", "doc_count": 1 }
                    ]
                },
                "types": {
                    "buckets": [
                        { "key": "code", "doc_count": 2 },
                        { "key": "doc", "doc_count": 1 }
                    ]
                },
                "languages": {
                    "buckets": [
                        { "key": "rust", "doc_count": 2 },
                        { "key": "markdown", "doc_count": 1 }
                    ]
                },
                "tags": {
                    "buckets": [
                        { "key": "cli", "doc_count": 2 }
                    ]
                },
                "projects": {
                    "buckets": [
                        { "key": "backend", "doc_count": 3 }
                    ]
                }
            }
        }"#;

        let result = parse_search_execution_result(es_json, 10).expect("valid parse");
        assert_eq!(result.total, 3);
        assert_eq!(result.facets.extensions.len(), 2);
        assert_eq!(result.facets.extensions[0].key, "rs");
        assert_eq!(result.facets.extensions[0].doc_count, 2);
        assert_eq!(result.facets.types.len(), 2);
        assert_eq!(result.facets.languages.len(), 2);
        assert_eq!(result.facets.tags.len(), 1);
        assert_eq!(result.facets.projects.len(), 1);

        // Test missing aggregations block gracefully defaults to empty
        let empty_es_json = r#"{ "took": 1, "hits": { "total": { "value": 0 }, "hits": [] } }"#;
        let empty_res = parse_search_execution_result(empty_es_json, 1).expect("valid parse");
        assert!(empty_res.facets.extensions.is_empty());
        assert!(empty_res.facets.types.is_empty());
        assert!(empty_res.facets.languages.is_empty());
        assert!(empty_res.facets.tags.is_empty());
        assert!(empty_res.facets.projects.is_empty());
    }
}
