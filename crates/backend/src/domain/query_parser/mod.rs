use crate::domain::models::search::SearchQuery;
use crate::domain::models::types::{DocumentType, FilterKey, Language};
use std::collections::HashMap;
use std::str::FromStr;

pub struct QueryParser;

impl QueryParser {
    pub fn parse(input: &str) -> SearchQuery {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return SearchQuery::empty();
        }

        let mut free_terms = Vec::new();
        let mut phrase_terms = Vec::new();
        let mut filters = HashMap::new();
        let mut warnings = Vec::new();

        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            // Skip whitespace
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= len {
                break;
            }

            // Check if token starts with quote: "phrase"
            if chars[i] == '"' {
                i += 1; // skip opening quote
                let mut phrase = String::new();
                let mut closed = false;

                while i < len {
                    if chars[i] == '\\' && i + 1 < len && chars[i + 1] == '"' {
                        phrase.push('"');
                        i += 2;
                    } else if chars[i] == '"' {
                        closed = true;
                        i += 1;
                        break;
                    } else {
                        phrase.push(chars[i]);
                        i += 1;
                    }
                }

                if closed {
                    let clean = phrase.trim();
                    if !clean.is_empty() {
                        phrase_terms.push(clean.to_string());
                    }
                } else {
                    // Fallback for unclosed quote: split phrase words into free_terms
                    let mut unclosed_token = String::from("\"");
                    unclosed_token.push_str(&phrase);
                    for part in unclosed_token.split_whitespace() {
                        free_terms.push(part.to_string());
                    }
                }
                continue;
            }

            // Otherwise, read token until whitespace or quote
            let token_start = i;
            while i < len && !chars[i].is_whitespace() {
                // If we hit a quote that is part of key:"value"
                if chars[i] == '"' && i > token_start && chars[i - 1] == ':' {
                    break;
                }
                i += 1;
            }

            let token_slice: String = chars[token_start..i].iter().collect();

            // Check if this is a key:"value" token where i stopped at '"'
            if i < len && chars[i] == '"' && token_slice.ends_with(':') {
                let key = token_slice.trim_end_matches(':');
                i += 1; // skip opening quote
                let mut quoted_val = String::new();
                let mut closed = false;

                while i < len {
                    if chars[i] == '\\' && i + 1 < len && chars[i + 1] == '"' {
                        quoted_val.push('"');
                        i += 2;
                    } else if chars[i] == '"' {
                        closed = true;
                        i += 1;
                        break;
                    } else {
                        quoted_val.push(chars[i]);
                        i += 1;
                    }
                }

                if !closed {
                    // Malformed token: unclosed quote in filter value
                    warnings.push(format!(
                        "Filter '{key}' memiliki tanda kutip yang tidak tertutup dan diabaikan."
                    ));
                    let mut fallback = token_slice;
                    fallback.push('"');
                    fallback.push_str(&quoted_val);
                    for part in fallback.split_whitespace() {
                        free_terms.push(part.to_string());
                    }
                } else if is_identifier(key) {
                    apply_filter_value(key, &quoted_val, &mut filters, &mut warnings);
                } else {
                    let mut combined = token_slice;
                    combined.push('"');
                    combined.push_str(&quoted_val);
                    combined.push('"');
                    free_terms.push(combined);
                }
                continue;
            }

            // Normal token: check if it's key:value
            if let Some((key, val)) = parse_filter_pair(&token_slice) {
                apply_filter_value(key, val, &mut filters, &mut warnings);
                continue;
            }

            // Otherwise, free term
            free_terms.push(token_slice);
        }

        SearchQuery {
            raw_query: trimmed.to_string(),
            free_terms,
            phrase_terms,
            filters,
            warnings,
        }
    }
}

fn apply_filter_value(
    key: &str,
    raw_val: &str,
    filters: &mut HashMap<FilterKey, String>,
    warnings: &mut Vec<String>,
) {
    let clean_val = raw_val.trim();
    if clean_val.is_empty() {
        warnings.push(format!(
            "Filter '{key}' memiliki nilai kosong dan diabaikan."
        ));
        return;
    }

    let filter_key = match FilterKey::from_str(key) {
        Ok(k) => k,
        Err(_) => {
            warnings.push(format!("Filter '{key}' tidak dikenali dan diabaikan."));
            return;
        }
    };

    match filter_key {
        FilterKey::Language => {
            let mut valid_parts = Vec::new();
            let mut had_token = false;
            for part in clean_val.split(',') {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                had_token = true;
                let normalized = trimmed.to_ascii_lowercase();
                let lang = Language::from_str(&normalized).expect("infallible");
                if lang.is_known() {
                    valid_parts.push(normalized);
                } else {
                    warnings.push(format!(
                        "Filter language '{trimmed}' tidak dikenali dan diabaikan."
                    ));
                }
            }
            if !had_token {
                warnings.push(format!(
                    "Filter '{key}' memiliki nilai kosong dan diabaikan."
                ));
            } else if !valid_parts.is_empty() {
                merge_filter_value(filters, filter_key, &valid_parts.join(","));
            }
        }
        FilterKey::Extension => {
            let mut normalized_parts = Vec::new();
            for part in clean_val.split(',') {
                let stripped = part.trim().trim_start_matches('.');
                if !stripped.is_empty() {
                    normalized_parts.push(stripped.to_ascii_lowercase());
                }
            }
            if normalized_parts.is_empty() {
                warnings.push(format!(
                    "Filter '{key}' memiliki nilai kosong dan diabaikan."
                ));
                return;
            }
            merge_filter_value(filters, filter_key, &normalized_parts.join(","));
        }
        FilterKey::Type => {
            let mut valid_parts = Vec::new();
            for part in clean_val.split(',') {
                let trimmed = part.trim();
                let normalized = trimmed.to_ascii_lowercase();
                if DocumentType::from_str(&normalized).is_ok() {
                    valid_parts.push(normalized);
                } else if !trimmed.is_empty() {
                    warnings.push(format!(
                        "Filter type '{trimmed}' tidak dikenali dan diabaikan."
                    ));
                }
            }
            if !valid_parts.is_empty() {
                merge_filter_value(filters, filter_key, &valid_parts.join(","));
            }
        }
        FilterKey::Tag | FilterKey::Project => {
            merge_filter_value(filters, filter_key, clean_val);
        }
    }
}

