use serde_json::{Value, json};

pub mod fields {
    pub const ID: &str = "id";
    pub const FOLDER_ID: &str = "folder_id";
    pub const RELATIVE_PATH: &str = "relative_path";
    pub const ABSOLUTE_PATH: &str = "absolute_path";
    pub const TITLE: &str = "title";
    pub const TITLE_CODE: &str = "title.code";
    pub const TITLE_SUGGEST: &str = "title.suggest";
    pub const CONTENT: &str = "content";
    pub const CONTENT_CODE: &str = "content.code";
    pub const TAGS: &str = "tags";
    pub const EXTENSION: &str = "extension";
    pub const LANGUAGE: &str = "language";
    pub const TYPE: &str = "type";
    pub const PROJECT: &str = "project";
    pub const FILE_SIZE_BYTES: &str = "file_size_bytes";
    pub const MODIFIED_AT: &str = "modified_at";
    pub const INDEXED_AT: &str = "indexed_at";
}

pub mod analysis {
    pub const CODE_ANALYZER: &str = "code_analyzer";
    pub const AUTOCOMPLETE_ANALYZER: &str = "autocomplete_analyzer";
    pub const CODE_SUBWORD_FILTER: &str = "code_subword_filter";
    pub const AUTOCOMPLETE_FILTER: &str = "autocomplete_filter";
}

pub const KEYWORD_FIELDS: &[&str] = &[
    fields::ID,
    fields::FOLDER_ID,
    fields::RELATIVE_PATH,
    fields::TAGS,
    fields::EXTENSION,
    fields::LANGUAGE,
    fields::TYPE,
    fields::PROJECT,
];

pub const ALL_FIELDS: &[&str] = &[
    fields::ID,
    fields::FOLDER_ID,
    fields::RELATIVE_PATH,
    fields::ABSOLUTE_PATH,
    fields::TITLE,
    fields::CONTENT,
    fields::TAGS,
    fields::EXTENSION,
    fields::LANGUAGE,
    fields::TYPE,
    fields::PROJECT,
    fields::FILE_SIZE_BYTES,
    fields::MODIFIED_AT,
    fields::INDEXED_AT,
];

/// Returns true if the field is mapped as a keyword in Elasticsearch.
pub fn is_keyword_field(field: &str) -> bool {
    KEYWORD_FIELDS.contains(&field)
}

/// Returns the complete mappings definition for the `lynx_documents` index.
pub fn document_mappings() -> Value {
    json!({
        "properties": {
            "id": { "type": "keyword" },
            "folder_id": { "type": "keyword" },
            "relative_path": { "type": "keyword" },
            "absolute_path": { "type": "keyword", "index": false },
            "title": {
                "type": "text",
                "analyzer": "standard",
                "fields": {
                    "code": { "type": "text", "analyzer": "code_analyzer" },
                    "suggest": {
                        "type": "text",
                        "analyzer": "autocomplete_analyzer",
                        "search_analyzer": "standard"
                    }
                }
            },
            "content": {
                "type": "text",
                "analyzer": "standard",
                "fields": {
                    "code": { "type": "text", "analyzer": "code_analyzer" }
                }
            },
            "tags": { "type": "keyword" },
            "extension": { "type": "keyword" },
            "language": { "type": "keyword" },
            "type": { "type": "keyword" },
            "project": { "type": "keyword" },
            "file_size_bytes": { "type": "long" },
            "modified_at": { "type": "date" },
            "indexed_at": { "type": "date" }
        }
    })
}

