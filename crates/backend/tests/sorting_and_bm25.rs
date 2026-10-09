use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::config::Bm25Weights;
use backend::domain::models::AppSettings;
use backend::domain::query_parser::QueryParser;
use backend::domain::services::search_query_builder::SearchQueryBuilder;
use serde_json::json;
use tower::ServiceExt;

#[test]
fn test_sorting_options_elasticsearch_dsl() {
    let query = QueryParser::parse("rust");

    // 1. relevance default -> no sort clause (ES defaults to _score desc)
    let b_relevance = SearchQueryBuilder::new(query.clone()).sort(Some("relevance".into()));
    assert!(b_relevance.build()["sort"].is_null());

    let b_none = SearchQueryBuilder::new(query.clone()).sort(None);
    assert!(b_none.build()["sort"].is_null());

    // 2. modified_desc & modified_at alias
    let b_mod_desc = SearchQueryBuilder::new(query.clone()).sort(Some("modified_desc".into()));
    assert_eq!(
        b_mod_desc.build()["sort"],
        json!([
            { "modified_at": { "order": "desc", "missing": "_last" } },
            "_score"
        ])
    );

    let b_mod_at = SearchQueryBuilder::new(query.clone()).sort(Some("modified_at".into()));
    assert_eq!(
        b_mod_at.build()["sort"],
        json!([
            { "modified_at": { "order": "desc", "missing": "_last" } },
            "_score"
        ])
    );

    // 3. modified_asc
    let b_mod_asc = SearchQueryBuilder::new(query.clone()).sort(Some("modified_asc".into()));
    assert_eq!(
        b_mod_asc.build()["sort"],
        json!([
            { "modified_at": { "order": "asc", "missing": "_last" } },
            "_score"
        ])
    );

    // 4. name_asc & name alias (using relative_path keyword)
    let b_name_asc = SearchQueryBuilder::new(query.clone()).sort(Some("name_asc".into()));
    assert_eq!(
        b_name_asc.build()["sort"],
        json!([
            { "relative_path": { "order": "asc", "missing": "_last" } },
            "_score"
        ])
    );

    let b_name = SearchQueryBuilder::new(query.clone()).sort(Some("name".into()));
    assert_eq!(
        b_name.build()["sort"],
        json!([
            { "relative_path": { "order": "asc", "missing": "_last" } },
            "_score"
        ])
    );

    // 5. name_desc
    let b_name_desc = SearchQueryBuilder::new(query.clone()).sort(Some("name_desc".into()));
    assert_eq!(
        b_name_desc.build()["sort"],
        json!([
            { "relative_path": { "order": "desc", "missing": "_last" } },
            "_score"
        ])
    );

    // 6. size_desc & size_asc
    let b_size_desc = SearchQueryBuilder::new(query.clone()).sort(Some("size_desc".into()));
    assert_eq!(
        b_size_desc.build()["sort"],
        json!([
            { "file_size_bytes": { "order": "desc", "missing": "_last" } },
            "_score"
        ])
    );

    let b_size_asc = SearchQueryBuilder::new(query).sort(Some("size_asc".into()));
    assert_eq!(
        b_size_asc.build()["sort"],
        json!([
            { "file_size_bytes": { "order": "asc", "missing": "_last" } },
            "_score"
        ])
    );
}

#[test]
fn test_pagination_with_sort_consistency() {
    let query = QueryParser::parse("search");

    let page1 = SearchQueryBuilder::new(query.clone())
        .page(1)
        .per_page(20)
        .sort(Some("name_asc".into()))
        .build();

    let page2 = SearchQueryBuilder::new(query)
        .page(2)
        .per_page(20)
        .sort(Some("name_asc".into()))
        .build();

    assert_eq!(page1["from"], 0);
    assert_eq!(page1["size"], 20);
    assert_eq!(page2["from"], 20);
    assert_eq!(page2["size"], 20);

    // Sort order must remain identical between pages
    assert_eq!(page1["sort"], page2["sort"]);
}

#[test]
fn test_bm25_weights_runtime_application() {
    let query = QueryParser::parse("indexing");

    let custom_weights = Bm25Weights {
        title: 10.0,
        tags: 5.0,
        content: 2.0,
    };

    let builder = SearchQueryBuilder::new(query)
        .weights(custom_weights)
        .query_code_subfields(true);

    let fields = builder.build_search_fields();
    assert_eq!(
        fields,
        vec![
            "title^10.0",
            "tags^5.0",
            "content^2.0",
            "title.code^10.0",
            "content.code^2.0"
        ]
    );

    let dsl = builder.build();
    let query_clause = &dsl["query"];
    let clause_str = serde_json::to_string(query_clause).unwrap();
    assert!(clause_str.contains("title^10.0"));
    assert!(clause_str.contains("tags^5.0"));
    assert!(clause_str.contains("content^2.0"));
}

#[tokio::test]
async fn test_settings_api_update_persistence_and_reset() {
    let (state, _rx) = backend::AppState::test_state();
    let app = backend::create_router_with_state(state.clone());

    // 1. Initial GET /api/settings has default weights
    let req = Request::builder()
        .uri("/api/settings")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let settings: AppSettings = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(settings.weights.title, 3.0);
    assert_eq!(settings.weights.tags, 2.0);
    assert_eq!(settings.weights.content, 1.0);

    // 2. PUT /api/settings with valid updated weights
    let update_body = json!({
        "weights": {
            "title": 6.5,
            "tags": 3.5,
            "content": 1.2
        }
    });
    let req = Request::builder()
        .uri("/api/settings")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify state reflects updated weights
    let in_memory = state.get_settings().await;
    assert_eq!(in_memory.weights.title, 6.5);
    assert_eq!(in_memory.weights.tags, 3.5);
    assert_eq!(in_memory.weights.content, 1.2);

    // 3. PUT /api/settings with invalid negative or zero weight is rejected
    let invalid_body = json!({
        "weights": {
            "title": 0.0
        }
    });
    let req = Request::builder()
        .uri("/api/settings")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let negative_body = json!({
        "weights": {
            "content": -1.0
        }
    });
    let req = Request::builder()
        .uri("/api/settings")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&negative_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 4. POST /api/settings/reset resets to default
    let req = Request::builder()
        .uri("/api/settings/reset")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let reset_settings: AppSettings = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reset_settings.weights.title, 3.0);
    assert_eq!(reset_settings.weights.tags, 2.0);
    assert_eq!(reset_settings.weights.content, 1.0);
}
