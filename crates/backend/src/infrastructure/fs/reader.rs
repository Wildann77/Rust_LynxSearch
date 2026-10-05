use crate::domain::ports::file_system::{FilePayload, FileReader, ReadOptions, compute_sha256};
use crate::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Adapter pembaca konten berkas lokal dengan pembatasan konkurensi (semaphore)
/// dan offload CPU/IO ke Tokio blocking thread pool.
#[derive(Clone, Debug)]
pub struct LocalFileReader {
    semaphore: Arc<Semaphore>,
}

impl LocalFileReader {
    /// Jumlah permit konkurensi default sesuai spesifikasi ARCHITECTURE.md & TASK.md.
    pub const DEFAULT_CONCURRENCY_PERMITS: usize = 50;

    /// Membuat instance LocalFileReader dengan Arc<Semaphore> yang disediakan (misal dari AppState).
    pub fn new(semaphore: Arc<Semaphore>) -> Self {
        Self { semaphore }
    }

    /// Membuat instance LocalFileReader dengan limit default 50 permits.
    pub fn with_default_permits() -> Self {
        Self::new(Arc::new(Semaphore::new(Self::DEFAULT_CONCURRENCY_PERMITS)))
    }

    /// Membuat instance LocalFileReader dengan batas permit kustom.
    pub fn with_permits(permits: usize) -> Self {
        Self::new(Arc::new(Semaphore::new(permits)))
    }

    /// Mengakses referensi Semaphore yang digunakan.
    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }

    /// Jumlah permit yang saat ini tersedia.
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

impl Default for LocalFileReader {
    fn default() -> Self {
        Self::with_default_permits()
    }
}

#[async_trait]
impl FileReader for LocalFileReader {
    async fn read_file(&self, path: &Path, options: &ReadOptions) -> Result<FilePayload, AppError> {
        // Ambil permit semaphore secara asinkron sebelum spawn blocking task
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::Internal("File I/O semaphore closed".into()))?;

        let path_buf = path.to_path_buf();
        let max_file_size = options.max_file_size_bytes;

        // Offload pembacaan I/O disk dan komputasi SHA-256 hash ke thread blocking
        tokio::task::spawn_blocking(move || {
            // Permit dijaga hidup di dalam scope closure dan dilepaskan saat closure selesai
            let _permit = permit;

            let metadata = std::fs::metadata(&path_buf)?;
            if !metadata.is_file() {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Path is not a regular file: {}", path_buf.display()),
                )));
            }

            let file_size = metadata.len();
            if let Some(limit) = max_file_size
                && file_size > limit
            {
                return Err(AppError::ValidationFailed(format!(
                    "File '{}' exceeds maximum allowed size: {} > {} bytes",
                    path_buf.display(),
                    file_size,
                    limit
                )));
            }

            let modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);
            let bytes = std::fs::read(&path_buf)?;
            let hash = compute_sha256(&bytes);

            Ok(FilePayload {
                relative_path: None,
                absolute_path: path_buf,
                bytes,
                hash,
                size: file_size,
                modified_at,
            })
        })
        .await
        .map_err(|join_err| {
            AppError::Internal(format!("File read blocking task failed: {join_err}"))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::file_system::DiscoveredFile;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_regular_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sample.txt");
        let content = b"Hello LynxSearch Reader!";
        fs::write(&file_path, content).unwrap();

        let reader = LocalFileReader::with_default_permits();
        let payload = reader
            .read_file(&file_path, &ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(payload.bytes, content);
        assert_eq!(payload.size, content.len() as u64);
        assert_eq!(payload.hash, compute_sha256(content));
        assert_eq!(payload.absolute_path, file_path);
        assert!(payload.modified_at.is_some());
        assert_eq!(payload.relative_path, None);
    }

    #[tokio::test]
    async fn test_read_empty_file_hash() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let reader = LocalFileReader::with_default_permits();
        let payload = reader
            .read_file(&file_path, &ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(payload.bytes, Vec::<u8>::new());
        assert_eq!(payload.size, 0);
        // SHA-256 baku untuk empty string
        assert_eq!(
            payload.hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn test_read_discovered_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("doc.md");
        fs::write(&file_path, b"# Title\nContent").unwrap();

        let discovered = DiscoveredFile {
            relative_path: PathBuf::from("nested/doc.md"),
            absolute_path: file_path.clone(),
            file_size: 15,
            modified_at: Some(Utc::now()),
        };

        let reader = LocalFileReader::with_default_permits();
        let payload = reader
            .read_discovered(&discovered, &ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(payload.relative_path, Some(PathBuf::from("nested/doc.md")));
        assert_eq!(payload.absolute_path, file_path);
        assert_eq!(payload.bytes, b"# Title\nContent");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let reader = LocalFileReader::with_default_permits();
        let err = reader
            .read_file(Path::new("/nonexistent/file.xyz"), &ReadOptions::default())
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::Io(_)));
    }

    #[tokio::test]
    async fn test_read_directory_fails() {
        let dir = tempdir().unwrap();
        let reader = LocalFileReader::with_default_permits();
        let err = reader
            .read_file(dir.path(), &ReadOptions::default())
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::Io(_)));
    }

    #[tokio::test]
    async fn test_read_file_exceeds_max_size() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("large.bin");
        let content = vec![0u8; 1024]; // 1 KB
        fs::write(&file_path, &content).unwrap();

        let reader = LocalFileReader::with_default_permits();
        let opts = ReadOptions::new(Some(512)); // Batas 512 bytes

        let err = reader.read_file(&file_path, &opts).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationFailed(_)));
    }

    #[tokio::test]
    async fn test_semaphore_concurrency_throttling() {
        let dir = tempdir().unwrap();
        let mut file_paths = Vec::new();

        // Siapkan 10 berkas
        for i in 0..10 {
            let p = dir.path().join(format!("file_{i}.txt"));
            fs::write(&p, format!("Content of file {i}")).unwrap();
            file_paths.push(p);
        }

        // Batasi konkurensi menjadi 2 permits
        let reader = Arc::new(LocalFileReader::with_permits(2));
        assert_eq!(reader.available_permits(), 2);

        let mut join_handles = Vec::new();
        for path in file_paths {
            let r = Arc::clone(&reader);
            join_handles.push(tokio::spawn(async move {
                r.read_file(&path, &ReadOptions::default()).await
            }));
        }

        for handle in join_handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Semua permit harus kembali pulih setelah semua task selesai
        assert_eq!(reader.available_permits(), 2);
    }
}
