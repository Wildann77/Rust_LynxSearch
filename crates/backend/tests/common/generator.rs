use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

#[allow(dead_code)]
pub struct FixtureTreeGenerator {
    file_count: usize,
    deterministic: bool,
}

#[allow(dead_code)]
impl FixtureTreeGenerator {
    pub fn new(file_count: usize) -> Self {
        Self {
            file_count,
            deterministic: true,
        }
    }

    pub fn deterministic(mut self, val: bool) -> Self {
        self.deterministic = val;
        self
    }

    /// Generates a test fixture directory with `file_count` files distributed in subfolders.
    pub fn generate(&self) -> (TempDir, Vec<PathBuf>) {
        let temp_dir = TempDir::new().expect("Failed to create tempdir for dynamic fixtures");
        let root = temp_dir.path();
        let mut created_files = Vec::with_capacity(self.file_count);

        // Subdirectories to spread files evenly
        let subdirs = [
            "docs/guides",
            "docs/api",
            "src/core",
            "src/utils",
            "configs",
            "benchmarks",
        ];

        for sub in &subdirs {
            fs::create_dir_all(root.join(sub)).expect("Create subdir");
        }

        for i in 0..self.file_count {
            let subdir = subdirs[i % subdirs.len()];
            let (ext, content) = match i % 5 {
                0 => (
                    "md",
                    format!(
                        "---\ntitle: Doc {i}\ntags:\n  - guide\n  - tag_{}\n---\n# Document {i}\nDeterministic content for document index {i}.\n",
                        i % 10
                    ),
                ),
                1 => (
                    "rs",
                    format!(
                        "//! Source module {i}\npub fn execute_step_{i}() -> usize {{\n    let calculated_val = {i} * 2;\n    calculated_val\n}}\n"
                    ),
                ),
                2 => (
                    "toml",
                    format!(
                        "[config_{i}]\nenabled = true\nworkers = {}\nprefix = \"worker_{i}\"\n",
                        (i % 8) + 1
                    ),
                ),
                3 => (
                    "json",
                    format!(
                        "{{\n  \"id\": {i},\n  \"name\": \"item_{i}\",\n  \"active\": {}\n}}\n",
                        i % 2 == 0
                    ),
                ),
                _ => (
                    "txt",
                    format!(
                        "Plain text log or document {i} with search token keyword_{}.\n",
                        i % 20
                    ),
                ),
            };

            let filename = format!("file_{i:04}.{ext}");
            let file_path = root.join(subdir).join(filename);
            let mut file = File::create(&file_path).expect("Create fixture file");
            file.write_all(content.as_bytes())
                .expect("Write fixture content");
            created_files.push(file_path);
        }

        (temp_dir, created_files)
    }
}
