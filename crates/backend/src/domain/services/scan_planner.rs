use crate::domain::models::{AppSettings, DocumentId, DocumentStatus, FolderId, RegistryEntry};
use crate::domain::ports::DiscoveredFile;
use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Menormalisasi relative path menjadi representasi string forward-slash tanpa prefix `./` atau `/`.
pub fn normalize_path_str(path: impl AsRef<Path>) -> String {
    let raw = path.as_ref().to_string_lossy();
    let normalized = raw.replace('\\', "/");
    let mut trimmed = normalized.as_str();

    while let Some(stripped) = trimmed.strip_prefix("./") {
        trimmed = stripped;
    }
    while let Some(stripped) = trimmed.strip_prefix('/') {
        trimmed = stripped;
    }

    trimmed.to_string()
}

/// Alasan mengapa berkas dilewati selama perencanaan pemindaian.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanSkipReason {
    /// Berkas sengaja dikecualikan pengguna di database (anti-ghost resurrection).
    UserExcluded,
    /// Metadata berkas (mtime dan ukuran) identik dengan DB (jalur cepat tanpa hashing).
    Unchanged,
    /// Metadata berubah tetapi hash konten identik setelah dikalkulasi ulang.
    HashUnchanged,
    /// Ukuran berkas melebihi batas konfigurasi `max_file_size_bytes`.
    ExceededMaxSize,
    /// Berkas berstatus EXCLUDED di DB tetapi fisiknya hilang dari disk (semantik eksklusi dipertahankan).
    PreservedExcluded,
    /// Gagal membaca atau menghitung hash berkas fisik.
    ReadFailed,
}

impl PlanSkipReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserExcluded => "UserExcluded",
            Self::Unchanged => "Unchanged",
            Self::HashUnchanged => "HashUnchanged",
            Self::ExceededMaxSize => "ExceededMaxSize",
            Self::PreservedExcluded => "PreservedExcluded",
            Self::ReadFailed => "ReadFailed",
        }
    }
}

/// Rencana penambahan berkas baru ke indeks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanAdd {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub relative_path: PathBuf,
    pub file_size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: String,
}

/// Rencana pembaruan berkas yang kontennya telah berubah.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanUpdate {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub relative_path: PathBuf,
    pub file_size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub old_hash: String,
    pub new_hash: String,
}

/// Rencana penghapusan berkas dari indeks karena berkas fisik tidak lagi ditemukan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDelete {
    pub id: DocumentId,
    pub folder_id: FolderId,
    pub relative_path: String,
    pub content_hash: String,
}

/// Rencana berkas yang dilewati beserta alasannya.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSkip {
    pub id: DocumentId,
    pub relative_path: String,
    pub reason: PlanSkipReason,
}

/// Rencana pemindahan/pengubahan nama berkas secara atomik (relokasi).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMove {
    pub old_id: DocumentId,
    pub new_id: DocumentId,
    pub folder_id: FolderId,
    pub old_path: String,
    pub new_path: PathBuf,
    pub content_hash: String,
    pub file_size: u64,
    pub modified_at: Option<DateTime<Utc>>,
}

/// Representasi keseluruhan rencana aksi pemindaian folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanPlan {
    pub to_add: Vec<PlanAdd>,
    pub to_update: Vec<PlanUpdate>,
    pub to_delete: Vec<PlanDelete>,
    pub to_skip: Vec<PlanSkip>,
    pub to_move: Vec<PlanMove>,
}

impl ScanPlan {
    pub fn is_empty(&self) -> bool {
        self.to_add.is_empty()
            && self.to_update.is_empty()
            && self.to_delete.is_empty()
            && self.to_move.is_empty()
    }

    pub fn has_changes(&self) -> bool {
        !self.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.to_add.len() + self.to_update.len() + self.to_delete.len() + self.to_move.len()
    }

    pub fn total_files(&self) -> usize {
        self.total_changes() + self.to_skip.len()
    }
}

struct CandidateAdd {
    id: DocumentId,
    folder_id: FolderId,
    relative_path: PathBuf,
    file_size: u64,
    modified_at: Option<DateTime<Utc>>,
    content_hash: String,
}

struct CandidateDelete {
    id: DocumentId,
    folder_id: FolderId,
    relative_path: String,
    content_hash: String,
}