fn merge_filter_value(filters: &mut HashMap<FilterKey, String>, key: FilterKey, new_val: &str) {
    filters
        .entry(key)
        .and_modify(|existing| {
            let mut parts: Vec<String> = existing.split(',').map(|s| s.to_string()).collect();
            for part in new_val.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() && !parts.iter().any(|p| p == trimmed) {
                    parts.push(trimmed.to_string());
                }
            }
            *existing = parts.join(",");
        })
        .or_insert_with(|| {
            let mut parts = Vec::new();
            for part in new_val.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() && !parts.contains(&trimmed) {
                    parts.push(trimmed);
                }
            }
            parts.join(",")
        });
}

fn parse_filter_pair(token: &str) -> Option<(&str, &str)> {
    if token.contains("://") || token.contains("::") {
        return None;
    }
    let (key, val) = token.split_once(':')?;
    if is_identifier(key) {
        Some((key, val))
    } else {
        None
    }
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_and_whitespace() {
        assert!(QueryParser::parse("").is_empty());
        assert!(QueryParser::parse("   ").is_empty());
        assert!(QueryParser::parse("\t\n  ").is_empty());
    }

    #[test]
    fn test_parse_free_terms_multi_word() {
        let q = QueryParser::parse("rust async tokio");
        assert_eq!(q.raw_query, "rust async tokio");
        assert_eq!(q.free_terms, vec!["rust", "async", "tokio"]);
        assert!(q.phrase_terms.is_empty());
        assert!(q.filters.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_quoted_phrase() {
        let q = QueryParser::parse(r#"rust "borrow checker" ownership"#);
        assert_eq!(q.free_terms, vec!["rust", "ownership"]);
        assert_eq!(q.phrase_terms, vec!["borrow checker"]);
        assert!(q.filters.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_multiple_quoted_phrases() {
        let q = QueryParser::parse(r#""tokio runtime" "clean architecture""#);
        assert!(q.free_terms.is_empty());
        assert_eq!(q.phrase_terms, vec!["tokio runtime", "clean architecture"]);
        assert!(q.filters.is_empty());
    }

    #[test]
    fn test_parse_empty_quotes() {
        let q = QueryParser::parse(r#"rust "" "   " tokio"#);
        assert_eq!(q.free_terms, vec!["rust", "tokio"]);
        assert!(q.phrase_terms.is_empty());
    }

    #[test]
    fn test_parse_unclosed_quote_fallback() {
        let q = QueryParser::parse(r#"rust "unclosed phrase"#);
        assert_eq!(q.free_terms, vec!["rust", "\"unclosed", "phrase"]);
        assert!(q.phrase_terms.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_inline_filters() {
        let q = QueryParser::parse(
            "ownership language:rust tag:concurrency project:backend extension:rs type:code",
        );
        assert_eq!(q.free_terms, vec!["ownership"]);
        assert_eq!(q.filters.get(&FilterKey::Language).unwrap(), "rust");
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "concurrency");
        assert_eq!(q.filters.get(&FilterKey::Project).unwrap(), "backend");
        assert_eq!(q.filters.get(&FilterKey::Extension).unwrap(), "rs");
        assert_eq!(q.filters.get(&FilterKey::Type).unwrap(), "code");
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_quoted_filter_value() {
        let q = QueryParser::parse(r#"project:"backend service" tag:"web api""#);
        assert_eq!(
            q.filters.get(&FilterKey::Project).unwrap(),
            "backend service"
        );
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "web api");
        assert!(q.free_terms.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_multiple_filter_occurrences_joined() {
        let q = QueryParser::parse("tag:rust tag:cli");
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "rust,cli");
    }

    #[test]
    fn test_parse_unknown_filter_warning() {
        let q = QueryParser::parse("ownership unknown:value");
        assert_eq!(q.free_terms, vec!["ownership"]);
        assert!(q.filters.is_empty());
        assert_eq!(
            q.warnings,
            vec!["Filter 'unknown' tidak dikenali dan diabaikan."]
        );
    }

    #[test]
    fn test_parse_empty_filter_value_warning() {
        let q = QueryParser::parse("language: ownership");
        assert_eq!(q.free_terms, vec!["ownership"]);
        assert!(q.filters.is_empty());
        assert_eq!(
            q.warnings,
            vec!["Filter 'language' memiliki nilai kosong dan diabaikan."]
        );
    }

    #[test]
    fn test_parse_urls_and_code_identifiers_not_filters() {
        let q = QueryParser::parse("http://127.0.0.1:3001 std::sync::Arc");
        assert_eq!(
            q.free_terms,
            vec!["http://127.0.0.1:3001", "std::sync::Arc"]
        );
        assert!(q.filters.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_special_characters_safety() {
        let q = QueryParser::parse("C++ C# !term +add -del (group) [bracket] ^boost");
        assert_eq!(
            q.free_terms,
            vec![
                "C++",
                "C#",
                "!term",
                "+add",
                "-del",
                "(group)",
                "[bracket]",
                "^boost"
            ]
        );
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_architecture_spec_example() {
        let q = QueryParser::parse(
            r#""tokio runtime" language:rust tag:concurrency project:backend type:code ownership"#,
        );
        assert_eq!(q.free_terms, vec!["ownership"]);
        assert_eq!(q.phrase_terms, vec!["tokio runtime"]);
        assert_eq!(q.filters.get(&FilterKey::Language).unwrap(), "rust");
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "concurrency");
        assert_eq!(q.filters.get(&FilterKey::Project).unwrap(), "backend");
        assert_eq!(q.filters.get(&FilterKey::Type).unwrap(), "code");
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_case_insensitivity_keys_and_values() {
        let q = QueryParser::parse("LANGUAGE:Rust EXT:RS TYPE:CODE tags:concurrency");
        assert_eq!(q.filters.get(&FilterKey::Language).unwrap(), "rust");
        assert_eq!(q.filters.get(&FilterKey::Extension).unwrap(), "rs");
        assert_eq!(q.filters.get(&FilterKey::Type).unwrap(), "code");
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "concurrency");
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_extension_with_dot_stripped() {
        let q = QueryParser::parse("extension:.rs ext:.ts");
        assert_eq!(q.filters.get(&FilterKey::Extension).unwrap(), "rs,ts");
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_unclosed_filter_quote_malformed_warning() {
        let q = QueryParser::parse(r#"project:"backend service"#);
        assert_eq!(
            q.warnings,
            vec!["Filter 'project' memiliki tanda kutip yang tidak tertutup dan diabaikan."]
        );
        assert!(q.filters.is_empty());
        assert_eq!(q.free_terms, vec![r#"project:"backend"#, "service"]);
    }

    #[test]
    fn test_parse_empty_quotes_in_filter() {
        let q = QueryParser::parse(r#"language:"" tag:"   ""#);
        assert_eq!(
            q.warnings,
            vec![
                "Filter 'language' memiliki nilai kosong dan diabaikan.",
                "Filter 'tag' memiliki nilai kosong dan diabaikan."
            ]
        );
        assert!(q.filters.is_empty());
    }

    #[test]
    fn test_parse_invalid_type_filter_warning() {
        let q = QueryParser::parse("type:invalid_type type:code");
        assert_eq!(
            q.warnings,
            vec!["Filter type 'invalid_type' tidak dikenali dan diabaikan."]
        );
        assert_eq!(q.filters.get(&FilterKey::Type).unwrap(), "code");
    }

    #[test]
    fn test_parse_duplicate_filter_value_deduplication() {
        let q = QueryParser::parse("tag:rust tag:cli tag:rust");
        assert_eq!(q.filters.get(&FilterKey::Tag).unwrap(), "rust,cli");
    }

    #[test]
    fn test_parse_numeric_colons_and_ports_as_free_terms() {
        let q = QueryParser::parse("12:00 16:9 8080:tcp");
        assert_eq!(q.free_terms, vec!["12:00", "16:9", "8080:tcp"]);
        assert!(q.filters.is_empty());
        assert!(q.warnings.is_empty());
    }

    #[test]
    fn test_parse_invalid_language_filter_warning() {
        let q = QueryParser::parse("language:bogus_lang language:rust");
        assert_eq!(
            q.warnings,
            vec!["Filter language 'bogus_lang' tidak dikenali dan diabaikan."]
        );
        assert_eq!(q.filters.get(&FilterKey::Language).unwrap(), "rust");
    }

    #[test]
    fn test_parse_warning_order_deterministic() {
        let q = QueryParser::parse("foo:1 bar:2 baz:");
        assert_eq!(
            q.warnings,
            vec![
                "Filter 'foo' tidak dikenali dan diabaikan.",
                "Filter 'bar' tidak dikenali dan diabaikan.",
                "Filter 'baz' memiliki nilai kosong dan diabaikan."
            ]
        );
    }
}
