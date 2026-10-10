use crate::domain::models::document::IndexedDocument;
use crate::domain::models::id::{DocumentId, FolderId};
use crate::domain::models::types::{DocumentType, Language};
use chrono::{DateTime, Utc};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;
const BINARY_SCAN_BYTES_LIMIT: usize = 8192;
const MAGIC_BYTES_LIMIT: usize = 512;

const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bin", "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tiff",
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "wasm", "pdf", "iso", "dmg", "pkg", "deb", "rpm",
    "class", "pyc", "pyo", "o", "a", "obj", "lib", "mp3", "mp4", "mov", "avi", "mkv", "wav",
    "flac", "ogg", "m4a", "woff", "woff2", "ttf", "eot", "otf", "sqlite", "sqlite3", "db",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    SecretFile(String),
    BinaryDetected(String),
    ExceededMaxSize { size: u64, max: u64 },
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub max_file_size_bytes: Option<u64>,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            max_file_size_bytes: Some(DEFAULT_MAX_FILE_SIZE_BYTES),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDoc {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub extension: Option<String>,
    pub language: Option<Language>,
    pub doc_type: DocumentType,
    pub project: Option<String>,
    pub file_size_bytes: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub is_lossy_encoding: bool,
}

impl ExtractedDoc {
    pub fn into_indexed_doc(
        self,
        id: DocumentId,
        folder_id: FolderId,
        relative_path: String,
        absolute_path: String,
        content_hash: Option<String>,
        indexed_at: DateTime<Utc>,
    ) -> IndexedDocument {
        IndexedDocument {
            id,
            folder_id,
            relative_path,
            absolute_path,
            title: self.title,
            content: self.content,
            tags: self.tags,
            extension: self.extension,
            language: self.language,
            doc_type: self.doc_type,
            project: self.project,
            file_size_bytes: self.file_size_bytes,
            modified_at: self.modified_at.unwrap_or(indexed_at),
            indexed_at,
            content_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionResult {
    Extracted(ExtractedDoc),
    Skipped(SkipReason),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DocumentExtractor;

impl DocumentExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(
        &self,
        relative_path: &Path,
        bytes: &[u8],
        file_size: u64,
        modified_at: Option<DateTime<Utc>>,
        options: &ExtractOptions,
    ) -> ExtractionResult {
        if is_secret_file(relative_path) {
            let filename = relative_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            return ExtractionResult::Skipped(SkipReason::SecretFile(filename));
        }

        if let Some(max_size) = options.max_file_size_bytes
            && file_size > max_size
        {
            return ExtractionResult::Skipped(SkipReason::ExceededMaxSize {
                size: file_size,
                max: max_size,
            });
        }

        let extension = relative_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        if let Some(reason) = detect_binary(bytes, extension.as_deref()) {
            return ExtractionResult::Skipped(SkipReason::BinaryDetected(reason));
        }

        let (text, is_lossy) = decode_text(bytes);

        let doc_type = extension
            .as_deref()
            .and_then(DocumentType::from_extension)
            .unwrap_or(DocumentType::Doc);

        let language = extension
            .as_deref()
            .and_then(Language::from_extension)
            .or_else(|| {
                if doc_type == DocumentType::Doc {
                    Some(Language::Text)
                } else {
                    None
                }
            });

        let project = extract_project(relative_path);

        let (title, content, tags) =
            if extension.as_deref() == Some("md") || extension.as_deref() == Some("markdown") {
                let (stripped_content, front_matter_tags) = extract_front_matter_and_strip(&text);
                let md_title = extract_markdown_h1(&stripped_content).unwrap_or_else(|| {
                    relative_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string()
                });

                (md_title, stripped_content, front_matter_tags)
            } else {
                let file_title = relative_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();

                (file_title, text, Vec::new())
            };

        ExtractionResult::Extracted(ExtractedDoc {
            title,
            content,
            tags,
            extension,
            language,
            doc_type,
            project,
            file_size_bytes: file_size,
            modified_at,
            is_lossy_encoding: is_lossy,
        })
    }
}

pub fn extract_project(relative_path: &Path) -> Option<String> {
    let mut normal_components = relative_path.components().filter_map(|c| match c {
        std::path::Component::Normal(p) => p.to_str(),
        _ => None,
    });

    let first = normal_components.next()?;
    if normal_components.next().is_some() {
        Some(first.to_string())
    } else {
        None
    }
}

pub fn is_secret_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    let name_lower = file_name.to_ascii_lowercase();

    if name_lower.starts_with(".env") {
        return true;
    }

    if name_lower.starts_with("id_rsa")
        || name_lower.starts_with("id_dsa")
        || name_lower.starts_with("id_ecdsa")
        || name_lower.starts_with("id_ed25519")
    {
        return true;
    }

    if name_lower.ends_with(".pem")
        || name_lower.ends_with(".key")
        || name_lower.ends_with(".p12")
        || name_lower.ends_with(".pfx")
        || name_lower.ends_with(".crt")
        || name_lower.ends_with(".keypair")
        || name_lower.ends_with(".keystore")
    {
        return true;
    }

    if name_lower.contains("credential")
        || name_lower.contains("secret")
        || name_lower.contains("token")
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let is_code_ext = matches!(
            ext.as_str(),
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "cs"
                | "rb"
                | "php"
                | "swift"
                | "vue"
                | "svelte"
                | "lua"
                | "sh"
                | "sql"
                | "html"
                | "css"
                | "scss"
        );

        if !is_code_ext {
            return true;
        }
    }

    false
}

fn detect_binary(bytes: &[u8], extension: Option<&str>) -> Option<String> {
    if let Some(ext) = extension {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        if BINARY_EXTENSIONS.contains(&clean.as_str()) {
            return Some("ExtensionBlacklist".to_string());
        }
    }

    let magic_slice = &bytes[..MAGIC_BYTES_LIMIT.min(bytes.len())];
    if let Some(kind) = infer::get(magic_slice) {
        let mime = kind.mime_type();
        if !mime.starts_with("text/") {
            match kind.matcher_type() {
                infer::MatcherType::App
                | infer::MatcherType::Archive
                | infer::MatcherType::Audio
                | infer::MatcherType::Book
                | infer::MatcherType::Doc
                | infer::MatcherType::Font
                | infer::MatcherType::Image
                | infer::MatcherType::Video => {
                    return Some(format!("MagicBytes:{mime}"));
                }
                _ => {
                    if mime.starts_with("image/")
                        || mime.starts_with("audio/")
                        || mime.starts_with("video/")
                        || mime == "application/pdf"
                        || mime == "application/octet-stream"
                        || mime == "application/zip"
                        || mime == "application/x-executable"
                    {
                        return Some(format!("MagicBytes:{mime}"));
                    }
                }
            }
        }
    }

    let scan_len = BINARY_SCAN_BYTES_LIMIT.min(bytes.len());
    if bytes[..scan_len].contains(&0x00) {
        return Some("NullByteDetected".to_string());
    }

    None
}

fn decode_text(bytes: &[u8]) -> (String, bool) {
    if let Ok(utf8_str) = std::str::from_utf8(bytes) {
        return (utf8_str.to_string(), false);
    }

    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (cow, had_errors) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
        return (cow.into_owned(), had_errors);
    }

