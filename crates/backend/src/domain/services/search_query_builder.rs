use crate::config::Bm25Weights;
use crate::domain::models::search::SearchQuery;
use crate::domain::models::types::FilterKey;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_PAGE: usize = 1;
pub const DEFAULT_PER_PAGE: usize = 20;
pub const MAX_PER_PAGE: usize = 100;

pub const DEFAULT_SOURCE_FIELDS: &[&str] = &[
    "id",
    "folder_id",
    "relative_path",
    "absolute_path",
    "title",
    "content",
    "tags",
    "extension",
    "language",
    "type",
    "project",
    "file_size_bytes",
    "modified_at",
    "indexed_at",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightConfig {
    pub pre_tags: Vec<String>,
    pub post_tags: Vec<String>,
    pub fragment_size: u32,
    pub number_of_fragments: u32,
    pub require_field_match: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_chars: Option<String>,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            pre_tags: vec!["<em>".to_string()],
            post_tags: vec!["</em>".to_string()],
            fragment_size: 150,
            number_of_fragments: 3,
            require_field_match: false,
            order: Some("score".to_string()),
            boundary_chars: None,
        }
    }
}

impl HighlightConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fragments(mut self, count: u32, size: u32) -> Self {
        self.number_of_fragments = count;
        self.fragment_size = size;
        self
    }

    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    pub fn with_boundary_chars(mut self, boundary_chars: impl Into<String>) -> Self {
        self.boundary_chars = Some(boundary_chars.into());
        self
    }
}

/// Pure domain query builder converting `SearchQuery` AST and search options
/// into an Elasticsearch 8.x Query DSL payload.
#[derive(Debug, Clone)]
pub struct SearchQueryBuilder {
    query: SearchQuery,
    page: usize,
    per_page: usize,
    weights: Bm25Weights,
    query_code_subfields: bool,
    track_total_hits: bool,
    track_scores: bool,
    source_fields: Vec<String>,
    highlight: Option<HighlightConfig>,
    sort: Option<String>,
}

impl SearchQueryBuilder {
    pub fn new(query: SearchQuery) -> Self {
        Self {
            query,
            page: DEFAULT_PAGE,
            per_page: DEFAULT_PER_PAGE,
            weights: Bm25Weights::default(),
            query_code_subfields: true,
            track_total_hits: true,
            track_scores: true,
            source_fields: DEFAULT_SOURCE_FIELDS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            highlight: Some(HighlightConfig::default()),
            sort: None,
        }
    }

    pub fn page(mut self, page: usize) -> Self {
        self.page = page.max(1);
        self
    }

    pub fn per_page(mut self, per_page: usize) -> Self {
        self.per_page = per_page.clamp(1, MAX_PER_PAGE);
        self
    }

    pub fn weights(mut self, weights: Bm25Weights) -> Self {
        self.weights = weights;
        self
    }

    pub fn query_code_subfields(mut self, enabled: bool) -> Self {
        self.query_code_subfields = enabled;
        self
    }

    pub fn track_total_hits(mut self, enabled: bool) -> Self {
        self.track_total_hits = enabled;
        self
    }

    pub fn track_scores(mut self, enabled: bool) -> Self {
        self.track_scores = enabled;
        self
    }

    pub fn source_fields(mut self, fields: Vec<String>) -> Self {
        self.source_fields = fields;
        self
    }

    pub fn highlight(mut self, config: Option<HighlightConfig>) -> Self {
        self.highlight = config;
        self
    }

    pub fn disable_highlight(mut self) -> Self {
        self.highlight = None;
        self
    }

    pub fn highlight_config(&self) -> Option<&HighlightConfig> {
        self.highlight.as_ref()
    }

    pub fn sort(mut self, sort: Option<String>) -> Self {
        self.sort = sort;
        self
    }

    pub fn sort_option(&self) -> Option<&str> {
        self.sort.as_deref()
    }

    pub fn query(&self) -> &SearchQuery {
        &self.query
    }

    pub fn offset(&self) -> usize {
        (self.page.max(1) - 1) * self.per_page
    }

    pub fn size(&self) -> usize {
        self.per_page
    }

    fn format_boost(field: &str, weight: f32) -> String {
        if (weight.fract()).abs() < 1e-6 {
            format!("{field}^{:.1}", weight)
        } else {
            format!("{field}^{weight}")
        }
    }