/// Returns the index settings with 1 shard, 0 replicas, and code/autocomplete analyzers.
pub fn document_index_settings() -> Value {
    json!({
        "number_of_shards": 1,
        "number_of_replicas": 0,
        "analysis": {
            "filter": {
                "code_subword_filter": {
                    "type": "word_delimiter_graph",
                    "generate_word_parts": true,
                    "generate_number_parts": true,
                    "catenate_words": false,
                    "split_on_case_change": true,
                    "split_on_numerics": true,
                    "preserve_original": true
                },
                "autocomplete_filter": {
                    "type": "edge_ngram",
                    "min_gram": 2,
                    "max_gram": 20
                }
            },
            "analyzer": {
                "code_analyzer": {
                    "type": "custom",
                    "tokenizer": "whitespace",
                    "filter": [
                        "code_subword_filter",
                        "lowercase",
                        "flatten_graph"
                    ]
                },
                "autocomplete_analyzer": {
                    "type": "custom",
                    "tokenizer": "standard",
                    "filter": [
                        "lowercase",
                        "autocomplete_filter"
                    ]
                }
            }
        }
    })
}

/// Returns the complete schema for index creation payload containing both settings and mappings.
pub fn document_index_schema() -> Value {
    json!({
        "settings": document_index_settings(),
        "mappings": document_mappings()
    })
}

/// Returns the complete schema for index creation payload including an initial alias.
pub fn document_index_schema_with_alias(alias: &str) -> Value {
    json!({
        "settings": document_index_settings(),
        "mappings": document_mappings(),
        "aliases": {
            alias: {}
        }
    })
}

/// Returns the physical versioned index name for a given alias and numeric version.
/// E.g. ("lynx_documents", 1) -> "lynx_documents_v1"
pub fn physical_index_name(alias: &str, version: u32) -> String {
    format!("{alias}_v{version}")
}