/// Pure domain service untuk merencanakan aksi sinkronisasi pemindaian berkas.
pub struct ScanPlanner;

impl ScanPlanner {
    /// Menghasilkan `ScanPlan` deterministik murni tanpa interaksi I/O langsung.
    ///
    /// Logika hashing diinjeksikan melalui closure `hasher`, yang hanya dipanggil
    /// ketika file belum ada di DB atau ketika metadata (mtime/ukuran) berbeda.
    pub fn plan<F>(
        folder_id: FolderId,
        inventory: &[DiscoveredFile],
        db_entries: &[RegistryEntry],
        settings: &AppSettings,
        mut hasher: F,
    ) -> ScanPlan
    where
        F: FnMut(&DiscoveredFile) -> Result<String, AppError>,
    {
        let mut db_map: HashMap<String, &RegistryEntry> = HashMap::with_capacity(db_entries.len());
        for entry in db_entries {
            let norm = normalize_path_str(&entry.relative_path);
            db_map.insert(norm, entry);
        }

        let mut fs_set: HashSet<String> = HashSet::with_capacity(inventory.len());
        let mut to_add_candidates: Vec<CandidateAdd> = Vec::new();
        let mut to_update: Vec<PlanUpdate> = Vec::new();
        let mut to_skip: Vec<PlanSkip> = Vec::new();

        // Phase 1A: Evaluasi berkas di filesystem lokal
        for file in inventory {
            let norm_path = normalize_path_str(&file.relative_path);
            if !fs_set.insert(norm_path.clone()) {
                // Lewati duplikat jika traversal menghasilkan path yang sama
                continue;
            }

            let doc_id = DocumentId::from_relative_path(folder_id, &file.relative_path);

            // Periksa batasan ukuran berkas dari konfigurasi
            if settings.max_file_size_bytes > 0 && file.file_size > settings.max_file_size_bytes {
                to_skip.push(PlanSkip {
                    id: doc_id,
                    relative_path: norm_path,
                    reason: PlanSkipReason::ExceededMaxSize,
                });
                continue;
            }

            if let Some(entry) = db_map.get(&norm_path) {
                // Berkas ada di disk dan ada di DB
                if entry.status == DocumentStatus::Excluded {
                    to_skip.push(PlanSkip {
                        id: doc_id,
                        relative_path: norm_path,
                        reason: PlanSkipReason::UserExcluded,
                    });
                    continue;
                }

                // Fast path: periksa kesesuaian mtime dan ukuran berkas
                let mtime_match = match (file.modified_at, entry.updated_at) {
                    (Some(fs_mtime), db_mtime) => {
                        fs_mtime.timestamp_millis() == db_mtime.timestamp_millis()
                    }
                    _ => false,
                };
                let size_match = (file.file_size as i64) == entry.file_size;

                if mtime_match && size_match && entry.status == DocumentStatus::Indexed {
                    to_skip.push(PlanSkip {
                        id: doc_id,
                        relative_path: norm_path,
                        reason: PlanSkipReason::Unchanged,
                    });
                    continue;
                }

                // Jalur lambat: metadata berbeda atau status bukan Indexed -> hitung hash
                let hash = match hasher(file) {
                    Ok(h) => h,
                    Err(_) => {
                        to_skip.push(PlanSkip {
                            id: doc_id,
                            relative_path: norm_path,
                            reason: PlanSkipReason::ReadFailed,
                        });
                        continue;
                    }
                };

                if !entry.content_hash.is_empty()
                    && hash == entry.content_hash
                    && entry.status == DocumentStatus::Indexed
                {
                    to_skip.push(PlanSkip {
                        id: doc_id,
                        relative_path: norm_path,
                        reason: PlanSkipReason::HashUnchanged,
                    });
                } else {
                    to_update.push(PlanUpdate {
                        id: doc_id,
                        folder_id,
                        relative_path: file.relative_path.clone(),
                        file_size: file.file_size,
                        modified_at: file.modified_at,
                        old_hash: entry.content_hash.clone(),
                        new_hash: hash,
                    });
                }
            } else {
                // Berkas ada di disk tetapi belum ada di DB: kandidat to_add
                let hash = match hasher(file) {
                    Ok(h) => h,
                    Err(_) => {
                        to_skip.push(PlanSkip {
                            id: doc_id,
                            relative_path: norm_path,
                            reason: PlanSkipReason::ReadFailed,
                        });
                        continue;
                    }
                };

                to_add_candidates.push(CandidateAdd {
                    id: doc_id,
                    folder_id,
                    relative_path: file.relative_path.clone(),
                    file_size: file.file_size,
                    modified_at: file.modified_at,
                    content_hash: hash,
                });
            }
        }

        // Phase 1B: Evaluasi entri DB yang berkas fisiknya hilang dari disk
        let mut to_delete_candidates: Vec<CandidateDelete> = Vec::new();
        for entry in db_entries {
            let norm = normalize_path_str(&entry.relative_path);
            if !fs_set.contains(&norm) {
                if entry.status == DocumentStatus::Excluded {
                    to_skip.push(PlanSkip {
                        id: entry.id,
                        relative_path: entry.relative_path.clone(),
                        reason: PlanSkipReason::PreservedExcluded,
                    });
                } else {
                    to_delete_candidates.push(CandidateDelete {
                        id: entry.id,
                        folder_id: entry.folder_id,
                        relative_path: entry.relative_path.clone(),
                        content_hash: entry.content_hash.clone(),
                    });
                }
            }
        }

        // Phase 2: Deteksi Relokasi/Rename (Move Matching)
        to_add_candidates.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        to_delete_candidates.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let mut to_move: Vec<PlanMove> = Vec::new();
        let mut to_add: Vec<PlanAdd> = Vec::new();
        let mut matched_delete_indices: HashSet<usize> = HashSet::new();

        for add in to_add_candidates {
            let matched_del_idx = if !add.content_hash.is_empty() {
                to_delete_candidates
                    .iter()
                    .enumerate()
                    .position(|(idx, del)| {
                        !matched_delete_indices.contains(&idx)
                            && !del.content_hash.is_empty()
                            && del.content_hash == add.content_hash
                    })
            } else {
                None
            };

            if let Some(del_idx) = matched_del_idx {
                matched_delete_indices.insert(del_idx);
                let del = &to_delete_candidates[del_idx];
                to_move.push(PlanMove {
                    old_id: del.id,
                    new_id: add.id,
                    folder_id,
                    old_path: del.relative_path.clone(),
                    new_path: add.relative_path,
                    content_hash: add.content_hash,
                    file_size: add.file_size,
                    modified_at: add.modified_at,
                });
            } else {
                to_add.push(PlanAdd {
                    id: add.id,
                    folder_id: add.folder_id,
                    relative_path: add.relative_path,
                    file_size: add.file_size,
                    modified_at: add.modified_at,
                    content_hash: add.content_hash,
                });
            }
        }

        let mut to_delete: Vec<PlanDelete> = Vec::new();
        for (idx, del) in to_delete_candidates.into_iter().enumerate() {
            if !matched_delete_indices.contains(&idx) {
                to_delete.push(PlanDelete {
                    id: del.id,
                    folder_id: del.folder_id,
                    relative_path: del.relative_path,
                    content_hash: del.content_hash,
                });
            }
        }

        ScanPlan {
            to_add,
            to_update,
            to_delete,
            to_skip,
            to_move,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::cell::Cell;

    fn make_test_settings(max_file_size_bytes: u64) -> AppSettings {
        AppSettings {
            max_file_size_bytes,
            ..Default::default()
        }
    }

    fn sample_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 3, 10, 0, 0).unwrap()
    }

