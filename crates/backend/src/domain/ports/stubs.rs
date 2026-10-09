use super::{
    DocumentRegistryRepository, FolderRepository, JobRepository, SearchRepository,
    SettingsRepository,
    file_system::{DiscoveredFile, FilePayload, FileReader, FileWalker, ReadOptions, WalkOptions},
};
use crate::config::Bm25Weights;
use crate::domain::models::{
    AppSettings, BulkIndexReport, DocumentId, DocumentStatus, Folder, FolderId, FolderStatus,
    IndexedDocument, IndexingJob, JobId, JobProgressUpdate, JobStatus, RegistryEntry,
    SearchRawResponse,
};
use crate::error::AppError;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct InMemoryFolderRepository {
    folders: Arc<RwLock<HashMap<FolderId, Folder>>>,
    document_counts: Arc<RwLock<HashMap<FolderId, u64>>>,
}

impl InMemoryFolderRepository {
    pub fn set_document_count(&self, id: FolderId, count: u64) {
        self.document_counts.write().insert(id, count);
    }
}

#[async_trait]
impl FolderRepository for InMemoryFolderRepository {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn create_folder(&self, folder: &Folder) -> Result<(), AppError> {
        self.folders.write().insert(folder.id, folder.clone());
        Ok(())
    }

    async fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, AppError> {
        Ok(self.folders.read().get(id).cloned())
    }

    async fn find_by_path(&self, path: &Path) -> Result<Option<Folder>, AppError> {
        Ok(self
            .folders
            .read()
            .values()
            .find(|f| f.path == path)
            .cloned())
    }

    async fn list_folders(&self) -> Result<Vec<Folder>, AppError> {
        let mut list: Vec<Folder> = self.folders.read().values().cloned().collect();
        list.sort_by_key(|f| f.created_at);
        Ok(list)
    }

    async fn list_folders_with_counts(&self) -> Result<Vec<(Folder, u64)>, AppError> {
        let folders = self.folders.read();
        let counts = self.document_counts.read();
        let mut list: Vec<(Folder, u64)> = folders
            .values()
            .map(|f| {
                let count = counts.get(&f.id).copied().unwrap_or(0);
                (f.clone(), count)
            })
            .collect();
        list.sort_by_key(|(f, _)| f.created_at);
        Ok(list)
    }

    async fn delete_folder(&self, id: &FolderId) -> Result<(), AppError> {
        self.folders.write().remove(id);
        self.document_counts.write().remove(id);
        Ok(())
    }

