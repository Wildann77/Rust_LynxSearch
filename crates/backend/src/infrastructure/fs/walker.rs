use crate::domain::ports::file_system::{DiscoveredFile, FileWalker, WalkOptions};
use crate::error::AppError;
use chrono::{DateTime, Utc};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use std::path::{Path, PathBuf};

/// Daftar folder yang diabaikan secara default untuk menjaga performa dan relevansi indeks.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[".git", "node_modules", "target", "dist", "build"];

/// Deteksi berkas sensitif / secret agar tidak pernah terbaca atau terindeks.
pub fn is_secret_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    let name_lower = file_name.to_ascii_lowercase();

    // 1. Berkas environment (.env, .env.local, .env.production, dsb.)
    if name_lower.starts_with(".env") {
        return true;
    }

    // 2. Kunci SSH private/public
    if name_lower.starts_with("id_rsa")
        || name_lower.starts_with("id_dsa")
        || name_lower.starts_with("id_ecdsa")
        || name_lower.starts_with("id_ed25519")
    {
        return true;
    }

    // 3. Sertifikat dan kunci kriptografi
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

    // 4. Berkas kredensial, token, dan secret
    if name_lower.contains("credential")
        || name_lower.contains("secret")
        || name_lower.contains("token")
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Jangan blokir berkas source code yang sah (misal token.rs, tokenizer.py)
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

/// Memeriksa apakah nama folder termasuk dalam folder default yang diabaikan.
pub fn is_default_ignored_dir(name: &str) -> bool {
    let name_lower = name.to_ascii_lowercase();
    DEFAULT_IGNORED_DIRS.iter().any(|&dir| name_lower == dir)
}

/// Implementasi lokal adapter filesystem walker berbasis crate `ignore`.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFileWalker;

impl LocalFileWalker {
    pub fn new() -> Self {
        Self
    }
}