    if let Some(cow) =
        encoding_rs::WINDOWS_1252.decode_without_bom_handling_and_without_replacement(bytes)
    {
        return (cow.into_owned(), false);
    }

    let (cow, _, had_errors) = encoding_rs::UTF_8.decode(bytes);
    (cow.into_owned(), had_errors)
}

fn extract_markdown_h1(markdown: &str) -> Option<String> {
    let parser = Parser::new(markdown);
    let mut in_h1 = false;
    let mut h1_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
                h1_text.clear();
            }
            Event::Text(text) | Event::Code(text) if in_h1 => {
                h1_text.push_str(&text);
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if in_h1 => {
                let trimmed = h1_text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
                in_h1 = false;
            }
            _ => {}
        }
    }

    None
}

fn extract_front_matter_and_strip(content: &str) -> (String, Vec<String>) {
    let trimmed_start = content.trim_start_matches('\u{feff}');
    let (has_fm, after_lead) = if let Some(stripped) = trimmed_start.strip_prefix("---\r\n") {
        (true, stripped)
    } else if let Some(stripped) = trimmed_start.strip_prefix("---\n") {
        (true, stripped)
    } else {
        (false, trimmed_start)
    };

    if !has_fm {
        return (trimmed_start.to_string(), Vec::new());
    }

    let end_idx = after_lead
        .find("\r\n---")
        .or_else(|| after_lead.find("\n---"));

    let Some(idx) = end_idx else {
        return (trimmed_start.to_string(), Vec::new());
    };

    let yaml_part = &after_lead[..idx];
    let remainder = &after_lead[idx..];

    let post_front_matter = if let Some(rest) = remainder.strip_prefix("\r\n---\r\n") {
        rest
    } else if let Some(rest) = remainder.strip_prefix("\n---\n") {
        rest
    } else if let Some(rest) = remainder.strip_prefix("\r\n---\n") {
        rest
    } else if let Some(rest) = remainder.strip_prefix("\n---\r\n") {
        rest
    } else {
        remainder
    };

    let tags = parse_yaml_tags(yaml_part);
    (post_front_matter.trim_start().to_string(), tags)
}