    pub fn build_search_fields(&self) -> Vec<String> {
        let mut fields = vec![
            Self::format_boost("title", self.weights.title),
            Self::format_boost("tags", self.weights.tags),
            Self::format_boost("content", self.weights.content),
        ];

        if self.query_code_subfields {
            fields.push(Self::format_boost("title.code", self.weights.title));
            fields.push(Self::format_boost("content.code", self.weights.content));
        }

        fields
    }

    fn build_query_clause(&self) -> Value {
        let fields = self.build_search_fields();
        let mut must_clauses = Vec::new();

        // 1. Free terms: multi_match with best_fields
        if !self.query.free_terms.is_empty() {
            let free_text = self.query.free_terms.join(" ");
            must_clauses.push(json!({
                "multi_match": {
                    "query": free_text,
                    "fields": fields.clone(),
                    "type": "best_fields"
                }
            }));
        }

        // 2. Phrase terms: multi_match with type phrase for each quoted phrase
        for phrase in &self.query.phrase_terms {
            must_clauses.push(json!({
                "multi_match": {
                    "query": phrase,
                    "fields": fields.clone(),
                    "type": "phrase"
                }
            }));
        }

        // 3. Filters in bool.filter (separate from scoring)
        let mut filter_clauses = Vec::new();
        let mut sorted_filters: Vec<(&FilterKey, &String)> = self.query.filters.iter().collect();
        sorted_filters.sort_by_key(|(k, _)| k.as_str());

        for (key, val) in sorted_filters {
            let field_name = match key {
                FilterKey::Tag => "tags",
                FilterKey::Language => "language",
                FilterKey::Project => "project",
                FilterKey::Extension => "extension",
                FilterKey::Type => "type",
            };
            filter_clauses.push(json!({
                "term": {
                    field_name: val
                }
            }));
        }

        if must_clauses.is_empty() && filter_clauses.is_empty() {
            json!({ "match_all": {} })
        } else {
            let mut bool_body = serde_json::Map::new();
            if !must_clauses.is_empty() {
                bool_body.insert("must".to_string(), Value::Array(must_clauses));
            }
            if !filter_clauses.is_empty() {
                bool_body.insert("filter".to_string(), Value::Array(filter_clauses));
            }
            json!({ "bool": Value::Object(bool_body) })
        }
    }

    fn build_highlight_clause(&self) -> Option<Value> {
        let config = self.highlight.as_ref()?;

        let mut content_field = serde_json::Map::new();
        content_field.insert("fragment_size".to_string(), json!(config.fragment_size));
        content_field.insert(
            "number_of_fragments".to_string(),
            json!(config.number_of_fragments),
        );

        if let Some(order) = &config.order {
            content_field.insert("order".to_string(), json!(order));
        }

        if let Some(boundary_chars) = &config.boundary_chars {
            content_field.insert("boundary_chars".to_string(), json!(boundary_chars));
        }

        Some(json!({
            "pre_tags": config.pre_tags,
            "post_tags": config.post_tags,
            "require_field_match": config.require_field_match,
            "fields": {
                "title": {
                    "number_of_fragments": 0
                },
                "content": Value::Object(content_field)
            }
        }))
    }

    pub fn build(&self) -> Value {
        let mut root = serde_json::Map::new();

        root.insert("from".to_string(), json!(self.offset()));
        root.insert("size".to_string(), json!(self.size()));
        root.insert("track_total_hits".to_string(), json!(self.track_total_hits));

        if self.track_scores {
            root.insert("track_scores".to_string(), json!(true));
        }

        root.insert("_source".to_string(), json!(self.source_fields));
        root.insert("query".to_string(), self.build_query_clause());

        if let Some(highlight) = self.build_highlight_clause() {
            root.insert("highlight".to_string(), highlight);
        }

        if let Some(sort_str) = &self.sort {
            let sort_clause = match sort_str.to_lowercase().as_str() {
                "modified_desc" => Some(json!([
                    { "modified_at": { "order": "desc", "missing": "_last" } },
                    "_score"
                ])),
                "modified_asc" => Some(json!([
                    { "modified_at": { "order": "asc", "missing": "_last" } },
                    "_score"
                ])),
                "size_desc" => Some(json!([
                    { "file_size_bytes": { "order": "desc", "missing": "_last" } },
                    "_score"
                ])),
                "size_asc" => Some(json!([
                    { "file_size_bytes": { "order": "asc", "missing": "_last" } },
                    "_score"
                ])),
                "relevance" => None,
                _ => None,
            };
            if let Some(clause) = sort_clause {
                root.insert("sort".to_string(), clause);
            }
        }

        Value::Object(root)
    }

