use std::time::Duration;

use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::infrastructure::elasticsearch::{
    create_es_client_with_timeout, document_index_schema_with_alias, ping_elasticsearch,
};
use elasticsearch::indices::{IndicesAnalyzeParts, IndicesCreateParts, IndicesDeleteParts};
use serde_json::json;

#[tokio::test]
async fn test_custom_analyzer_baseline_on_live_elasticsearch() {
    let client =
        match create_es_client_with_timeout(DEFAULT_ELASTICSEARCH_URL, Duration::from_secs(15)) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping test: cannot initialize Elasticsearch client: {e}");
                return;
            }
        };

    if ping_elasticsearch(&client).await.is_err() {
        eprintln!(
            "Elasticsearch not reachable at {DEFAULT_ELASTICSEARCH_URL}, skipping live integration test"
        );
        return;
    }

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_test_alias_{test_suffix}");
    let test_index = format!("lynx_test_index_{test_suffix}");

    // 1. Verify index creation succeeds against ES 8.19.22
    let schema_payload = document_index_schema_with_alias(&test_alias);
    let create_res = client
        .indices()
        .create(IndicesCreateParts::Index(&test_index))
        .body(schema_payload)
        .send()
        .await
        .expect("Failed to send index create request");

    let create_status = create_res.status_code();
    let create_body = create_res.text().await.unwrap_or_default();
    assert!(
        create_status.is_success(),
        "Index creation failed with status {create_status}: {create_body}"
    );

    // 2. Test code_analyzer with camelCase
    let camel_analyze_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "authenticateUser"
        }))
        .send()
        .await
        .expect("Failed to analyze camelCase text");

    assert!(camel_analyze_res.status_code().is_success());
    let camel_json: serde_json::Value = camel_analyze_res
        .json()
        .await
        .expect("Failed to parse camelCase analyze response");

    let camel_tokens: Vec<String> = camel_json["tokens"]
        .as_array()
        .expect("tokens array")
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(
        camel_tokens.contains(&"authenticateuser".to_string()),
        "code_analyzer must preserve original token: {camel_tokens:?}"
    );
    assert!(
        camel_tokens.contains(&"authenticate".to_string()),
        "code_analyzer must split camelCase subword: {camel_tokens:?}"
    );
    assert!(
        camel_tokens.contains(&"user".to_string()),
        "code_analyzer must split camelCase subword: {camel_tokens:?}"
    );

    // 3. Test code_analyzer with snake_case
    let snake_analyze_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "authenticate_user"
        }))
        .send()
        .await
        .expect("Failed to analyze snake_case text");

    assert!(snake_analyze_res.status_code().is_success());
    let snake_json: serde_json::Value = snake_analyze_res
        .json()
        .await
        .expect("Failed to parse snake_case analyze response");

    let snake_tokens: Vec<String> = snake_json["tokens"]
        .as_array()
        .expect("tokens array")
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(
        snake_tokens.contains(&"authenticate_user".to_string()),
        "code_analyzer must preserve original snake_case token: {snake_tokens:?}"
    );
    assert!(
        snake_tokens.contains(&"authenticate".to_string()),
        "code_analyzer must split snake_case subword: {snake_tokens:?}"
    );
    assert!(
        snake_tokens.contains(&"user".to_string()),
        "code_analyzer must split snake_case subword: {snake_tokens:?}"
    );

    // 4. Test code_analyzer with alphanumeric splitting
    let alpha_analyze_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "XL500"
        }))
        .send()
        .await
        .expect("Failed to analyze alphanumeric text");

    assert!(alpha_analyze_res.status_code().is_success());
    let alpha_json: serde_json::Value = alpha_analyze_res
        .json()
        .await
        .expect("Failed to parse alphanumeric analyze response");

    let alpha_tokens: Vec<String> = alpha_json["tokens"]
        .as_array()
        .expect("tokens array")
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(
        alpha_tokens.contains(&"xl500".to_string()),
        "code_analyzer must preserve original alphanumeric token: {alpha_tokens:?}"
    );
    assert!(
        alpha_tokens.contains(&"xl".to_string()),
        "code_analyzer must split alpha part: {alpha_tokens:?}"
    );
    assert!(
        alpha_tokens.contains(&"500".to_string()),
        "code_analyzer must split numeric part: {alpha_tokens:?}"
    );

    // 5. Test autocomplete_analyzer with edge_ngram
    let auto_analyze_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "autocomplete_analyzer",
            "text": "Search"
        }))
        .send()
        .await
        .expect("Failed to analyze autocomplete text");

    assert!(auto_analyze_res.status_code().is_success());
    let auto_json: serde_json::Value = auto_analyze_res
        .json()
        .await
        .expect("Failed to parse autocomplete analyze response");

    let auto_tokens: Vec<String> = auto_json["tokens"]
        .as_array()
        .expect("tokens array")
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert_eq!(
        auto_tokens,
        vec!["se", "sea", "sear", "searc", "search"],
        "autocomplete_analyzer must generate lowercase edge-ngrams from 2 to 20"
    );

    // 6. Cleanup test index
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
