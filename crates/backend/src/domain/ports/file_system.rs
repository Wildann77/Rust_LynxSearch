use crate::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Menghitung representasi hex string SHA-256 (64 karakter lowercase) dari byte slice.
pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Representasi berkas hasil temuan selama traversal filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
    /// Relative path terhadap root folder yang di-scan.
    pub relative_path: PathBuf,
    /// Absolute path berkas pada filesystem lokal.
    pub absolute_path: PathBuf,
    /// Ukuran file dalam bytes.
    pub file_size: u64,
    /// Timestamp modifikasi terakhir (jika tersedia).
    pub modified_at: Option<DateTime<Utc>>,
}

/// Opsi konfigurasi pemindaian direktori.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkOptions {
    /// Pola ignore kustom tambahan (misalnya dari AppSettings atau input pengguna).
    pub custom_ignore_patterns: Vec<String>,
    /// Apakah berkas/direktori tersembunyi (diawali titik) diabaikan.
    pub skip_hidden: bool,
    /// Apakah aturan .gitignore dan .ignore dihormati.
    pub respect_gitignore: bool,
    /// Kedalaman maksimum traversal (None = tanpa batas).
    pub max_depth: Option<usize>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            custom_ignore_patterns: Vec::new(),
            skip_hidden: true,
            respect_gitignore: true,
            max_depth: None,
        }
    }
}

/// Port trait untuk abstraksi filesystem walker.
pub trait FileWalker: Send + Sync {
    /// Menghasilkan iterator stream dari berkas yang ditemukan.
    fn walk<'a>(
        &'a self,
        root: &'a Path,
        options: &'a WalkOptions,
    ) -> Result<Box<dyn Iterator<Item = Result<DiscoveredFile, AppError>> + Send + 'a>, AppError>;

    /// Mengumpulkan seluruh berkas sekaligus (eager collection).
    fn walk_all(
        &self,
        root: &Path,
        options: &WalkOptions,
    ) -> Result<Vec<DiscoveredFile>, AppError> {
        let iter = self.walk(root, options)?;
        let mut files = Vec::new();
        for item in iter {
            files.push(item?);
        }
        Ok(files)
    }
}

/// Representasi muatan konten berkas mentah beserta metadata yang dibutuhkan document extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePayload {
    /// Relative path terhadap root folder pemindaian (jika tersedia).
    pub relative_path: Option<PathBuf>,
    /// Absolute path berkas pada filesystem lokal.
    pub absolute_path: PathBuf,
    /// Isi berkas mentah (raw bytes).
    pub bytes: Vec<u8>,
    /// SHA-256 hash representasi string hex 64-karakter lowercase.
    pub hash: String,
    /// Ukuran berkas dalam bytes.
    pub size: u64,
    /// Timestamp modifikasi terakhir (jika tersedia).
    pub modified_at: Option<DateTime<Utc>>,
}

impl FilePayload {
    /// Membuat FilePayload baru dan menghitung SHA-256 hash serta ukuran secara otomatis dari `bytes`.
    pub fn new(
        absolute_path: PathBuf,
        relative_path: Option<PathBuf>,
        bytes: Vec<u8>,
        modified_at: Option<DateTime<Utc>>,
    ) -> Self {
        let hash = compute_sha256(&bytes);
        let size = bytes.len() as u64;
        Self {
            relative_path,
            absolute_path,
            bytes,
            hash,
            size,
            modified_at,
        }
    }
}

/// Opsi konfigurasi pembacaan berkas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadOptions {
    /// Batas ukuran maksimum berkas dalam bytes (None = tanpa batas).
    pub max_file_size_bytes: Option<u64>,
}

impl ReadOptions {
    pub fn new(max_file_size_bytes: Option<u64>) -> Self {
        Self {
            max_file_size_bytes,
        }
    }
}

/// Port trait untuk abstraksi pembaca konten berkas dan hashing (I/O offloading).
#[async_trait]
pub trait FileReader: Send + Sync {
    /// Membaca berkas dari filesystem lokal secara asinkron.
    async fn read_file(&self, path: &Path, options: &ReadOptions) -> Result<FilePayload, AppError>;

    /// Membaca berkas berdasarkan entitas `DiscoveredFile` hasil scan traversal.
    async fn read_discovered(
        &self,
        discovered: &DiscoveredFile,
        options: &ReadOptions,
    ) -> Result<FilePayload, AppError> {
        let mut payload = self.read_file(&discovered.absolute_path, options).await?;
        payload.relative_path = Some(discovered.relative_path.clone());
        if payload.modified_at.is_none() {
            payload.modified_at = discovered.modified_at;
        }
        Ok(payload)
    }

    /// Membaca berkas dengan menyertakan relative path eksplisit.
    async fn read_file_with_relative_path(
        &self,
        absolute_path: &Path,
        relative_path: &Path,
        options: &ReadOptions,
    ) -> Result<FilePayload, AppError> {
        let mut payload = self.read_file(absolute_path, options).await?;
        payload.relative_path = Some(relative_path.to_path_buf());
        Ok(payload)
    }
}