    pub fn build_raw(&self) -> Result<Box<serde_json::value::RawValue>, serde_json::Error> {
        let val = self.build();
        let s = serde_json::to_string(&val)?;
        serde_json::value::RawValue::from_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::query_parser::QueryParser;

    #[test]
    fn test_empty_query_builds_match_all_with_defaults() {
        let query = SearchQuery::empty();
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        assert_eq!(dsl["from"], 0);
        assert_eq!(dsl["size"], 20);
        assert_eq!(dsl["track_total_hits"], true);
        assert_eq!(dsl["track_scores"], true);
        assert_eq!(dsl["query"]["match_all"], json!({}));

        let source = dsl["_source"].as_array().expect("_source array");
        assert_eq!(source.len(), DEFAULT_SOURCE_FIELDS.len());
        assert!(source.contains(&json!("title")));
        assert!(source.contains(&json!("content")));

        assert_eq!(dsl["highlight"]["pre_tags"], json!(["<em>"]));
        assert_eq!(dsl["highlight"]["post_tags"], json!(["</em>"]));
        assert_eq!(dsl["highlight"]["require_field_match"], false);
        assert_eq!(
            dsl["highlight"]["fields"]["title"]["number_of_fragments"],
            0
        );
        assert_eq!(
            dsl["highlight"]["fields"]["content"]["number_of_fragments"],
            3
        );
        assert_eq!(dsl["highlight"]["fields"]["content"]["fragment_size"], 150);
        assert_eq!(dsl["highlight"]["fields"]["content"]["order"], "score");
    }

    #[test]
    fn test_free_terms_multi_match_boosts_and_code_subfields() {
        let query = QueryParser::parse("rust ownership");
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        let must = dsl["query"]["bool"]["must"].as_array().expect("must array");
        assert_eq!(must.len(), 1);

        let mm = &must[0]["multi_match"];
        assert_eq!(mm["query"], "rust ownership");
        assert_eq!(mm["type"], "best_fields");

        let fields = mm["fields"].as_array().expect("fields array");
        assert_eq!(
            fields,
            &vec![
                json!("title^3.0"),
                json!("tags^2.0"),
                json!("content^1.0"),
                json!("title.code^3.0"),
                json!("content.code^1.0"),
            ]
        );
    }

    #[test]
    fn test_custom_bm25_weights_and_disable_code_subfields() {
        let query = QueryParser::parse("tokio");
        let custom_weights = Bm25Weights {
            title: 5.0,
            tags: 4.0,
            content: 2.0,
        };
        let builder = SearchQueryBuilder::new(query)
            .weights(custom_weights)
            .query_code_subfields(false);
        let dsl = builder.build();

        let mm = &dsl["query"]["bool"]["must"][0]["multi_match"];
        let fields = mm["fields"].as_array().expect("fields array");
        assert_eq!(
            fields,
            &vec![json!("title^5.0"), json!("tags^4.0"), json!("content^2.0"),]
        );
    }

    #[test]
    fn test_phrase_query_multi_match() {
        let query = QueryParser::parse("\"async await\"");
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        let must = dsl["query"]["bool"]["must"].as_array().expect("must array");
        assert_eq!(must.len(), 1);

        let mm = &must[0]["multi_match"];
        assert_eq!(mm["query"], "async await");
        assert_eq!(mm["type"], "phrase");
    }

    #[test]
    fn test_combined_free_terms_and_phrase_terms() {
        let query = QueryParser::parse("tokio \"bounded channel\"");
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        let must = dsl["query"]["bool"]["must"].as_array().expect("must array");
        assert_eq!(must.len(), 2);

        assert_eq!(must[0]["multi_match"]["query"], "tokio");
        assert_eq!(must[0]["multi_match"]["type"], "best_fields");

        assert_eq!(must[1]["multi_match"]["query"], "bounded channel");
        assert_eq!(must[1]["multi_match"]["type"], "phrase");
    }

    #[test]
    fn test_filter_separation_in_bool_filter() {
        let query =
            QueryParser::parse("language:rust tag:cli project:backend type:code extension:rs");
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        assert!(dsl["query"]["bool"]["must"].is_null());
        let filters = dsl["query"]["bool"]["filter"]
            .as_array()
            .expect("filter array");
        assert_eq!(filters.len(), 5);

        assert_eq!(filters[0]["term"]["extension"], "rs");
        assert_eq!(filters[1]["term"]["language"], "rust");
        assert_eq!(filters[2]["term"]["project"], "backend");
        assert_eq!(filters[3]["term"]["tags"], "cli");
        assert_eq!(filters[4]["term"]["type"], "code");
    }

    #[test]
    fn test_search_with_both_text_and_filters() {
        let query = QueryParser::parse("ownership language:rust tag:memory");
        let builder = SearchQueryBuilder::new(query);
        let dsl = builder.build();

        let must = dsl["query"]["bool"]["must"].as_array().expect("must array");
        assert_eq!(must.len(), 1);
        assert_eq!(must[0]["multi_match"]["query"], "ownership");

        let filter = dsl["query"]["bool"]["filter"]
            .as_array()
            .expect("filter array");
        assert_eq!(filter.len(), 2);
        assert_eq!(filter[0]["term"]["language"], "rust");
        assert_eq!(filter[1]["term"]["tags"], "memory");
    }

    #[test]
    fn test_pagination_and_clamping() {
        let query = SearchQuery::empty();
        let builder = SearchQueryBuilder::new(query).page(3).per_page(15);
        let dsl = builder.build();

        assert_eq!(dsl["from"], 30);
        assert_eq!(dsl["size"], 15);

        let clamped = SearchQueryBuilder::new(SearchQuery::empty())
            .page(0)
            .per_page(200);
        let clamped_dsl = clamped.build();
        assert_eq!(clamped_dsl["from"], 0);
        assert_eq!(clamped_dsl["size"], 100);
    }

    #[test]
    fn test_disable_highlight_and_custom_source_fields() {
        let query = QueryParser::parse("test");
        let builder = SearchQueryBuilder::new(query)
            .disable_highlight()
            .source_fields(vec!["id".into(), "title".into()]);
        let dsl = builder.build();

        assert!(dsl["highlight"].is_null());
        assert_eq!(dsl["_source"], json!(["id", "title"]));
    }

    #[test]
    fn test_build_raw_produces_valid_raw_value() {
        let query = QueryParser::parse("test query language:rust");
        let builder = SearchQueryBuilder::new(query);
        let raw = builder.build_raw().expect("Build RawValue");

        let parsed: Value = serde_json::from_str(raw.get()).expect("Parse back json");
        assert_eq!(parsed["from"], 0);
        assert_eq!(parsed["size"], 20);
        assert_eq!(parsed["track_total_hits"], true);
    }

    #[test]
    fn test_highlight_custom_fragments_order_and_boundary_chars() {
        let query = QueryParser::parse("ownership");
        let hl = HighlightConfig::new()
            .with_fragments(5, 200)
            .with_order("score")
            .with_boundary_chars(".,!? \n");
        let builder = SearchQueryBuilder::new(query).highlight(Some(hl));
        let dsl = builder.build();

        assert_eq!(
            dsl["highlight"]["fields"]["content"]["number_of_fragments"],
            5
        );
        assert_eq!(dsl["highlight"]["fields"]["content"]["fragment_size"], 200);
        assert_eq!(dsl["highlight"]["fields"]["content"]["order"], "score");
        assert_eq!(
            dsl["highlight"]["fields"]["content"]["boundary_chars"],
            ".,!? \n"
        );
    }

    #[test]
    fn test_search_query_builder_sort_options() {
        let query = QueryParser::parse("rust");

        let builder_relevance =
            SearchQueryBuilder::new(query.clone()).sort(Some("relevance".into()));
        assert!(builder_relevance.build()["sort"].is_null());

        let builder_modified_desc =
            SearchQueryBuilder::new(query.clone()).sort(Some("modified_desc".into()));
        assert_eq!(
            builder_modified_desc.build()["sort"],
            json!([
                { "modified_at": { "order": "desc", "missing": "_last" } },
                "_score"
            ])
        );

        let builder_size_asc = SearchQueryBuilder::new(query).sort(Some("size_asc".into()));
        assert_eq!(
            builder_size_asc.build()["sort"],
            json!([
                { "file_size_bytes": { "order": "asc", "missing": "_last" } },
                "_score"
            ])
        );
    }
}