    #[test]
    fn test_path_normalization() {
        assert_eq!(normalize_path_str("src/main.rs"), "src/main.rs");
        assert_eq!(normalize_path_str("src\\main.rs"), "src/main.rs");
        assert_eq!(normalize_path_str("./src/main.rs"), "src/main.rs");
        assert_eq!(normalize_path_str(".\\src\\main.rs"), "src/main.rs");
        assert_eq!(normalize_path_str("/src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_fast_path_unchanged_does_not_invoke_hasher() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let doc_id = DocumentId::from_relative_path(folder_id, "notes.md");

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("notes.md"),
            absolute_path: PathBuf::from("/work/notes.md"),
            file_size: 100,
            modified_at: Some(time),
        }];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "notes.md".into(),
            content_hash: "hash_notes_100".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let hasher_called = Cell::new(false);
        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| {
                hasher_called.set(true);
                Ok("some_hash".into())
            },
        );

        assert!(
            !hasher_called.get(),
            "Hasher must not be called on fast path"
        );
        assert!(plan.to_add.is_empty());
        assert!(plan.to_update.is_empty());
        assert!(plan.to_delete.is_empty());
        assert!(plan.to_move.is_empty());
        assert_eq!(plan.to_skip.len(), 1);
        assert_eq!(plan.to_skip[0].reason, PlanSkipReason::Unchanged);
        assert!(!plan.has_changes());
    }

    #[test]
    fn test_slow_path_hash_unchanged_skips() {
        let folder_id = FolderId::new();
        let time1 = sample_time();
        let time2 = Utc.with_ymd_and_hms(2026, 10, 3, 10, 5, 0).unwrap();
        let doc_id = DocumentId::from_relative_path(folder_id, "notes.md");

        // mtime changed, but size and content hash same
        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("notes.md"),
            absolute_path: PathBuf::from("/work/notes.md"),
            file_size: 100,
            modified_at: Some(time2),
        }];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "notes.md".into(),
            content_hash: "stable_hash".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time1,
            updated_at: time1,
        }];

        let hasher_calls = Cell::new(0);
        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| {
                hasher_calls.set(hasher_calls.get() + 1);
                Ok("stable_hash".into())
            },
        );

        assert_eq!(hasher_calls.get(), 1);
        assert!(plan.to_add.is_empty());
        assert!(plan.to_update.is_empty());
        assert_eq!(plan.to_skip.len(), 1);
        assert_eq!(plan.to_skip[0].reason, PlanSkipReason::HashUnchanged);
        assert!(!plan.has_changes());
    }

    #[test]
    fn test_slow_path_content_changed_schedules_update() {
        let folder_id = FolderId::new();
        let time1 = sample_time();
        let time2 = Utc.with_ymd_and_hms(2026, 10, 3, 10, 5, 0).unwrap();
        let doc_id = DocumentId::from_relative_path(folder_id, "notes.md");

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("notes.md"),
            absolute_path: PathBuf::from("/work/notes.md"),
            file_size: 150,
            modified_at: Some(time2),
        }];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "notes.md".into(),
            content_hash: "old_hash".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time1,
            updated_at: time1,
        }];

        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("new_hash".into()),
        );

        assert_eq!(plan.to_update.len(), 1);
        assert_eq!(plan.to_update[0].old_hash, "old_hash");
        assert_eq!(plan.to_update[0].new_hash, "new_hash");
        assert_eq!(plan.to_update[0].file_size, 150);
        assert_eq!(plan.to_update[0].id, doc_id);
        assert!(plan.has_changes());
    }

    #[test]
    fn test_new_file_schedules_add_with_deterministic_id() {
        let folder_id = FolderId::new();
        let time = sample_time();

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("src/lib.rs"),
            absolute_path: PathBuf::from("/work/src/lib.rs"),
            file_size: 250,
            modified_at: Some(time),
        }];

        let db_entries = vec![];

        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("hash_lib".into()),
        );

        assert_eq!(plan.to_add.len(), 1);
        assert_eq!(plan.to_add[0].content_hash, "hash_lib");
        let expected_id = DocumentId::from_relative_path(folder_id, "src/lib.rs");
        assert_eq!(plan.to_add[0].id, expected_id);
    }

    #[test]
    fn test_missing_physical_file_schedules_delete() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let doc_id = DocumentId::from_relative_path(folder_id, "deleted.rs");

        let inventory = vec![];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "deleted.rs".into(),
            content_hash: "hash_del".into(),
            file_size: 50,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("unused".into()),
        );

        assert_eq!(plan.to_delete.len(), 1);
        assert_eq!(plan.to_delete[0].id, doc_id);
        assert_eq!(plan.to_delete[0].relative_path, "deleted.rs");
    }

    #[test]
    fn test_user_excluded_file_on_disk_is_skipped_without_hashing() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let doc_id = DocumentId::from_relative_path(folder_id, "ignored.md");

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("ignored.md"),
            absolute_path: PathBuf::from("/work/ignored.md"),
            file_size: 100,
            modified_at: Some(time),
        }];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "ignored.md".into(),
            content_hash: "old_hash".into(),
            file_size: 100,
            status: DocumentStatus::Excluded,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let hasher_called = Cell::new(false);
        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| {
                hasher_called.set(true);
                Ok("should_not_run".into())
            },
        );

        assert!(!hasher_called.get());
        assert!(plan.to_add.is_empty());
        assert!(plan.to_update.is_empty());
        assert_eq!(plan.to_skip.len(), 1);
        assert_eq!(plan.to_skip[0].reason, PlanSkipReason::UserExcluded);
    }

    #[test]
    fn test_user_excluded_file_missing_on_disk_preserves_exclusion() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let doc_id = DocumentId::from_relative_path(folder_id, "ghost.md");

        let inventory = vec![];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "ghost.md".into(),
            content_hash: "hash_ghost".into(),
            file_size: 200,
            status: DocumentStatus::Excluded,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("unused".into()),
        );

        assert!(plan.to_delete.is_empty(), "Must not delete excluded file");
        assert_eq!(plan.to_skip.len(), 1);
        assert_eq!(plan.to_skip[0].reason, PlanSkipReason::PreservedExcluded);
        assert!(!plan.has_changes());
    }

    #[test]
    fn test_move_rename_detection_phase_two() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let old_id = DocumentId::from_relative_path(folder_id, "old_name.rs");
        let new_id = DocumentId::from_relative_path(folder_id, "new_name.rs");

        // Old file deleted, new file added with identical content hash
        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("new_name.rs"),
            absolute_path: PathBuf::from("/work/new_name.rs"),
            file_size: 300,
            modified_at: Some(time),
        }];

        let db_entries = vec![RegistryEntry {
            id: old_id,
            folder_id,
            relative_path: "old_name.rs".into(),
            content_hash: "matching_sha256".into(),
            file_size: 300,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("matching_sha256".into()),
        );

        assert!(plan.to_add.is_empty());
        assert!(plan.to_delete.is_empty());
        assert_eq!(plan.to_move.len(), 1);

        let m = &plan.to_move[0];
        assert_eq!(m.old_id, old_id);
        assert_eq!(m.new_id, new_id);
        assert_eq!(m.old_path, "old_name.rs");
        assert_eq!(m.new_path, PathBuf::from("new_name.rs"));
        assert_eq!(m.content_hash, "matching_sha256");
        assert_eq!(m.file_size, 300);
    }

    #[test]
    fn test_exceeded_max_file_size_skipped() {
        let folder_id = FolderId::new();
        let time = sample_time();

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("huge.bin"),
            absolute_path: PathBuf::from("/work/huge.bin"),
            file_size: 5 * 1024 * 1024, // 5 MB
            modified_at: Some(time),
        }];

        let db_entries = vec![];

        let hasher_called = Cell::new(false);
        let plan = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(2 * 1024 * 1024), // 2 MB limit
            |_| {
                hasher_called.set(true);
                Ok("hash".into())
            },
        );

        assert!(!hasher_called.get());
        assert!(plan.to_add.is_empty());
        assert_eq!(plan.to_skip.len(), 1);
        assert_eq!(plan.to_skip[0].reason, PlanSkipReason::ExceededMaxSize);
    }

    #[test]
    fn test_idempotence_repeated_scan() {
        let folder_id = FolderId::new();
        let time = sample_time();
        let doc_id = DocumentId::from_relative_path(folder_id, "doc.md");

        let inventory = vec![DiscoveredFile {
            relative_path: PathBuf::from("doc.md"),
            absolute_path: PathBuf::from("/work/doc.md"),
            file_size: 100,
            modified_at: Some(time),
        }];

        let db_entries = vec![RegistryEntry {
            id: doc_id,
            folder_id,
            relative_path: "doc.md".into(),
            content_hash: "hash_doc".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: time,
            updated_at: time,
        }];

        let plan1 = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("hash_doc".into()),
        );
        let plan2 = ScanPlanner::plan(
            folder_id,
            &inventory,
            &db_entries,
            &make_test_settings(1024 * 1024),
            |_| Ok("hash_doc".into()),
        );

        assert_eq!(plan1, plan2);
        assert!(!plan1.has_changes());
        assert!(!plan2.has_changes());
    }
}
