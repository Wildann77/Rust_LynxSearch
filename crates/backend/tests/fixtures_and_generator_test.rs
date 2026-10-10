mod common;

use backend::domain::models::{DocumentType, Language};
use backend::domain::ports::FileWalker;
use backend::domain::services::document_extractor::{
    DocumentExtractor, ExtractOptions, ExtractionResult, SkipReason,
};
use backend::infrastructure::fs::walker::LocalFileWalker;
use common::generator::FixtureTreeGenerator;
use std::path::Path;

#[test]
fn test_fixture_tree_10_4_contracts() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join("tests/fixtures/knowledge_base");

    assert!(fixture_root.exists(), "Fixture root directory must exist");

    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    // 1. notes/rust/ownership.md has front-matter tags rust, memory
    let ownership_path = fixture_root.join("notes/rust/ownership.md");
    let ownership_content = std::fs::read(&ownership_path).expect("read ownership.md");
    match extractor.extract(
        &ownership_path,
        &ownership_content,
        ownership_content.len() as u64,
        None,
        &opts,
    ) {
        ExtractionResult::Extracted(doc) => {
            assert_eq!(doc.doc_type, DocumentType::Doc);
            assert_eq!(doc.title, "Rust Ownership");
            assert!(doc.tags.contains(&"rust".to_string()));
            assert!(doc.tags.contains(&"memory".to_string()));
        }
        other => panic!("Expected Extracted for ownership.md, got {other:?}"),
    }

    // 2. src/auth/service.rs contains authenticateUser and validate_token
    let service_path = fixture_root.join("src/auth/service.rs");
    let service_content = std::fs::read(&service_path).expect("read service.rs");
    let service_str = String::from_utf8_lossy(&service_content);
    assert!(service_str.contains("authenticateUser"));
    assert!(service_str.contains("validate_token"));
    match extractor.extract(
        &service_path,
        &service_content,
        service_content.len() as u64,
        None,
        &opts,
    ) {
        ExtractionResult::Extracted(doc) => {
            assert_eq!(doc.doc_type, DocumentType::Code);
            assert_eq!(doc.language, Some(Language::Rust));
        }
        other => panic!("Expected Extracted for service.rs, got {other:?}"),
    }

    // 3. configs/app.toml is valid config
    let app_toml_path = fixture_root.join("configs/app.toml");
    let app_toml_content = std::fs::read(&app_toml_path).expect("read app.toml");
    let app_toml_str = String::from_utf8_lossy(&app_toml_content);
    assert!(app_toml_str.contains("[server]"));
    assert!(app_toml_str.contains("host = \"127.0.0.1\""));
    assert!(app_toml_str.contains("port = 3001"));
    match extractor.extract(
        &app_toml_path,
        &app_toml_content,
        app_toml_content.len() as u64,
        None,
        &opts,
    ) {
        ExtractionResult::Extracted(doc) => {
            assert_eq!(doc.doc_type, DocumentType::Config);
            assert_eq!(doc.language, Some(Language::Toml));
        }
        other => panic!("Expected Extracted for app.toml, got {other:?}"),
    }

    // 4. assets/sample.bin is detected as binary
    let bin_path = fixture_root.join("assets/sample.bin");
    let bin_content = std::fs::read(&bin_path).expect("read sample.bin");
    match extractor.extract(
        &bin_path,
        &bin_content,
        bin_content.len() as u64,
        None,
        &opts,
    ) {
        ExtractionResult::Skipped(SkipReason::BinaryDetected(_)) => {
            // Success
        }
        other => panic!("Expected Skipped(BinaryFile) for sample.bin, got {other:?}"),
    }

    // 5. .git/HEAD and node_modules/dummy.js are ignored by FileWalker
    let walker = LocalFileWalker::new();
    let discovered = walker
        .walk_all(
            &fixture_root,
            &backend::domain::ports::WalkOptions::default(),
        )
        .expect("Walk fixture folder");

    let rel_paths: Vec<String> = discovered
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(
        !rel_paths.iter().any(|p| p.starts_with(".git")),
        ".git files must be ignored"
    );
    assert!(
        !rel_paths.iter().any(|p| p.starts_with("node_modules")),
        "node_modules files must be ignored"
    );
    assert!(
        rel_paths.iter().any(|p| p.ends_with("ownership.md")),
        "ownership.md must be discovered"
    );
}

#[test]
fn test_dynamic_fixture_generator_10_5() {
    let generator = FixtureTreeGenerator::new(1000);
    let (temp_dir, created_files) = generator.generate();

    assert_eq!(created_files.len(), 1000);
    assert!(temp_dir.path().exists());

    // Verify first 5 files exist on disk with deterministic content
    for (i, path) in created_files.iter().take(5).enumerate() {
        assert!(path.exists(), "File {i} must exist");
        let content = std::fs::read_to_string(path).expect("read file");
        assert!(
            content.contains(&format!("{i}")),
            "Content must be deterministic containing index {i}"
        );
    }

    // RAII cleanup verified when temp_dir is dropped
    let path_copy = temp_dir.path().to_path_buf();
    drop(temp_dir);
    assert!(!path_copy.exists(), "TempDir must be cleaned up on drop");
}