fn parse_yaml_tags(yaml_str: &str) -> Vec<String> {
    let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) else {
        return Vec::new();
    };

    let Some(mapping) = val.as_mapping() else {
        return Vec::new();
    };

    let mut result = Vec::new();

    let tags_val = mapping
        .get(serde_yaml::Value::String("tags".to_string()))
        .or_else(|| mapping.get(serde_yaml::Value::String("tag".to_string())));

    if let Some(tags) = tags_val {
        match tags {
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() && !result.iter().any(|existing| existing == trimmed)
                        {
                            result.push(trimmed.to_string());
                        }
                    }
                }
            }
            serde_yaml::Value::String(s) => {
                for part in s.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() && !result.iter().any(|existing| existing == trimmed) {
                        result.push(trimmed.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_markdown_h1_title_and_front_matter_strip() {
        let md = r#"---
tags:
  - rust
  - concurrency
---
# Ownership and Lifetimes

This is the document content explaining borrow checker.
"#;
        let extractor = DocumentExtractor::new();
        let path = Path::new("rust/ownership.md");
        let result = extractor.extract(
            path,
            md.as_bytes(),
            md.len() as u64,
            None,
            &ExtractOptions::default(),
        );

        match result {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.title, "Ownership and Lifetimes");
                assert_eq!(doc.tags, vec!["rust", "concurrency"]);
                assert_eq!(doc.project, Some("rust".to_string()));
                assert_eq!(doc.extension, Some("md".to_string()));
                assert_eq!(doc.language, Some(Language::Markdown));
                assert_eq!(doc.doc_type, DocumentType::Doc);
                assert!(!doc.content.contains("---"));
                assert!(doc.content.starts_with("# Ownership and Lifetimes"));
                assert!(!doc.is_lossy_encoding);
            }
            ExtractionResult::Skipped(reason) => panic!("Expected extracted, got {reason:?}"),
        }
    }

    #[test]
    fn test_markdown_fallback_to_filename_when_no_h1() {
        let md = "## Secondary Heading\n\nNo H1 heading here.";
        let extractor = DocumentExtractor::new();
        let path = Path::new("notes.md");
        let result = extractor.extract(
            path,
            md.as_bytes(),
            md.len() as u64,
            None,
            &ExtractOptions::default(),
        );

        match result {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.title, "notes.md");
                assert!(doc.project.is_none());
            }
            ExtractionResult::Skipped(reason) => panic!("Unexpected skip: {reason:?}"),
        }
    }

    #[test]
    fn test_code_file_extraction() {
        let code = "fn main() {\n    println!(\"Hello LynxSearch\");\n}";
        let extractor = DocumentExtractor::new();
        let path = Path::new("crates/backend/src/main.rs");
        let result = extractor.extract(
            path,
            code.as_bytes(),
            code.len() as u64,
            None,
            &ExtractOptions::default(),
        );

        match result {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.title, "main.rs");
                assert_eq!(doc.project, Some("crates".to_string()));
                assert_eq!(doc.doc_type, DocumentType::Code);
                assert_eq!(doc.language, Some(Language::Rust));
                assert_eq!(doc.content, code);
                assert!(doc.tags.is_empty());
            }
            ExtractionResult::Skipped(reason) => panic!("Unexpected skip: {reason:?}"),
        }
    }

    #[test]
    fn test_config_file_extraction() {
        let json = r#"{"name": "lynx", "version": "0.1.0"}"#;
        let extractor = DocumentExtractor::new();
        let path = Path::new("package.json");
        let result = extractor.extract(
            path,
            json.as_bytes(),
            json.len() as u64,
            None,
            &ExtractOptions::default(),
        );

        match result {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.title, "package.json");
                assert_eq!(doc.doc_type, DocumentType::Config);
                assert_eq!(doc.language, Some(Language::Json));
                assert!(doc.project.is_none());
            }
            ExtractionResult::Skipped(reason) => panic!("Unexpected skip: {reason:?}"),
        }
    }

    #[test]
    fn test_fallback_unregistered_extension_to_doc_text() {
        let text = "Log entry 2026-10-03 info: starting server";
        let extractor = DocumentExtractor::new();
        let path = Path::new("logs/server.log");
        let result = extractor.extract(
            path,
            text.as_bytes(),
            text.len() as u64,
            None,
            &ExtractOptions::default(),
        );

        match result {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.title, "server.log");
                assert_eq!(doc.doc_type, DocumentType::Doc);
                assert_eq!(doc.language, Some(Language::Text));
                assert_eq!(doc.project, Some("logs".to_string()));
            }
            ExtractionResult::Skipped(reason) => panic!("Unexpected skip: {reason:?}"),
        }
    }

    #[test]
    fn test_project_extraction_root_fallback() {
        assert_eq!(extract_project(Path::new("README.md")), None);
        assert_eq!(extract_project(Path::new("LICENSE")), None);
        assert_eq!(
            extract_project(Path::new("rust/ownership.md")),
            Some("rust".to_string())
        );
        assert_eq!(
            extract_project(Path::new("deep/nested/path/to/file.rs")),
            Some("deep".to_string())
        );
    }

    #[test]
    fn test_secret_file_rejection() {
        let extractor = DocumentExtractor::new();
        let files = [
            ".env",
            ".env.local",
            "id_rsa",
            "id_ed25519",
            "server.key",
            "cert.pem",
            "credentials.json",
        ];

        for f in files {
            let res = extractor.extract(
                Path::new(f),
                b"dummy secret content",
                20,
                None,
                &ExtractOptions::default(),
            );
            assert!(
                matches!(res, ExtractionResult::Skipped(SkipReason::SecretFile(_))),
                "File {f} should be rejected as secret"
            );
        }

        // Code file with 'token' in name should NOT be rejected
        let res = extractor.extract(
            Path::new("src/token.rs"),
            b"pub struct Token;",
            17,
            None,
            &ExtractOptions::default(),
        );
        assert!(matches!(res, ExtractionResult::Extracted(_)));
    }

    #[test]
    fn test_max_file_size_exceeded() {
        let extractor = DocumentExtractor::new();
        let options = ExtractOptions {
            max_file_size_bytes: Some(100),
        };
        let res = extractor.extract(Path::new("large.txt"), &[b'a'; 150], 150, None, &options);
        assert_eq!(
            res,
            ExtractionResult::Skipped(SkipReason::ExceededMaxSize {
                size: 150,
                max: 100
            })
        );
    }

    #[test]
    fn test_binary_detection_tiers() {
        let extractor = DocumentExtractor::new();

        // Tier 1: Extension blacklist
        let res1 = extractor.extract(
            Path::new("image.png"),
            b"dummy",
            5,
            None,
            &ExtractOptions::default(),
        );
        assert_eq!(
            res1,
            ExtractionResult::Skipped(SkipReason::BinaryDetected("ExtensionBlacklist".to_string()))
        );

        // Tier 2: Magic bytes via infer (PNG header on unknown extension)
        let png_header = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52,
        ];
        let res2 = extractor.extract(
            Path::new("mystery.dat"),
            &png_header,
            png_header.len() as u64,
            None,
            &ExtractOptions::default(),
        );
        assert!(
            matches!(res2, ExtractionResult::Skipped(SkipReason::BinaryDetected(ref s)) if s.starts_with("MagicBytes:"))
        );

        // Tier 3: Null byte scan
        let mut null_bytes = vec![b'a'; 100];
        null_bytes[40] = 0x00;
        let res3 = extractor.extract(
            Path::new("corrupted.txt"),
            &null_bytes,
            100,
            None,
            &ExtractOptions::default(),
        );
        assert_eq!(
            res3,
            ExtractionResult::Skipped(SkipReason::BinaryDetected("NullByteDetected".to_string()))
        );
    }

    #[test]
    fn test_encoding_decoding_windows_1252_and_lossy() {
        let (utf8_text, is_lossy) = decode_text("Valid UTF-8".as_bytes());
        assert_eq!(utf8_text, "Valid UTF-8");
        assert!(!is_lossy);

        // Windows-1252 byte: 0x93 (left double quote) and 0x94 (right double quote)
        let win1252_bytes = [0x93, b'H', b'e', b'l', b'l', b'o', 0x94];
        let (win_text, is_lossy_win) = decode_text(&win1252_bytes);
        assert!(!win_text.is_empty());
        assert!(!is_lossy_win);
    }

    #[test]
    fn test_into_indexed_doc_mapping() {
        let extracted = ExtractedDoc {
            title: "Test".to_string(),
            content: "Content".to_string(),
            tags: vec!["tag1".to_string()],
            extension: Some("rs".to_string()),
            language: Some(Language::Rust),
            doc_type: DocumentType::Code,
            project: Some("lynx".to_string()),
            file_size_bytes: 50,
            modified_at: None,
            is_lossy_encoding: false,
        };

        let now = Utc::now();
        let doc_id = DocumentId::from_uuid(Uuid::new_v4());
        let folder_id = FolderId::from_uuid(Uuid::new_v4());
        let indexed = extracted.into_indexed_doc(
            doc_id,
            folder_id,
            "src/lib.rs".to_string(),
            "/home/user/src/lib.rs".to_string(),
            Some("mock_hash".to_string()),
            now,
        );

        assert_eq!(indexed.id, doc_id);
        assert_eq!(indexed.folder_id, folder_id);
        assert_eq!(indexed.title, "Test");
        assert_eq!(indexed.modified_at, now);
        assert_eq!(indexed.content_hash, Some("mock_hash".to_string()));
    }

    #[test]
    fn test_phase0_canonical_mappings_and_secrets() {
        let extractor = DocumentExtractor::new();
        let opts = ExtractOptions::default();

        // 1. Verify code mappings
        let code_exts = [
            ("rs", Language::Rust),
            ("ts", Language::Typescript),
            ("tsx", Language::Typescript),
            ("js", Language::Javascript),
            ("jsx", Language::Javascript),
            ("py", Language::Python),
            ("go", Language::Go),
            ("java", Language::Java),
            ("kt", Language::Kotlin),
            ("c", Language::C),
            ("cpp", Language::Cpp),
            ("h", Language::C),
            ("cs", Language::Csharp),
            ("rb", Language::Ruby),
            ("php", Language::Php),
            ("swift", Language::Swift),
            ("sh", Language::Shell),
            ("sql", Language::Sql),
            ("html", Language::Html),
            ("css", Language::Css),
            ("scss", Language::Scss),
            ("vue", Language::Vue),
            ("svelte", Language::Svelte),
            ("lua", Language::Lua),
        ];

        for (ext, expected_lang) in code_exts {
            let filename = format!("src/file.{ext}");
            let res = extractor.extract(Path::new(&filename), b"content", 7, None, &opts);
            match res {
                ExtractionResult::Extracted(doc) => {
                    assert_eq!(
                        doc.doc_type,
                        DocumentType::Code,
                        "failed doc_type for {ext}"
                    );
                    assert_eq!(
                        doc.language,
                        Some(expected_lang),
                        "failed language for {ext}"
                    );
                }
                other => panic!("expected extracted for {ext}, got {other:?}"),
            }
        }

        // 2. Verify config mappings
        let config_exts = [
            ("json", Language::Json),
            ("toml", Language::Toml),
            ("yaml", Language::Yaml),
            ("yml", Language::Yaml),
            ("ini", Language::Ini),
        ];

        for (ext, expected_lang) in config_exts {
            let filename = format!("config/settings.{ext}");
            let res = extractor.extract(Path::new(&filename), b"key = value", 11, None, &opts);
            match res {
                ExtractionResult::Extracted(doc) => {
                    assert_eq!(
                        doc.doc_type,
                        DocumentType::Config,
                        "failed doc_type for {ext}"
                    );
                    assert_eq!(
                        doc.language,
                        Some(expected_lang),
                        "failed language for {ext}"
                    );
                }
                other => panic!("expected extracted for {ext}, got {other:?}"),
            }
        }

        // 3. Verify secrets rejection
        let secret_files = [
            ".env",
            ".env.local",
            ".env.production",
            "id_rsa",
            "id_rsa.pub",
            "id_ed25519",
            "server.pem",
            "cert.key",
            "keystore.p12",
            "keys.pfx",
            "secret.json",
            "credentials.yaml",
            "api_token.txt",
        ];

        for secret in secret_files {
            let res = extractor.extract(Path::new(secret), b"super_secret_value", 18, None, &opts);
            assert!(
                matches!(res, ExtractionResult::Skipped(SkipReason::SecretFile(_))),
                "secret {secret} was not skipped!"
            );
        }
    }
}