/// Parses the numeric version from a physical index name matching `{alias}_v{version}`.
/// Returns `None` if the index name does not match the versioned format.
pub fn parse_index_version(index_name: &str, alias: &str) -> Option<u32> {
    let prefix = format!("{alias}_v");
    index_name
        .strip_prefix(&prefix)
        .and_then(|suffix| suffix.parse::<u32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_14_fields_present_in_mappings() {
        let mappings = document_mappings();
        let properties = mappings
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("properties object must exist");

        assert_eq!(
            properties.len(),
            14,
            "Must define exactly 14 top-level fields"
        );
        for field in ALL_FIELDS {
            assert!(
                properties.contains_key(*field),
                "Missing mapping property: {field}"
            );
        }
    }

    #[test]
    fn test_absolute_path_mapping_is_not_indexed() {
        let mappings = document_mappings();
        let abs_path = &mappings["properties"]["absolute_path"];
        assert_eq!(abs_path["type"], "keyword");
        assert_eq!(abs_path["index"], false);
    }

    #[test]
    fn test_title_analyzers_and_multifields() {
        let mappings = document_mappings();
        let title = &mappings["properties"]["title"];
        assert_eq!(title["type"], "text");
        assert_eq!(title["analyzer"], "standard");

        let code_field = &title["fields"]["code"];
        assert_eq!(code_field["type"], "text");
        assert_eq!(code_field["analyzer"], "code_analyzer");

        let suggest_field = &title["fields"]["suggest"];
        assert_eq!(suggest_field["type"], "text");
        assert_eq!(suggest_field["analyzer"], "autocomplete_analyzer");
        assert_eq!(suggest_field["search_analyzer"], "standard");
    }

    #[test]
    fn test_content_analyzers_and_multifields() {
        let mappings = document_mappings();
        let content = &mappings["properties"]["content"];
        assert_eq!(content["type"], "text");
        assert_eq!(content["analyzer"], "standard");

        let code_field = &content["fields"]["code"];
        assert_eq!(code_field["type"], "text");
        assert_eq!(code_field["analyzer"], "code_analyzer");
    }

    #[test]
    fn test_keyword_fields_types() {
        let mappings = document_mappings();
        let properties = &mappings["properties"];

        for field in KEYWORD_FIELDS {
            assert_eq!(
                properties[*field]["type"], "keyword",
                "Field {field} must be keyword type"
            );
            assert!(is_keyword_field(field));
        }

        assert!(!is_keyword_field(fields::TITLE));
        assert!(!is_keyword_field(fields::CONTENT));
        assert!(!is_keyword_field(fields::FILE_SIZE_BYTES));
    }

    #[test]
    fn test_numeric_and_date_field_types() {
        let mappings = document_mappings();
        let properties = &mappings["properties"];

        assert_eq!(properties["file_size_bytes"]["type"], "long");
        assert_eq!(properties["modified_at"]["type"], "date");
        assert_eq!(properties["indexed_at"]["type"], "date");
    }

    #[test]
    fn test_index_settings_and_analysis() {
        let settings = document_index_settings();
        assert_eq!(settings["number_of_shards"], 1);
        assert_eq!(settings["number_of_replicas"], 0);

        let analysis = &settings["analysis"];
        let subword_filter = &analysis["filter"]["code_subword_filter"];
        assert!(subword_filter.is_object());
        assert_eq!(subword_filter["type"], "word_delimiter_graph");
        assert_eq!(subword_filter["generate_word_parts"], true);
        assert_eq!(subword_filter["generate_number_parts"], true);
        assert_eq!(subword_filter["catenate_words"], false);
        assert_eq!(subword_filter["split_on_case_change"], true);
        assert_eq!(subword_filter["split_on_numerics"], true);
        assert_eq!(subword_filter["preserve_original"], true);

        let auto_filter = &analysis["filter"]["autocomplete_filter"];
        assert!(auto_filter.is_object());
        assert_eq!(auto_filter["type"], "edge_ngram");
        assert_eq!(auto_filter["min_gram"], 2);
        assert_eq!(auto_filter["max_gram"], 20);

        let code_analyzer = &analysis["analyzer"]["code_analyzer"];
        assert_eq!(code_analyzer["type"], "custom");
        assert_eq!(code_analyzer["tokenizer"], "whitespace");
        assert_eq!(
            code_analyzer["filter"],
            serde_json::json!(["code_subword_filter", "lowercase", "flatten_graph"])
        );

        let auto_analyzer = &analysis["analyzer"]["autocomplete_analyzer"];
        assert_eq!(auto_analyzer["type"], "custom");
        assert_eq!(auto_analyzer["tokenizer"], "standard");
        assert_eq!(
            auto_analyzer["filter"],
            serde_json::json!(["lowercase", "autocomplete_filter"])
        );
    }

    #[test]
    fn test_document_index_schema_payload_structure() {
        let schema = document_index_schema();
        assert!(schema.get("settings").is_some());
        assert!(schema.get("mappings").is_some());
        assert!(schema["mappings"].get("properties").is_some());
    }

    #[test]
    fn test_document_index_schema_with_alias() {
        let schema = document_index_schema_with_alias("lynx_documents");
        assert!(schema.get("settings").is_some());
        assert!(schema.get("mappings").is_some());
        let aliases = schema
            .get("aliases")
            .and_then(|a| a.as_object())
            .expect("aliases object must exist");
        assert!(aliases.contains_key("lynx_documents"));
    }

    #[test]
    fn test_physical_index_naming_and_version_parsing() {
        assert_eq!(
            physical_index_name("lynx_documents", 1),
            "lynx_documents_v1"
        );
        assert_eq!(
            physical_index_name("lynx_documents", 2),
            "lynx_documents_v2"
        );
        assert_eq!(physical_index_name("custom_alias", 10), "custom_alias_v10");

        assert_eq!(
            parse_index_version("lynx_documents_v1", "lynx_documents"),
            Some(1)
        );
        assert_eq!(
            parse_index_version("lynx_documents_v42", "lynx_documents"),
            Some(42)
        );
        assert_eq!(
            parse_index_version("lynx_documents_v0", "lynx_documents"),
            Some(0)
        );
        assert_eq!(
            parse_index_version("lynx_documents_vinvalid", "lynx_documents"),
            None
        );
        assert_eq!(
            parse_index_version("lynx_documents", "lynx_documents"),
            None
        );
        assert_eq!(
            parse_index_version("other_alias_v1", "lynx_documents"),
            None
        );
    }
}