impl FileWalker for LocalFileWalker {
    fn walk<'a>(
        &'a self,
        root: &'a Path,
        options: &'a WalkOptions,
    ) -> Result<Box<dyn Iterator<Item = Result<DiscoveredFile, AppError>> + Send + 'a>, AppError>
    {
        if !root.exists() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory does not exist: {}", root.display()),
            )));
        }
        if !root.is_dir() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Path is not a directory: {}", root.display()),
            )));
        }

        let mut builder = WalkBuilder::new(root);

        builder
            .hidden(options.skip_hidden)
            .git_ignore(options.respect_gitignore)
            .git_global(options.respect_gitignore)
            .git_exclude(options.respect_gitignore)
            .ignore(true)
            .parents(true)
            .require_git(false)
            .follow_links(false);

        if let Some(depth) = options.max_depth {
            builder.max_depth(Some(depth));
        }

        // Siapkan override matcher untuk pola default dan custom ignore
        let mut override_builder = OverrideBuilder::new(root);
        for default_dir in DEFAULT_IGNORED_DIRS {
            let _ = override_builder.add(&format!("!{default_dir}"));
            let _ = override_builder.add(&format!("!{default_dir}/**"));
        }

        for pattern in &options.custom_ignore_patterns {
            let trimmed = pattern.trim();
            if trimmed.is_empty() {
                continue;
            }
            let rule = if trimmed.starts_with('!') {
                trimmed.to_string()
            } else {
                format!("!{trimmed}")
            };
            if let Err(err) = override_builder.add(&rule) {
                tracing::warn!(pattern = %trimmed, error = %err, "Failed to register custom ignore pattern");
            }
        }

        if let Ok(overrides) = override_builder.build() {
            builder.overrides(overrides);
        }

        // Terapkan filter callback untuk proteksi secret dan pengabaian folder
        let custom_patterns = options.custom_ignore_patterns.clone();
        builder.filter_entry(move |entry| {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return true,
            };

            // Jangan telusuri berkas atau direktori sensitif
            if is_secret_file(path) {
                return false;
            }

            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if is_default_ignored_dir(file_name) {
                    return false;
                }
                if custom_patterns.iter().any(|p| p.trim() == file_name) {
                    return false;
                }
            } else if custom_patterns.iter().any(|p| p.trim() == file_name) {
                return false;
            }

            true
        });

        let walk = builder.build();
        let root_buf = root.to_path_buf();

        let iter = walk.filter_map(move |entry_res| {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => return Some(Err(AppError::Io(std::io::Error::other(e.to_string())))),
            };

            let path = entry.path();

            // Hanya hasilkan berkas reguler (bukan direktori)
            let file_type = match entry.file_type() {
                Some(ft) => ft,
                None => return None,
            };
            if !file_type.is_file() {
                return None;
            }

            // Validasi proteksi secret tambahan
            if is_secret_file(path) {
                return None;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => return Some(Err(AppError::Io(std::io::Error::other(e.to_string())))),
            };

            let relative_path = match path.strip_prefix(&root_buf) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => PathBuf::from(entry.file_name()),
            };

            let modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);

            Some(Ok(DiscoveredFile {
                relative_path,
                absolute_path: path.to_path_buf(),
                file_size: metadata.len(),
                modified_at,
            }))
        });

        Ok(Box::new(iter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_secret_detection() {
        assert!(is_secret_file(Path::new(".env")));
        assert!(is_secret_file(Path::new(".env.local")));
        assert!(is_secret_file(Path::new(".env.production")));
        assert!(is_secret_file(Path::new("id_rsa")));
        assert!(is_secret_file(Path::new("id_ed25519.pub")));
        assert!(is_secret_file(Path::new("server.key")));
        assert!(is_secret_file(Path::new("cert.pem")));
        assert!(is_secret_file(Path::new("keystore.p12")));
        assert!(is_secret_file(Path::new("bundle.pfx")));
        assert!(is_secret_file(Path::new("domain.crt")));
        assert!(is_secret_file(Path::new("credentials.json")));
        assert!(is_secret_file(Path::new("client_secret.json")));
        assert!(is_secret_file(Path::new("api_token.txt")));

        // Legitimate source code files must NOT be detected as secrets
        assert!(!is_secret_file(Path::new("token.rs")));
        assert!(!is_secret_file(Path::new("tokenizer.py")));
        assert!(!is_secret_file(Path::new("auth_token.ts")));
        assert!(!is_secret_file(Path::new("main.rs")));
        assert!(!is_secret_file(Path::new("README.md")));
    }

    #[test]
    fn test_default_ignored_dirs() {
        assert!(is_default_ignored_dir(".git"));
        assert!(is_default_ignored_dir("node_modules"));
        assert!(is_default_ignored_dir("target"));
        assert!(is_default_ignored_dir("dist"));
        assert!(is_default_ignored_dir("build"));
        assert!(!is_default_ignored_dir("src"));
        assert!(!is_default_ignored_dir("docs"));
    }

    #[test]
    fn test_walker_traversal_and_defaults() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        // Valid files
        create_test_file(&root.join("src/main.rs"), "fn main() {}");
        create_test_file(&root.join("docs/guide.md"), "# Guide");

        // Files in default ignored directories
        create_test_file(
            &root.join("node_modules/pkg/index.js"),
            "module.exports = {};",
        );
        create_test_file(&root.join("target/debug/app"), "binary");
        create_test_file(&root.join("dist/bundle.js"), "bundle");
        create_test_file(&root.join("build/output.js"), "output");
        create_test_file(&root.join(".git/HEAD"), "ref: refs/heads/main");

        // Secret files
        create_test_file(&root.join(".env"), "SECRET=123");
        create_test_file(&root.join("certs/server.key"), "PRIVATE KEY");
        create_test_file(&root.join("id_rsa"), "SSH KEY");
        create_test_file(&root.join("credentials.json"), "{}");

        let walker = LocalFileWalker::new();
        let files = walker.walk_all(root, &WalkOptions::default()).unwrap();

        let rel_paths: Vec<String> = files
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert!(rel_paths.contains(&"src/main.rs".to_string()));
        assert!(rel_paths.contains(&"docs/guide.md".to_string()));

        // Default ignored directories must be excluded
        assert!(!rel_paths.iter().any(|p| p.starts_with("node_modules")));
        assert!(!rel_paths.iter().any(|p| p.starts_with("target")));
        assert!(!rel_paths.iter().any(|p| p.starts_with("dist")));
        assert!(!rel_paths.iter().any(|p| p.starts_with("build")));
        assert!(!rel_paths.iter().any(|p| p.starts_with(".git")));

        // Secrets must be excluded
        assert!(!rel_paths.iter().any(|p| p.contains(".env")));
        assert!(!rel_paths.iter().any(|p| p.contains("server.key")));
        assert!(!rel_paths.iter().any(|p| p.contains("id_rsa")));
        assert!(!rel_paths.iter().any(|p| p.contains("credentials.json")));
    }

    #[test]
    fn test_walker_respects_gitignore() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        create_test_file(&root.join(".gitignore"), "*.tmp\nignored_folder/\n");
        create_test_file(&root.join("file.txt"), "hello");
        create_test_file(&root.join("file.tmp"), "temp data");
        create_test_file(&root.join("ignored_folder/file.rs"), "fn ignored() {}");

        let walker = LocalFileWalker::new();
        let files = walker.walk_all(root, &WalkOptions::default()).unwrap();

        let rel_paths: Vec<String> = files
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert!(rel_paths.contains(&"file.txt".to_string()));
        assert!(!rel_paths.contains(&"file.tmp".to_string()));
        assert!(!rel_paths.iter().any(|p| p.starts_with("ignored_folder")));
    }

    #[test]
    fn test_walker_respects_ignore_file() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        create_test_file(&root.join(".ignore"), "ignore_me.rs\n");
        create_test_file(&root.join("keep_me.rs"), "fn keep() {}");
        create_test_file(&root.join("ignore_me.rs"), "fn ignore() {}");

        let walker = LocalFileWalker::new();
        let files = walker.walk_all(root, &WalkOptions::default()).unwrap();

        let rel_paths: Vec<String> = files
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert!(rel_paths.contains(&"keep_me.rs".to_string()));
        assert!(!rel_paths.contains(&"ignore_me.rs".to_string()));
    }

    #[test]
    fn test_walker_custom_ignore_patterns() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        create_test_file(&root.join("app.log"), "log line");
        create_test_file(&root.join("temp_dir/cache.json"), "{}");
        create_test_file(&root.join("valid.md"), "# Valid");

        let options = WalkOptions {
            custom_ignore_patterns: vec!["*.log".to_string(), "temp_dir".to_string()],
            ..Default::default()
        };

        let walker = LocalFileWalker::new();
        let files = walker.walk_all(root, &options).unwrap();

        let rel_paths: Vec<String> = files
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert!(rel_paths.contains(&"valid.md".to_string()));
        assert!(!rel_paths.contains(&"app.log".to_string()));
        assert!(!rel_paths.iter().any(|p| p.starts_with("temp_dir")));
    }

    #[test]
    fn test_walker_skip_hidden_files() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        create_test_file(&root.join("visible.txt"), "visible");
        create_test_file(&root.join(".hidden.txt"), "hidden");

        let walker = LocalFileWalker::new();

        // Default: skip_hidden = true
        let files_skipped = walker.walk_all(root, &WalkOptions::default()).unwrap();
        let rel_skipped: Vec<String> = files_skipped
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(rel_skipped.contains(&"visible.txt".to_string()));
        assert!(!rel_skipped.contains(&".hidden.txt".to_string()));

        // skip_hidden = false (tapi secret tetap diblokir)
        let options_allow_hidden = WalkOptions {
            skip_hidden: false,
            ..Default::default()
        };
        let files_allowed = walker.walk_all(root, &options_allow_hidden).unwrap();
        let rel_allowed: Vec<String> = files_allowed
            .into_iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(rel_allowed.contains(&"visible.txt".to_string()));
        assert!(rel_allowed.contains(&".hidden.txt".to_string()));
    }

    #[test]
    fn test_walker_not_found_or_invalid_root() {
        let walker = LocalFileWalker::new();
        let missing = Path::new("/non/existent/path/for/sure/12345");
        assert!(walker.walk_all(missing, &WalkOptions::default()).is_err());
    }
}