    async fn update_last_scanned(&self, id: &FolderId) -> Result<(), AppError> {
        if let Some(folder) = self.folders.write().get_mut(id) {
            folder.last_scanned_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn update_status(&self, id: &FolderId, status: FolderStatus) -> Result<(), AppError> {
        if let Some(folder) = self.folders.write().get_mut(id) {
            folder.status = status;
        }
        Ok(())
    }

    async fn reset_scanning_folders(&self) -> Result<u64, AppError> {
        let mut count = 0;
        let mut folders = self.folders.write();
        for folder in folders.values_mut() {
            if folder.status == FolderStatus::Scanning {
                folder.status = FolderStatus::Idle;
                count += 1;
            }
        }
        Ok(count)
    }
}

#[derive(Default)]
pub struct InMemoryDocumentRegistryRepository {
    entries: Arc<RwLock<HashMap<DocumentId, RegistryEntry>>>,
}

#[async_trait]
impl DocumentRegistryRepository for InMemoryDocumentRegistryRepository {
    async fn get_entry(&self, doc_id: &DocumentId) -> Result<Option<RegistryEntry>, AppError> {
        Ok(self.entries.read().get(doc_id).cloned())
    }

    async fn list_by_folder(&self, folder_id: &FolderId) -> Result<Vec<RegistryEntry>, AppError> {
        let entries = self
            .entries
            .read()
            .values()
            .filter(|e| e.folder_id == *folder_id)
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn list_by_status(&self, status: DocumentStatus) -> Result<Vec<RegistryEntry>, AppError> {
        let entries = self
            .entries
            .read()
            .values()
            .filter(|e| e.status == status)
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), AppError> {
        self.entries.write().insert(entry.id, entry.clone());
        Ok(())
    }

    async fn upsert_batch(&self, entries: &[RegistryEntry]) -> Result<u64, AppError> {
        let mut count = 0;
        let mut store = self.entries.write();
        for entry in entries {
            store.insert(entry.id, entry.clone());
            count += 1;
        }
        Ok(count)
    }

    async fn delete_entries(&self, ids: &[DocumentId]) -> Result<u64, AppError> {
        let mut count = 0;
        let mut entries = self.entries.write();
        for id in ids {
            if entries.remove(id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }
}

#[derive(Default)]
pub struct InMemoryJobRepository {
    jobs: Arc<RwLock<HashMap<JobId, IndexingJob>>>,
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), AppError> {
        self.jobs.write().insert(job.id, job.clone());
        Ok(())
    }

    async fn update_progress(
        &self,
        id: &JobId,
        update: &JobProgressUpdate,
    ) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id) {
            if let Some(status) = update.status
                && (!job.status.is_terminal() || job.status == status)
            {
                job.status = status;
            }
            if let Some(total) = update.files_total {
                job.files_total = total;
            }
            job.files_indexed += update.indexed_delta;
            job.files_skipped += update.skipped_delta;
            job.files_failed += update.failed_delta;
            job.files_processed += update.processed_delta;
            let sum_sub = job.files_indexed + job.files_skipped + job.files_failed;
            if sum_sub > job.files_processed {
                job.files_processed = sum_sub;
            }
            if update.error_summary.is_some() {
                job.error_summary = update.error_summary.clone();
            }
            if job.status.is_terminal() && job.completed_at.is_none() {
                job.completed_at = Some(chrono::Utc::now());
            }
        }
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<IndexingJob>, AppError> {
        Ok(self.jobs.read().get(id).cloned())
    }

    async fn mark_cancelled(&self, id: &JobId) -> Result<(), AppError> {
        if let Some(job) = self.jobs.write().get_mut(id)
            && !job.status.is_terminal()
        {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn find_dangling_jobs(&self) -> Result<Vec<IndexingJob>, AppError> {
        let dangling = self
            .jobs
            .read()
            .values()
            .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Pending)
            .cloned()
            .collect();
        Ok(dangling)
    }

    async fn recover_dangling_jobs(&self, error_summary: &str) -> Result<u64, AppError> {
        let mut count = 0;
        let mut jobs = self.jobs.write();
        let now = chrono::Utc::now();
        for job in jobs.values_mut() {
            if job.status == JobStatus::Running || job.status == JobStatus::Pending {
                job.status = JobStatus::Failed;
                job.completed_at = Some(now);
                job.error_summary = Some(error_summary.to_string());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn cancel_unfinished_jobs(&self, reason: &str) -> Result<u64, AppError> {
        let mut count = 0;
        let mut jobs = self.jobs.write();
        let now = chrono::Utc::now();
        for job in jobs.values_mut() {
            if job.status == JobStatus::Running || job.status == JobStatus::Pending {
                job.status = JobStatus::Cancelled;
                job.completed_at = Some(now);
                job.error_summary = Some(reason.to_string());
                count += 1;
            }
        }
        Ok(count)
    }
}

pub struct InMemorySettingsRepository {
    settings: Arc<RwLock<AppSettings>>,
}

impl Default for InMemorySettingsRepository {
    fn default() -> Self {
        Self {
            settings: Arc::new(RwLock::new(AppSettings {
                max_file_size_bytes: 2 * 1024 * 1024,
                weights: Bm25Weights {
                    title: 3.0,
                    tags: 2.0,
                    content: 1.0,
                },
                ignore_patterns: vec![
                    ".git".into(),
                    "node_modules".into(),
                    "target".into(),
                    "dist".into(),
                    "build".into(),
                ],
            })),
        }
    }
}

impl InMemorySettingsRepository {
    pub fn new(initial: AppSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(initial)),
        }
    }
}

#[async_trait]
impl SettingsRepository for InMemorySettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings, AppError> {
        Ok(self.settings.read().clone())
    }

    async fn update_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
        *self.settings.write() = settings.clone();
        Ok(())
    }

    async fn reset_settings(&self) -> Result<AppSettings, AppError> {
        let default_settings = AppSettings::default();
        *self.settings.write() = default_settings.clone();
        Ok(default_settings)
    }
}

pub(crate) fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let m = s1_chars.len();
    let n = s2_chars.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row = vec![0; n + 1];

    for i in 1..=m {
        curr_row[0] = i;
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[n]
}

pub(crate) fn score_term_in_field(field_text: &str, term_low: &str, base_weight: f32) -> f32 {
    let term_len = term_low.chars().count();
    let max_edits = if term_len <= 2 {
        0
    } else if term_len <= 5 {
        1
    } else {
        2
    };

    let words: Vec<&str> = field_text
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| !w.is_empty())
        .collect();

    let mut matched_exact = false;
    let mut matched_prefix = false;
    let mut matched_fuzzy = false;

    for w in words {
        if w == term_low {
            matched_exact = true;
        } else if w.starts_with(term_low) {
            matched_prefix = true;
        } else if max_edits > 0 && levenshtein_distance(w, term_low) <= max_edits {
            matched_fuzzy = true;
        }
    }

    if matched_exact {
        base_weight * 1.8
    } else if matched_prefix {
        base_weight * 0.8
    } else if matched_fuzzy {
        base_weight * 0.5
    } else if field_text.contains(term_low) {
        base_weight * 0.8
    } else {
        0.0
    }
}

#[derive(Default)]
pub struct InMemorySearchRepository {
    documents: Arc<RwLock<HashMap<DocumentId, IndexedDocument>>>,
    index_documents: Arc<RwLock<HashMap<String, HashMap<DocumentId, IndexedDocument>>>>,
    active_index: Arc<RwLock<Option<String>>>,
    indices: Arc<RwLock<HashSet<String>>>,
}

#[async_trait]
impl SearchRepository for InMemorySearchRepository {
    fn search_alias(&self) -> &str {
        "lynx_documents"
    }

    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError> {
        self.documents.write().insert(doc.id, doc.clone());
        let active = self
            .active_index
            .read()
            .clone()
            .unwrap_or_else(|| "lynx_documents_v1".to_string());
        self.index_documents
            .write()
            .entry(active)
            .or_default()
            .insert(doc.id, doc.clone());
        Ok(())
    }

    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        let active = self
            .active_index
            .read()
            .clone()
            .unwrap_or_else(|| self.search_alias().to_string());
        self.bulk_index_to_target(&active, docs).await
    }

    async fn bulk_index_to_target(
        &self,
        target_index: &str,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        let mut idx_docs = self.index_documents.write();
        let target_map = idx_docs.entry(target_index.to_string()).or_default();
        for doc in docs {
            target_map.insert(doc.id, doc.clone());
        }
        self.indices.write().insert(target_index.to_string());

        let active = self.active_index.read().clone();
        if target_index == self.search_alias() || active.as_deref() == Some(target_index) {
            let mut write = self.documents.write();
            for doc in docs {
                write.insert(doc.id, doc.clone());
            }
        }

        Ok(BulkIndexReport {
            indexed: docs.len(),
            failed: 0,
            errors: Vec::new(),
        })
    }

    async fn delete_document(&self, id: &DocumentId) -> Result<(), AppError> {
        self.documents.write().remove(id);
        if let Some(ref active) = *self.active_index.read()
            && let Some(map) = self.index_documents.write().get_mut(active)
        {
            map.remove(id);
        }
        Ok(())
    }

    async fn delete_documents_by_folder(&self, folder_id: &FolderId) -> Result<u64, AppError> {
        let mut write = self.documents.write();
        let initial_len = write.len();
        write.retain(|_, doc| doc.folder_id != *folder_id);
        let removed = initial_len - write.len();
        if let Some(ref active) = *self.active_index.read()
            && let Some(map) = self.index_documents.write().get_mut(active)
        {
            map.retain(|_, doc| doc.folder_id != *folder_id);
        }
        Ok(removed as u64)
    }

    async fn search(
        &self,
        query_dsl: &serde_json::value::RawValue,
    ) -> Result<SearchRawResponse, AppError> {
        let dsl: serde_json::Value = serde_json::from_str(query_dsl.get()).unwrap_or_default();
        let docs = self.documents.read();

        let from = dsl.get("from").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let size = dsl.get("size").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

        // 1. Extract query clauses
        let query_obj = dsl.get("query");
        let is_match_all = query_obj.and_then(|q| q.get("match_all")).is_some();

        let bool_obj = query_obj.and_then(|q| q.get("bool"));

        // Extract filters from post_filter (or bool.filter fallback)
        let mut filters: Vec<(String, Vec<String>)> = Vec::new();
        let parse_filter_node = |f: &serde_json::Value, target: &mut Vec<(String, Vec<String>)>| {
            if let Some(term_map) = f.get("term").and_then(|t| t.as_object()) {
                for (k, v) in term_map {
                    if let Some(v_str) = v.as_str() {
                        target.push((k.clone(), vec![v_str.to_string()]));
                    }
                }
            }
            if let Some(terms_map) = f.get("terms").and_then(|t| t.as_object()) {
                for (k, v) in terms_map {
                    if let Some(arr) = v.as_array() {
                        let vals: Vec<String> = arr
                            .iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect();
                        target.push((k.clone(), vals));
                    }
                }
            }
        };

        if let Some(post_filter) = dsl.get("post_filter") {
            if let Some(filter_arr) = post_filter
                .get("bool")
                .and_then(|b| b.get("filter"))
                .and_then(|f| f.as_array())
            {
                for f in filter_arr {
                    parse_filter_node(f, &mut filters);
                }
            } else {
                parse_filter_node(post_filter, &mut filters);
            }
        } else if let Some(filter_arr) = bool_obj
            .and_then(|b| b.get("filter"))
            .and_then(|f| f.as_array())
        {
            for f in filter_arr {
                parse_filter_node(f, &mut filters);
            }
        }

        // Extract must queries: phrase vs free terms
        let mut phrase_terms: Vec<String> = Vec::new();
        let mut free_terms: Vec<String> = Vec::new();
        if let Some(must_arr) = bool_obj
            .and_then(|b| b.get("must"))
            .and_then(|m| m.as_array())
        {
            for m in must_arr {
                if let Some(mm) = m.get("multi_match") {
                    let match_type = mm
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("best_fields");
                    if let Some(q_str) = mm.get("query").and_then(|q| q.as_str()) {
                        if match_type == "phrase" {
                            phrase_terms.push(q_str.to_string());
                        } else {
                            for word in q_str.split_whitespace() {
                                if !free_terms.contains(&word.to_string()) {
                                    free_terms.push(word.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Extract should queries: free terms (exact, prefix, fuzzy)
        if let Some(should_arr) = bool_obj
            .and_then(|b| b.get("should"))
            .and_then(|s| s.as_array())
        {
            for s in should_arr {
                if let Some(mm) = s.get("multi_match")
                    && let Some(q_str) = mm.get("query").and_then(|q| q.as_str())
                {
                    for word in q_str.split_whitespace() {
                        let w = word.to_string();
                        if !free_terms.contains(&w) {
                            free_terms.push(w);
                        }
                    }
                }
            }
        }

        let has_highlight = dsl.get("highlight").is_some();

        // 2. Score documents matching the query (for both aggregations and post-filtered hits)
        let mut query_matched_docs: Vec<(f32, IndexedDocument, HashMap<String, Vec<String>>)> =
            Vec::new();

        for doc in docs.values() {
            let mut score = 0.0f32;
            let mut matched_query = false;

            if is_match_all || (phrase_terms.is_empty() && free_terms.is_empty()) {
                score = 1.0;
                matched_query = true;
            } else {
                let title_lower = doc.title.to_lowercase();
                let content_lower = doc.content.to_lowercase();
                let tags_lower: Vec<String> = doc.tags.iter().map(|t| t.to_lowercase()).collect();

                let mut phrases_matched = true;
                for phrase in &phrase_terms {
                    let phrase_low = phrase.to_lowercase();
                    let in_title = title_lower.contains(&phrase_low);
                    let in_content = content_lower.contains(&phrase_low);
                    let in_tags = tags_lower.iter().any(|t| t.contains(&phrase_low));

                    if in_title || in_content || in_tags {
                        if in_title {
                            score += 5.0;
                        }
                        if in_tags {
                            score += 2.0;
                        }
                        if in_content {
                            score += 1.0;
                        }
                    } else {
                        phrases_matched = false;
                        break;
                    }
                }

                if !phrase_terms.is_empty() && !phrases_matched {
                    continue;
                }

                if !free_terms.is_empty() {
                    let mut terms_matched = 0;
                    for term in &free_terms {
                        let term_low = term.to_lowercase();
                        let title_s = score_term_in_field(&title_lower, &term_low, 5.0);
                        let tags_s = tags_lower
                            .iter()
                            .map(|t| score_term_in_field(t, &term_low, 2.0))
                            .fold(0.0f32, f32::max);
                        let content_s = score_term_in_field(&content_lower, &term_low, 1.0);

                        let term_total = title_s + tags_s + content_s;
                        if term_total > 0.0 {
                            terms_matched += 1;
                            score += term_total;
                        }
                    }
                    if terms_matched > 0 {
                        matched_query = true;
                    }
                } else if phrases_matched {
                    matched_query = true;
                }
            }

            if !matched_query {
                continue;
            }

            let mut highlights: HashMap<String, Vec<String>> = HashMap::new();
            if has_highlight && (!phrase_terms.is_empty() || !free_terms.is_empty()) {
                let mut all_match_words = phrase_terms.clone();
                all_match_words.extend(free_terms.clone());

                let mut content_snippets = Vec::new();
                for word in &all_match_words {
                    if word.is_empty() {
                        continue;
                    }
                    let word_low = word.to_lowercase();
                    let content = &doc.content;
                    let content_low = content.to_lowercase();

                    let token_to_find = if content_low.contains(&word_low) {
                        Some(word_low.clone())
                    } else {
                        let w_len = word_low.chars().count();
                        let max_edits = if w_len <= 2 {
                            0
                        } else if w_len <= 5 {
                            1
                        } else {
                            2
                        };
                        if max_edits > 0 {
                            content_low
                                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                                .filter(|w| !w.is_empty())
                                .find(|w| levenshtein_distance(w, &word_low) <= max_edits)
                                .map(|w| w.to_string())
                        } else {
                            None
                        }
                    };

                    let Some(highlight_token) = token_to_find else {
                        continue;
                    };

                    let mut search_from = 0;
                    while let Some(idx) = content_low[search_from..].find(&highlight_token) {
                        let abs_idx = search_from + idx;
                        let line_start = content[..abs_idx].rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let line_end = content[abs_idx..]
                            .find('\n')
                            .map(|p| abs_idx + p)
                            .unwrap_or(content.len());
                        let line_snippet = &content[line_start..line_end];

                        let match_in_line = abs_idx - line_start;
                        let prefix = &line_snippet[..match_in_line];
                        let matched_orig =
                            &line_snippet[match_in_line..match_in_line + highlight_token.len()];
                        let suffix = &line_snippet[match_in_line + highlight_token.len()..];
                        let snippet = format!("{prefix}<em>{matched_orig}</em>{suffix}");

                        if !content_snippets.contains(&snippet) {
                            content_snippets.push(snippet);
                        }
                        search_from = abs_idx + highlight_token.len();
                        if content_snippets.len() >= 5 {
                            break;
                        }
                    }
                }
                if !content_snippets.is_empty() {
                    highlights.insert("content".to_string(), content_snippets);
                }

                let mut title_snippets = Vec::new();
                for word in &all_match_words {
                    if word.is_empty() {
                        continue;
                    }
                    let word_low = word.to_lowercase();
                    let title = &doc.title;
                    let title_low = title.to_lowercase();

                    let token_to_find = if title_low.contains(&word_low) {
                        Some(word_low.clone())
                    } else {
                        let w_len = word_low.chars().count();
                        let max_edits = if w_len <= 2 {
                            0
                        } else if w_len <= 5 {
                            1
                        } else {
                            2
                        };
                        if max_edits > 0 {
                            title_low
                                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                                .filter(|w| !w.is_empty())
                                .find(|w| levenshtein_distance(w, &word_low) <= max_edits)
                                .map(|w| w.to_string())
                        } else {
                            None
                        }
                    };

                    let Some(highlight_token) = token_to_find else {
                        continue;
                    };

                    if let Some(idx) = title_low.find(&highlight_token) {
                        let prefix = &title[..idx];
                        let matched_orig = &title[idx..idx + highlight_token.len()];
                        let suffix = &title[idx + highlight_token.len()..];
                        let snippet = format!("{prefix}<em>{matched_orig}</em>{suffix}");
                        if !title_snippets.contains(&snippet) {
                            title_snippets.push(snippet);
                        }
                    }
                }
                if !title_snippets.is_empty() {
                    highlights.insert("title".to_string(), title_snippets);
                }
            }

            query_matched_docs.push((score, doc.clone(), highlights));
        }

        // Hitung agregasi (aggs) terhadap seluruh query_matched_docs (sesuai post_filter semantics)
        let mut ext_counts: HashMap<String, u64> = HashMap::new();
        let mut type_counts: HashMap<String, u64> = HashMap::new();
        let mut lang_counts: HashMap<String, u64> = HashMap::new();
        let mut proj_counts: HashMap<String, u64> = HashMap::new();
        let mut tag_counts: HashMap<String, u64> = HashMap::new();

        for (_, doc, _) in &query_matched_docs {
            if let Some(ext) = &doc.extension {
                *ext_counts.entry(ext.clone()).or_insert(0) += 1;
            }
            *type_counts
                .entry(doc.doc_type.as_str().to_string())
                .or_insert(0) += 1;
            if let Some(lang) = &doc.language {
                *lang_counts.entry(lang.as_str().to_string()).or_insert(0) += 1;
            }
            if let Some(proj) = &doc.project {
                *proj_counts.entry(proj.clone()).or_insert(0) += 1;
            }
            for tag in &doc.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        // Terapkan post_filter untuk mendapatkan scored_docs (hits)
        let mut scored_docs: Vec<(f32, IndexedDocument, HashMap<String, Vec<String>>)> = Vec::new();
        for (score, doc, hl) in query_matched_docs {
            let mut filter_passed = true;
            for (field, values) in &filters {
                let matched = match field.as_str() {
                    "tags" => doc
                        .tags
                        .iter()
                        .any(|t| values.iter().any(|v| t.eq_ignore_ascii_case(v))),
                    "language" => doc
                        .language
                        .as_ref()
                        .map(|l| l.as_str())
                        .map(|l| values.iter().any(|v| l.eq_ignore_ascii_case(v)))
                        .unwrap_or(false),
                    "type" => values
                        .iter()
                        .any(|v| doc.doc_type.as_str().eq_ignore_ascii_case(v)),
                    "project" => doc
                        .project
                        .as_deref()
                        .map(|p| values.iter().any(|v| p.eq_ignore_ascii_case(v)))
                        .unwrap_or(false),
                    "extension" => doc
                        .extension
                        .as_deref()
                        .map(|e| values.iter().any(|v| e.eq_ignore_ascii_case(v)))
                        .unwrap_or(false),
                    _ => true,
                };
                if !matched {
                    filter_passed = false;
                    break;
                }
            }
            if filter_passed {
                scored_docs.push((score, doc, hl));
            }
        }

        // 3. Sorting
        let sort_val = dsl.get("sort");
        if let Some(sort_arr) = sort_val.and_then(|s| s.as_array()) {
            let mut sort_by_size = false;
            let mut sort_by_date = false;
            let mut is_desc = true;

            for item in sort_arr {
                if let Some(size_obj) = item.get("file_size_bytes") {
                    sort_by_size = true;
                    is_desc = size_obj.get("order").and_then(|o| o.as_str()) != Some("asc");
                    break;
                }
                if let Some(date_obj) = item.get("modified_at") {
                    sort_by_date = true;
                    is_desc = date_obj.get("order").and_then(|o| o.as_str()) != Some("asc");
                    break;
                }
            }

            if sort_by_size {
                scored_docs.sort_by(|a, b| {
                    if is_desc {
                        b.1.file_size_bytes.cmp(&a.1.file_size_bytes)
                    } else {
                        a.1.file_size_bytes.cmp(&b.1.file_size_bytes)
                    }
                });
            } else if sort_by_date {
                scored_docs.sort_by(|a, b| {
                    if is_desc {
                        b.1.modified_at.cmp(&a.1.modified_at)
                    } else {
                        a.1.modified_at.cmp(&b.1.modified_at)
                    }
                });
            } else {
                scored_docs
                    .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            }
        } else {
            scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        let total = scored_docs.len();

        // 4. Pagination
        let paged: Vec<serde_json::Value> = scored_docs
            .iter()
            .skip(from)
            .take(size)
            .map(|(score, doc, hl)| {
                let mut hit_obj = serde_json::json!({
                    "_id": doc.id.to_string(),
                    "_score": score,
                    "_source": serde_json::to_value(doc).unwrap_or_default()
                });
                if !hl.is_empty() {
                    hit_obj["highlight"] = serde_json::to_value(hl).unwrap_or_default();
                }
                hit_obj
            })
            .collect();

        let to_bucket_list = |counts: HashMap<String, u64>| -> serde_json::Value {
            let mut list: Vec<serde_json::Value> = counts
                .into_iter()
                .map(|(key, doc_count)| serde_json::json!({ "key": key, "doc_count": doc_count }))
                .collect();
            list.sort_by(|a, b| {
                let ca = a["doc_count"].as_u64().unwrap_or(0);
                let cb = b["doc_count"].as_u64().unwrap_or(0);
                cb.cmp(&ca).then_with(|| {
                    let ka = a["key"].as_str().unwrap_or("");
                    let kb = b["key"].as_str().unwrap_or("");
                    ka.cmp(kb)
                })
            });
            serde_json::json!({ "buckets": list })
        };

        let mut res_obj = serde_json::json!({
            "took": 1,
            "hits": {
                "total": { "value": total },
                "hits": paged
            }
        });

        if dsl.get("aggs").is_some() {
            let aggs_val = serde_json::json!({
                "extensions": to_bucket_list(ext_counts),
                "types": to_bucket_list(type_counts),
                "languages": to_bucket_list(lang_counts),
                "projects": to_bucket_list(proj_counts),
                "tags": to_bucket_list(tag_counts),
            });
            res_obj["aggregations"] = aggs_val;
        }

        let json_str = res_obj.to_string();

        Ok(SearchRawResponse {
            raw_json: json_str,
            took_ms: 1,
        })
    }

    async fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>, AppError> {
        let prefix_lower = prefix.trim().to_lowercase();
        if prefix_lower.is_empty() {
            return Ok(Vec::new());
        }
        let docs = self.documents.read();
        let mut suggestions = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for doc in docs.values() {
            let t = doc.title.trim();
            if t.to_lowercase().contains(&prefix_lower) && seen.insert(t.to_lowercase()) {
                suggestions.push(t.to_string());
                if suggestions.len() >= limit {
                    break;
                }
            }
        }
        suggestions.sort();
        Ok(suggestions)
    }

    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), AppError> {
        let old_indices = self.get_alias_indices(alias).await?;
        self.swap_alias(alias, &old_indices, new_index).await?;
        let current_indices = self.get_alias_indices(alias).await?;
        if !current_indices.contains(&new_index.to_string()) {
            return Err(AppError::SearchEngine(format!(
                "Alias swap verification failed: alias '{alias}' does not point to '{new_index}'"
            )));
        }
        for old in &old_indices {
            if old != new_index {
                self.delete_index(old).await?;
            }
        }
        Ok(())
    }

    async fn ping(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn ensure_initial_index(&self) -> Result<String, AppError> {
        let mut active = self.active_index.write();
        if let Some(ref current) = *active {
            Ok(current.clone())
        } else {
            let initial = "lynx_documents_v1".to_string();
            *active = Some(initial.clone());
            self.indices.write().insert(initial.clone());
            self.index_documents
                .write()
                .entry(initial.clone())
                .or_default();
            Ok(initial)
        }
    }

    async fn get_active_physical_index(&self) -> Result<Option<String>, AppError> {
        Ok(self.active_index.read().clone())
    }

    async fn create_versioned_index(&self, version: u32) -> Result<String, AppError> {
        let name = format!("lynx_documents_v{version}");
        self.indices.write().insert(name.clone());
        self.index_documents
            .write()
            .entry(name.clone())
            .or_default();
        Ok(name)
    }

    async fn get_alias_indices(&self, _alias: &str) -> Result<Vec<String>, AppError> {
        Ok(self.active_index.read().clone().into_iter().collect())
    }

    async fn swap_alias(
        &self,
        _alias: &str,
        _old_indices: &[String],
        new_index: &str,
    ) -> Result<(), AppError> {
        self.indices.write().insert(new_index.to_string());
        *self.active_index.write() = Some(new_index.to_string());
        let target_docs = self
            .index_documents
            .read()
            .get(new_index)
            .cloned()
            .unwrap_or_default();
        *self.documents.write() = target_docs;
        Ok(())
    }

    async fn delete_index(&self, index_name: &str) -> Result<(), AppError> {
        self.indices.write().remove(index_name);
        self.index_documents.write().remove(index_name);
        let mut active = self.active_index.write();
        if active.as_deref() == Some(index_name) {
            *active = None;
            self.documents.write().clear();
        }
        Ok(())
    }
}

/// Stub in-memory untuk FileWalker untuk keperluan domain & usecase testing tanpa akses filesystem fisik.
#[derive(Default, Clone)]
pub struct InMemoryFileWalker {
    files: Arc<RwLock<Vec<DiscoveredFile>>>,
}

impl InMemoryFileWalker {
    pub fn new(files: Vec<DiscoveredFile>) -> Self {
        Self {
            files: Arc::new(RwLock::new(files)),
        }
    }

    pub fn add_file(&self, file: DiscoveredFile) {
        self.files.write().push(file);
    }
}

impl FileWalker for InMemoryFileWalker {
    fn walk<'a>(
        &'a self,
        _root: &'a Path,
        _options: &'a WalkOptions,
    ) -> Result<Box<dyn Iterator<Item = Result<DiscoveredFile, AppError>> + Send + 'a>, AppError>
    {
        let files = self.files.read().clone();
        Ok(Box::new(files.into_iter().map(Ok)))
    }
}

/// Stub in-memory untuk FileReader untuk keperluan domain & usecase testing tanpa akses filesystem fisik.
#[derive(Default, Clone)]
pub struct InMemoryFileReader {
    files: Arc<RwLock<HashMap<std::path::PathBuf, FilePayload>>>,
}

impl InMemoryFileReader {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_payload(&self, payload: FilePayload) {
        self.files
            .write()
            .insert(payload.absolute_path.clone(), payload);
    }

    pub fn add_file(
        &self,
        absolute_path: impl Into<std::path::PathBuf>,
        relative_path: Option<impl Into<std::path::PathBuf>>,
        bytes: Vec<u8>,
    ) {
        let abs = absolute_path.into();
        let rel = relative_path.map(|r| r.into());
        let payload = FilePayload::new(abs.clone(), rel, bytes, Some(chrono::Utc::now()));
        self.files.write().insert(abs, payload);
    }
}

#[async_trait]
impl FileReader for InMemoryFileReader {
    async fn read_file(&self, path: &Path, options: &ReadOptions) -> Result<FilePayload, AppError> {
        let files = self.files.read();
        let payload = files.get(path).cloned().ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found in in-memory reader: {}", path.display()),
            ))
        })?;

        if let Some(limit) = options.max_file_size_bytes
            && payload.size > limit
        {
            return Err(AppError::ValidationFailed(format!(
                "File '{}' exceeds maximum allowed size: {} > {} bytes",
                path.display(),
                payload.size,
                limit
            )));
        }

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{DocumentId, DocumentStatus, FolderId, RegistryEntry};
    use crate::domain::ports::compute_sha256;
    use chrono::Utc;

    #[tokio::test]
    async fn test_in_memory_registry_upsert_batch() {
        let repo = InMemoryDocumentRegistryRepository::default();
        let folder_id = FolderId::new();
        let doc1 = RegistryEntry {
            id: DocumentId::from_relative_path(folder_id, "file1.rs"),
            folder_id,
            relative_path: "file1.rs".into(),
            content_hash: "hash1".into(),
            file_size: 100,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let doc2 = RegistryEntry {
            id: DocumentId::from_relative_path(folder_id, "file2.rs"),
            folder_id,
            relative_path: "file2.rs".into(),
            content_hash: "hash2".into(),
            file_size: 200,
            status: DocumentStatus::Indexed,
            status_reason: None,
            indexed_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let inserted = repo
            .upsert_batch(&[doc1.clone(), doc2.clone()])
            .await
            .unwrap();
        assert_eq!(inserted, 2);

        let list = repo.list_by_folder(&folder_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_settings_reset() {
        let repo = InMemorySettingsRepository::default();
        let mut modified = repo.get_settings().await.unwrap();
        modified.max_file_size_bytes = 999999;
        repo.update_settings(&modified).await.unwrap();

        let current = repo.get_settings().await.unwrap();
        assert_eq!(current.max_file_size_bytes, 999999);

        let reset = repo.reset_settings().await.unwrap();
        assert_eq!(reset.max_file_size_bytes, 2 * 1024 * 1024);

        let after_reset = repo.get_settings().await.unwrap();
        assert_eq!(after_reset.max_file_size_bytes, 2 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_in_memory_search_repo_initial_index_and_active_index() {
        let repo = InMemorySearchRepository::default();
        assert_eq!(repo.get_active_physical_index().await.unwrap(), None);

        let initial = repo.ensure_initial_index().await.unwrap();
        assert_eq!(initial, "lynx_documents_v1");
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v1".to_string())
        );

        // Subsequent ensure call should return the already active index (idempotent)
        let second = repo.ensure_initial_index().await.unwrap();
        assert_eq!(second, "lynx_documents_v1");

        // Simulating rebuild swap
        repo.rebuild_index_with_alias("lynx_documents_v2", "lynx_documents")
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v2".to_string())
        );
    }

    #[tokio::test]
    async fn test_in_memory_search_repo_reindex_alias_contract() {
        let repo = InMemorySearchRepository::default();

        // 1. Initial index creation
        let v1 = repo.ensure_initial_index().await.unwrap();
        assert_eq!(v1, "lynx_documents_v1");

        // 2. Create versioned index
        let v2 = repo.create_versioned_index(2).await.unwrap();
        assert_eq!(v2, "lynx_documents_v2");

        // 3. Inspect alias
        let indices = repo.get_alias_indices("lynx_documents").await.unwrap();
        assert_eq!(indices, vec!["lynx_documents_v1".to_string()]);

        // 4. Swap alias
        repo.swap_alias("lynx_documents", &indices, &v2)
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v2".to_string())
        );

        // 5. Cleanup old index
        repo.delete_index(&v1).await.unwrap();

        // 6. Test rebuild_index_with_alias helper
        let v3 = repo.create_versioned_index(3).await.unwrap();
        repo.rebuild_index_with_alias(&v3, "lynx_documents")
            .await
            .unwrap();
        assert_eq!(
            repo.get_active_physical_index().await.unwrap(),
            Some("lynx_documents_v3".to_string())
        );
    }

    #[test]
    fn test_in_memory_file_walker() {
        let walker = InMemoryFileWalker::default();
        walker.add_file(DiscoveredFile {
            relative_path: "src/main.rs".into(),
            absolute_path: "/project/src/main.rs".into(),
            file_size: 1024,
            modified_at: Some(Utc::now()),
        });

        let files = walker
            .walk_all(Path::new("/project"), &WalkOptions::default())
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].relative_path,
            std::path::PathBuf::from("src/main.rs")
        );
    }

    #[tokio::test]
    async fn test_in_memory_file_reader() {
        let reader = InMemoryFileReader::new();
        let path = Path::new("/project/README.md");
        let content = b"# LynxSearch".to_vec();

        reader.add_file(path, Some("README.md"), content.clone());

        // 1. Read file successfully
        let payload = reader
            .read_file(path, &ReadOptions::default())
            .await
            .unwrap();
        assert_eq!(payload.bytes, content);
        assert_eq!(payload.size, content.len() as u64);
        assert_eq!(payload.hash, compute_sha256(&content));
        assert_eq!(
            payload.relative_path,
            Some(std::path::PathBuf::from("README.md"))
        );

        // 2. Read file with max size limit exceeded
        let strict_opts = ReadOptions::new(Some(5));
        let err = reader.read_file(path, &strict_opts).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationFailed(_)));

        // 3. File not found
        let not_found = reader
            .read_file(Path::new("/unknown.md"), &ReadOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(not_found, AppError::Io(_)));
    }
}
