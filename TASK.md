# TASK.md — LynxSearch
## Execution Plan Lengkap Fase 0–8 + Final Release Gate

> **Sumber utama:** `PRD-LynxSearch.md`, `ARCHITECTURE.md`, `DESIGN.md`
>
> **Tujuan file ini:** menjadi runbook implementasi yang dapat dieksekusi satu per satu. Setiap task dibuat kecil, dapat dicentang, dan memiliki dependency + verifikasi agar implementasi tidak melompat antar-layer.
>
> **Target lingkungan verifikasi primer:** Linux.
>
> **Scope:** Fase 1–8 sesuai spesifikasi; Fase 0 adalah preflight/reconciliation wajib agar kontrak teknis konsisten sebelum kode dibuat; bagian Final Release Gate hanya hardening dan integrasi dari seluruh scope yang sudah disepakati.

---

# 0. Aturan Eksekusi

## 0.1 Cara menggunakan task ini

- [ ] Kerjakan task **berurutan** dari atas ke bawah.
- [ ] Jangan masuk ke fase berikutnya sebelum **Phase Gate** fase aktif lulus.
- [ ] Setiap task yang menghasilkan perubahan kode wajib diikuti test/verification yang relevan.
- [ ] Jangan menambahkan fitur di luar scope PRD tanpa keputusan tertulis.
- [ ] Jika menemukan kontradiksi spesifikasi baru, **stop pada task tersebut**, dokumentasikan, lalu selesaikan sebagai reconciliation task sebelum lanjut.
- [ ] Jangan menganggap “compile berhasil” sebagai “feature selesai”. Feature baru dianggap selesai jika acceptance criteria dan test terkait lulus.
- [ ] Setelah refactor yang hanya mengubah struktur internal, seluruh test perilaku tetap harus lulus.

## 0.2 Prinsip arsitektur yang tidak boleh dilanggar

- [ ] Backend Rust dan desktop Tauri adalah **dua proses terpisah**.
- [ ] Backend tidak dijalankan sebagai embedded sidecar di Tauri.
- [ ] Docker Compose hanya untuk PostgreSQL dan Elasticsearch.
- [ ] Tauri tidak memiliki logika pencarian/indexing/parsing bisnis.
- [ ] Semua query/indexing/validation/domain logic berada di backend.
- [ ] Domain layer harus tetap bebas dari Axum, SQLx, Elasticsearch client, dan filesystem I/O.
- [ ] PostgreSQL adalah source of truth untuk metadata/state/settings.
- [ ] File di disk adalah source of truth untuk isi dokumen.
- [ ] Elasticsearch adalah disposable search index dan harus dapat dibangun ulang.
- [ ] Default backend bind tetap `127.0.0.1:3001`.
- [ ] Tidak ada credential/token rahasia hardcoded.
- [ ] Semua path filesystem lintas-platform menggunakan `Path`/`PathBuf`, bukan string separator manual.
- [ ] Index update normal bersifat manual/incremental; tidak ada real-time watcher.
- [ ] Tidak ada PDF/OCR/Word/image parsing.
- [ ] Tidak ada semantic/vector/KNN/hybrid search.
- [ ] Tidak ada AST/tree-sitter/symbol index/go-to-definition/find-references.
- [ ] Tidak ada multi-user/auth/remote access/cloud sync.

---

# 1. Audit Dokumen & Reconciliation Wajib (Fase 0)

## 1.1 Freeze sumber spesifikasi

- [x] Simpan tiga dokumen referensi di root repo atau lokasi dokumentasi yang disepakati:
  - [x] `PRD-LynxSearch.md`
  - [x] `ARCHITECTURE.md`
  - [x] `DESIGN.md`
- [x] Tandai `PRD-LynxSearch.md` sebagai source of truth untuk **requirement/user story**.
- [x] Tandai `ARCHITECTURE.md` sebagai source of truth untuk **komponen, boundary, data flow, persistence, API, concurrency, testing, security**.
- [x] Tandai `DESIGN.md` sebagai source of truth untuk **visual system, layout, micro-states, modal behavior, keyboard dan accessibility**.
- [x] Buat catatan keputusan bahwa task ini menggabungkan ketiganya tanpa mengubah product scope.

## 1.2 Resolusi konflik Elasticsearch client/server

**Blocking issue:**
- [x] Periksa seluruh referensi Elasticsearch client di `ARCHITECTURE.md`.
- [x] Hilangkan referensi yang menyatakan adapter memakai client **9.x**, karena stack utama menetapkan Elasticsearch **8.19.22** dan official Rust client major version **8.x**.
- [x] Jangan mengimplementasikan dua major version client.
- [x] Freeze satu dependency client Elasticsearch major-8 yang dapat terhubung ke server `8.19.22` (dibekukan: `elasticsearch = "=8.19.0-alpha.1"`).
- [ ] Jalankan `cargo check` setelah dependency dipasang (dituntaskan saat Cargo workspace diinisialisasi di Fase 1).
- [ ] Verifikasi request client dapat melakukan `ping`, index creation, bulk indexing dan search terhadap Elasticsearch `8.19.22` (dituntaskan saat docker compose aktif).
- [x] Update komentar/dokumentasi repository agar tidak ada lagi angka `9.x`.
- [ ] Catat versi final dependency yang benar di `Cargo.toml` dan `Cargo.lock` (dituntaskan saat `Cargo.toml` dibuat di Fase 1).

> **Gate:** tidak ada lagi referensi dependency Elasticsearch major 9 di codebase.

## 1.3 Resolusi konflik secret Docker

**Blocking issue:**
- [x] Hapus password PostgreSQL yang ditulis langsung di blueprint `docker-compose.yml` (`ARCHITECTURE.md`).
- [x] Pindahkan konfigurasi `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` ke environment substitution.
- [x] Pastikan `.env` masuk `.gitignore`.
- [x] Buat `.env.example` tanpa secret pribadi/credential produksi.
- [x] Dokumentasikan nilai contoh yang aman untuk local development.
- [x] Pastikan Compose gagal dengan jelas atau meminta environment jika variable wajib tidak tersedia (`${POSTGRES_PASSWORD:?...}`).
- [x] Pastikan tidak ada password/token rahasia yang ter-hardcode di source.
- [x] Verifikasi grep tidak menemukan credential nyata di workspace.

> **Catatan:** `xpack.security.enabled=false` tetap boleh untuk local-only Elasticsearch sesuai desain proyek, tetapi service tetap harus terikat ke `127.0.0.1`.

## 1.4 Resolusi konflik rename/move

- [x] Tandai perbedaan definisi rename/move antara PRD dan Architecture (diselesaikan: PRD diselaraskan dengan Architecture).
- [x] Gunakan detail implementasi `ScanPlanner` di Architecture sebagai kontrak eksekusi:
  - [x] cocokkan kandidat add/delete berdasarkan `content_hash`;
  - [x] buat `new_doc_id = UUIDv5(folder_id, new_relative_path)`;
  - [x] baca ulang file di `new_path` untuk metadata/content segar;
  - [x] index document baru;
  - [x] hapus document ID lama;
  - [x] update registry.
- [ ] Tambahkan test khusus rename/move agar tidak menghasilkan duplikasi (dituntaskan saat implementasi unit test `ScanPlanner` di Fase 2).
- [x] Dokumentasikan keputusan ini di ADR/decision note singkat (dicatat di PRD butir 6 dan Architecture 4.A: zero content redundancy mewajibkan re-read disk saat path berubah).

## 1.5 Resolusi query kosong vs DTO validation

- [x] Pertahankan kemampuan `q` optional.
- [x] Normalisasi `q` dengan trimming whitespace sebelum validasi semantic.
- [x] Query `None`, `""`, dan `"   "` diperlakukan sopan sebagai query kosong, bukan crash (validasi `max = 500` tanpa `min = 1` di DTO).
- [x] Untuk query kosong, tentukan kontrak response yang konsisten (HTTP 200 OK dengan `total: 0`, `items: []`, `took_ms: 0`, dan facets kosong).
- [ ] Tambahkan unit test untuk ketiganya (dituntaskan saat implementasi endpoint `/api/search` di Fase 5).

## 1.6 Freeze API contract sebelum implementasi handler

- [x] Bekukan endpoint berikut:
  - [x] `GET /api/health`
  - [x] `GET /api/health/live`
  - [x] `GET /api/health/ready`
  - [x] `GET /api/stats`
  - [x] `GET /api/folders`
  - [x] `POST /api/index/folder`
  - [x] `POST /api/index`
  - [x] `DELETE /api/folders/:id`
  - [x] `GET /api/index/jobs/:id`
  - [x] `POST /api/index/jobs/:id/cancel`
  - [x] `POST /api/index/rebuild`
  - [x] `GET /api/search`
  - [x] `GET /api/suggest`
  - [x] `GET /api/documents/:id`
  - [x] `DELETE /api/documents/:id`
  - [x] `GET /api/settings`
  - [x] `PUT /api/settings`
- [x] Tuliskan DTO request/response konkret untuk tiap endpoint sebelum handler dibuat (didefinisikan di ARCHITECTURE.md Section 6.2).
- [x] Pastikan response error mengikuti satu format:
  - [x] `code`
  - [x] `message`
  - [x] `details`
- [x] Bekukan status code utama:
  - [x] `200`
  - [x] `202`
  - [x] `400`
  - [x] `403`
  - [x] `404`
  - [x] `409`
  - [x] `422`
  - [x] `500`
  - [x] `503`
- [x] Bekukan field search result minimum:
  - [x] `id`
  - [x] `title`
  - [x] `relative_path`
  - [x] `project`
  - [x] `tags`
  - [x] `highlights`
  - [x] `score`
- [x] Tambahkan metadata yang dibutuhkan UI dari dokumen arsitektur tanpa mengubah makna result (`type`, `language`, `file_size`, `updated_at`).
- [x] Pastikan search response dapat membawa `warnings` untuk parser/filter warning non-fatal.
- [x] Pastikan search response membawa `total` dan `took_ms`.
- [x] Pastikan facet dapat dikembalikan pada response search (`types`, `languages`, `tags`, `projects`).

## 1.7 Freeze filter vocabulary dan canonical file mapping

- [x] Bekukan key filter:
  - [x] `language`
  - [x] `tag`
  - [x] `project`
  - [x] `extension`
  - [x] `type`
- [x] Bekukan `type`:
  - [x] `doc`
  - [x] `code`
  - [x] `config`
- [x] Bekukan mapping ekstensi:
  - [x] `.md`, `.markdown`, `.txt`
  - [x] `.json`, `.toml`, `.yaml`, `.yml`, `.ini`
  - [x] `.rs`
  - [x] `.ts`, `.tsx`, `.js`, `.jsx`
  - [x] `.py`
  - [x] `.go`
  - [x] `.java`, `.kt`
  - [x] `.c`, `.cpp`, `.h`
  - [x] `.cs`
  - [x] `.rb`
  - [x] `.php`
  - [x] `.swift`
  - [x] `.sh`
  - [x] `.sql`
  - [x] `.html`, `.css`, `.scss`
  - [x] `.vue`, `.svelte`, `.lua`
- [x] Bekukan secret rejection:
  - [x] `.env*`
  - [x] `*.pem`
  - [x] `*.key`
  - [x] `id_rsa`
  - [x] `*.p12`
- [x] Bekukan `project`:
  - [x] subfolder tingkat pertama di bawah root;
  - [x] file tepat di root -> `null`.
- [x] Bekukan `language` sebagai hasil deteksi ekstensi, bukan alias `tag`.
- [x] Bekukan `tag` berasal dari YAML front matter Markdown.

## 1.8 Freeze default settings

- [x] Default max file size = `2 MB` (2.097.152 bytes).
- [x] Default ignore patterns:
  - [x] `.git`
  - [x] `node_modules`
  - [x] `target`
  - [x] `dist`
  - [x] `build`
- [x] Default BM25 weights:
  - [x] title = `3.0`
  - [x] tags = `2.0`
  - [x] content = `1.0`
- [x] Bekukan persistence settings lewat PostgreSQL JSONB (`settings` table dengan `key = 'app_settings'`).
- [x] Bekukan reset-to-default behavior (fallback otomatis saat record kosong & reset jika payload PUT kosong/null).

## 1.9 Freeze scope yang tidak dikerjakan

- [x] Tidak implement semantic search.
- [x] Tidak implement vector/KNN/hybrid/reranking.
- [x] Tidak implement PDF/OCR/Word/image.
- [x] Tidak implement real-time filesystem watcher.
- [x] Tidak implement scheduled rescan.
- [x] Tidak implement AST/tree-sitter/symbol graph.
- [x] Tidak implement OpenSearch.
- [x] Tidak implement one-click installer yang membawa backend+DB+ES.
- [x] Tidak implement authentication/authorization.
- [x] Tidak implement remote access/LAN binding.
- [x] Tidak implement cloud sync.
- [x] Tidak implement network folder indexing.
- [x] Tidak implement full-content PostgreSQL storage.
- [x] Tidak implement automated desktop E2E browser test.
- [x] Tidak implement mobile/web client.
- [x] Tidak implement document editor di dalam LynxSearch.
- [x] Open question find-in-page preview tidak diimplementasikan kecuali keputusan scope diubah secara eksplisit.

## 1.10 Audit dependency yang belum eksplisit di hierarchy

- [x] Identifikasi dependency yang digunakan contoh kode tetapi belum tercantum jelas di stack.
- [x] Konfirmasi kebutuhan `dirs` untuk log directory (dieksplisitkan di ARCHITECTURE.md).
- [x] Konfirmasi `tempfile` untuk integration fixture generator (dieksplisitkan di ARCHITECTURE.md).
- [x] Konfirmasi `remark-gfm` atau ekuivalen untuk GitHub Flavored Markdown (dieksplisitkan di ARCHITECTURE.md).
- [x] Konfirmasi seluruh Radix primitives yang benar-benar digunakan (`@radix-ui/react-dialog`, `react-popover`, `react-scroll-area`, `react-tooltip`, `react-slider`).
- [x] Jangan menambah library hanya karena “umum”; setiap dependency harus punya fungsi yang jelas.
- [ ] Jalankan `cargo check` / `npm install` / `npm run build` setelah dependency freeze (dituntaskan saat inisialisasi monorepo di Fase 1).

## 1.11 Freeze design handoff

- [x] Default theme = dark.
- [x] Background dark = `#000000`.
- [x] Card = `#0a0a0a`.
- [x] Border = `#262626`.
- [x] Muted = `#a3a3a3`.
- [x] Accent = `#10a37f`.
- [x] Font sans = Inter/system fallback.
- [x] Font mono = JetBrains Mono/Fira Code/system monospace.
- [x] Default radius = 8px.
- [x] Desktop minimum window = `1024x640`.
- [x] Initial window = `1280x800`.
- [x] Sidebar facet = `260px`.
- [x] Preview panel = `420–640px`.
- [x] 4 micro-states wajib: Loading, Empty, Error, Success.
- [x] Reduced motion wajib.
- [x] Focus ring visible wajib.
- [x] Icon-only buttons wajib punya `aria-label`.

### Phase 0 Gate

- [x] Semua konflik spesifikasi di atas selesai dan keputusan tercatat.
- [x] Elasticsearch client/server major version tunggal dan konsisten.
- [x] Docker tidak memiliki password hardcoded.
- [x] API contract sudah dibekukan.
- [x] Filter vocabulary dan file mapping sudah dibekukan.
- [x] Scope out-of-scope sudah disetujui.
- [x] Dependency baseline dapat di-resolve tanpa error.

---

# 2. Fase 1 — Project & Workspace Setup

## 2.1 Prasyarat host

- [x] Verifikasi:
  - [x] `rustc --version` (`rustc 1.98.1`)
  - [x] `cargo --version` (`cargo 1.98.1`)
  - [x] `node --version` (`v24.11.1`)
  - [x] `npm --version` (`11.6.2`)
  - [x] `docker --version` (`Docker 29.8.1`)
  - [x] `docker compose version` (`Docker Compose v5.5.1`)
- [x] Pastikan Rust menggunakan Edition 2024 (didukung penuh rustc 1.98.1).
- [x] Pastikan toolchain dapat build Tauri native di Linux (terinstal: `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`).
- [x] Install `cargo-nextest` (terinstal: `cargo-nextest 0.9.146`).
- [x] Pastikan Git tersedia (`git 2.43.0`).

## 2.2 Buat struktur monorepo

- [x] Buat root workspace `Cargo.toml`.
- [x] Buat `crates/backend/`.
- [x] Buat `apps/desktop/`.
- [x] Buat `docker/`.
- [x] Buat `scripts/`.
- [x] Buat root `.env.example`.
- [x] Buat root `.gitignore`.
- [x] Pastikan root repo tidak mencampur artifact build.
- [x] Pastikan struktur sesuai:
  ```text
  Rust_LynxSearch/
  ├── docker/
  ├── crates/backend/
  ├── apps/desktop/
  ├── scripts/
  ├── Cargo.toml
  ├── .env.example
  └── .gitignore
  ```

## 2.3 Inisialisasi backend crate

- [x] Buat binary/library package untuk backend.
- [x] Sediakan `src/lib.rs`.
- [x] Sediakan `src/main.rs`.
- [x] Pastikan `main.rs` tetap tipis.
- [x] Siapkan modul:
  - [x] `config.rs`
  - [x] `error.rs`
  - [x] `api/`
  - [x] `application/`
  - [x] `domain/`
  - [x] `infrastructure/`
- [x] Tambahkan submodul placeholder sesuai architecture tree.

## 2.4 Inisialisasi desktop app

- [x] Buat app Tauri v2 + React + TypeScript + Vite.
- [x] Pastikan frontend berjalan sebagai webview desktop.
- [x] Pastikan backend tidak dijalankan sebagai sidecar.
- [x] Buat `src-tauri/`.
- [x] Buat `src-tauri/tauri.conf.json`.
- [x] Buat `src-tauri/capabilities/default.json`.
- [x] Buat `src/main.tsx` dan `src/App.tsx`.

## 2.5 Pasang dependency backend

- [x] Axum 0.8.
- [x] Tokio 1.x dengan fitur yang diperlukan.
- [x] tower 0.5.
- [x] tower-http 0.6 dengan feature yang diperlukan (`cors`, `trace`, `timeout`, dll. sesuai kebutuhan nyata).
- [x] SQLx 0.8.x dengan PostgreSQL + tokio + chrono features.
- [x] Elasticsearch client major 8 yang telah difreeze di Fase 0.
- [x] serde + serde_json.
- [x] validator.
- [x] thiserror.
- [x] chrono.
- [x] uuid dengan `v4`, `v5`, `serde`.
- [x] sha2.
- [x] hex.
- [x] ignore.
- [x] infer.
- [x] encoding_rs.
- [x] pulldown-cmark.
- [x] serde_yaml.
- [x] dotenvy.
- [x] tracing.
- [x] tracing-subscriber.
- [x] tracing-appender.
- [x] tokio-util untuk `CancellationToken`.
- [x] async-trait.
- [x] futures/futures-util.
- [x] parking_lot.
- [x] dashmap.
- [x] Tambahkan dependency valid lain hanya jika dibutuhkan oleh implementasi yang telah disepakati.

## 2.6 Pasang dependency frontend

- [x] React 19.3 + React DOM.
- [x] TypeScript 5.8+.
- [x] Vite 8.1.
- [x] `@vitejs/plugin-react`.
- [x] Tailwind CSS 4.3.
- [x] shadcn/ui + Radix primitives yang dibutuhkan.
- [x] `zod`.
- [x] `clsx`.
- [x] `tailwind-merge`.
- [x] `class-variance-authority`.
- [x] `lucide-react`.
- [x] `sonner`.
- [x] TanStack Query v5.
- [x] `@tanstack/react-virtual`.
- [x] Zustand 5.
- [x] `react-markdown`.
- [x] GFM plugin yang diperlukan.
- [x] Shiki.
- [x] Tauri API.
- [x] Tauri dialog plugin.
- [x] Tauri shell plugin.
- [x] Tauri opener plugin.
- [x] Tauri window-state plugin.
- [x] Tauri single-instance plugin.
- [x] Tauri clipboard-manager plugin.
- [x] Testing dependencies:
  - [x] Vitest.
  - [x] React Testing Library.
  - [x] `@testing-library/user-event`.
  - [x] MSW v2.
  - [x] `happy-dom`.
  - [x] `vitest-axe`.

## 2.7 Docker Compose

- [x] Buat `docker/docker-compose.yml`.
- [x] PostgreSQL:
  - [x] `postgres:18.6-alpine`
  - [x] localhost bind `127.0.0.1:5432`
  - [x] persistent volume `lynx_pgdata`
  - [x] healthcheck `pg_isready`
- [x] Elasticsearch:
  - [x] `elasticsearch:8.19.22`
  - [x] localhost bind `127.0.0.1:9200`
  - [x] persistent volume `lynx_esdata`
  - [x] `discovery.type=single-node`
  - [x] local-only `xpack.security.enabled=false`
  - [x] `ES_JAVA_OPTS=-Xms512m -Xmx512m`
  - [x] healthcheck cluster health
- [x] Pastikan tidak ada service backend di Compose.
- [x] Pastikan tidak ada service Tauri di Compose.

## 2.8 Environment

- [x] Buat `.env.example` dengan:
  - [x] `DATABASE_URL`
  - [x] `ELASTICSEARCH_URL`
  - [x] `BACKEND_BIND_ADDR`
  - [x] `RUST_LOG`
- [x] Tambahkan variable front-end API base URL bila diperlukan.
- [x] Default API target tetap localhost.
- [x] Pastikan `.env` di-ignore Git.
- [x] Pastikan config loader membaca `.env`.

## 2.9 Tooling dan quality scripts

- [x] Konfigurasi `rustfmt`.
- [x] Konfigurasi Clippy.
- [x] Tambahkan `nextest` command.
- [x] Tambahkan frontend lint command.
- [x] Tambahkan frontend typecheck command.
- [x] Tambahkan frontend coverage command.
- [x] Tambahkan build command.
- [x] Tambahkan visualizer build output.
- [x] Buat `scripts/bundle-budget-checker.py`.
- [x] Script harus memeriksa:
  - [x] JS gzip < 450 KB.
  - [x] CSS gzip < 50 KB.

## 2.10 Tauri configuration baseline

- [x] `productName = LynxSearch`.
- [x] Set identifier unik sesuai project.
- [x] Window `1280x800`.
- [x] Min `1024x640`.
- [x] Center window.
- [x] Drag region native.
- [x] CSP hanya membuka backend localhost yang diperlukan.
- [x] Tidak mengizinkan network eksternal yang tidak diperlukan.
- [x] Register plugin config.
- [x] Tambahkan capability permissions minimum untuk:
  - [x] core
  - [x] dialog
  - [x] shell
  - [x] opener
  - [x] window-state
  - [x] single-instance
  - [x] clipboard manager

## 2.11 Baseline build check

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo check --workspace`
- [x] `npm run typecheck`
- [x] `npm run build`
- [x] `docker compose -f docker/docker-compose.yml config`
- [x] `docker compose -f docker/docker-compose.yml up -d`
- [x] Pastikan PostgreSQL healthy.
- [x] Pastikan Elasticsearch healthy.
- [x] `docker compose ... down` berhasil tanpa error.

### Phase 1 Gate

- [x] Workspace dapat dibuild.
- [x] Docker data stores dapat hidup sehat.
- [x] Tidak ada secret hardcoded.
- [x] Backend dan desktop dapat dijalankan terpisah.
- [x] Quality commands tersedia dan baseline lulus.

---

# 3. Fase 2 — Backend Core & Axum API

## 3.1 `lib.rs` dan `main.rs`

- [ ] Pindahkan seluruh application/domain/infrastructure API export ke `lib.rs`.
- [ ] Pastikan `main.rs` hanya:
  - [ ] load dotenv/config;
  - [ ] initialize tracing;
  - [ ] create runtime resources;
  - [ ] bind listener;
  - [ ] start Axum server;
  - [ ] wire graceful shutdown.
- [ ] Pastikan integration test dapat mengakses router/library tanpa spawn binary.

## 3.2 Config layer

- [ ] Implement config struct untuk:
  - [ ] database URL
  - [ ] Elasticsearch URL
  - [ ] bind address
  - [ ] log level
  - [ ] max file size default
  - [ ] ignore patterns default
  - [ ] BM25 default weights
  - [ ] file-read concurrency limit
- [ ] Validasi konfigurasi saat startup.
- [ ] Beri error startup yang jelas jika variable wajib hilang/invalid.
- [ ] Jangan menyimpan secret di source.

## 3.3 AppState

- [ ] Buat shared state yang memuat:
  - [ ] DB pool.
  - [ ] Elasticsearch client.
  - [ ] repositories.
  - [ ] JobTracker.
  - [ ] worker queue sender.
  - [ ] file I/O semaphore.
  - [ ] runtime config/settings.
- [ ] Pastikan ownership/`Arc` semantics jelas.
- [ ] Hindari global mutable singleton.

## 3.4 Error taxonomy

- [ ] Implement `AppError` typed.
- [ ] Minimal domain categories:
  - [ ] folder not found
  - [ ] document not found
  - [ ] job conflict
  - [ ] validation failed
  - [ ] path traversal
  - [ ] invalid query
  - [ ] database error
  - [ ] search engine error
  - [ ] I/O error
  - [ ] internal error
- [ ] Map ke API error code yang telah dibekukan.
- [ ] Jangan bocorkan stack trace/internal detail ke UI.
- [ ] Simpan detail teknis pada tracing log.

## 3.5 Validation extractor

- [ ] Implement custom `ValidatedJson`.
- [ ] Implement custom `ValidatedQuery`.
- [ ] Jalankan `validator::Validate` sebelum use case.
- [ ] Validasi search:
  - [ ] page `1..=1000`
  - [ ] size `1..=100`
  - [ ] q max 500 setelah normalization.
- [ ] Validasi UUID path params.
- [ ] Validasi folder root path.
- [ ] Pastikan invalid DTO menghasilkan `422` sesuai kontrak.

## 3.6 Router skeleton

- [ ] Buat `api/mod.rs`.
- [ ] Buat `api/routes.rs`.
- [ ] Daftarkan seluruh endpoint sebagai route skeleton.
- [ ] Pisahkan handler:
  - [ ] `health.rs`
  - [ ] `search.rs`
  - [ ] `suggest.rs`
  - [ ] `folder.rs`
  - [ ] `index.rs`
  - [ ] `document.rs`
  - [ ] `settings.rs`

## 3.7 Middleware

- [ ] Tambahkan `TraceLayer`.
- [ ] Tambahkan request correlation/request_id.
- [ ] Tambahkan latency capture.
- [ ] Tambahkan local CORS untuk desktop client.
- [ ] Tambahkan timeout policy.
- [ ] Tambahkan compression sesuai kebutuhan.
- [ ] Pastikan error layer tidak mengubah structured error menjadi string generik.

## 3.8 Health endpoints

- [ ] Implement `GET /api/health/live`.
- [ ] Liveness hanya memverifikasi process/router hidup.
- [ ] Implement `GET /api/health/ready`.
- [ ] Readiness memeriksa PostgreSQL.
- [ ] Readiness memeriksa Elasticsearch.
- [ ] Return `503` jika dependency readiness gagal.
- [ ] Implement `GET /api/health`.
- [ ] Report:
  - [ ] backend status
  - [ ] database status + latency
  - [ ] Elasticsearch status + latency
- [ ] Pastikan frontend dapat membedakan offline vs dependency unavailable.

## 3.9 Startup crash recovery

- [ ] Saat startup, query job berstatus `RUNNING`/`PENDING`.
- [ ] Mark job tersebut `FAILED`.
- [ ] Isi `completed_at`.
- [ ] Isi `error_summary` recovery message.
- [ ] Reset folder `SCANNING` -> `IDLE`.
- [ ] Pastikan startup recovery idempoten.

## 3.10 Graceful shutdown

- [ ] Hook `SIGINT`/Ctrl+C.
- [ ] Hook `SIGTERM`.
- [ ] Stop accepting request baru.
- [ ] Trigger `CancellationToken`.
- [ ] Worker berhenti mengambil file baru.
- [ ] Selesaikan batch aktif.
- [ ] Terapkan grace period maksimum 10 detik.
- [ ] Mark unfinished jobs `CANCELLED`.
- [ ] Close DB pool.
- [ ] Close HTTP clients.
- [ ] Exit clean.

## 3.11 Worker supervisor skeleton

- [ ] Sediakan Supervisor Task.
- [ ] Spawn worker loop melalui `tokio::spawn`.
- [ ] Tangkap `JoinError`.
- [ ] Jika panic:
  - [ ] log critical error;
  - [ ] mark active job `FAILED`;
  - [ ] release folder lock;
  - [ ] restart worker setelah backoff 1 detik.
- [ ] Pastikan shutdown token menghentikan supervisor juga.

### Phase 2 Gate

- [ ] Liveness/readiness/health bekerja.
- [ ] Structured error response bekerja.
- [ ] Invalid DTO menghasilkan status yang benar.
- [ ] Backend hanya bind localhost.
- [ ] Graceful shutdown dan crash recovery teruji minimal lewat unit/integration test skeleton.
- [ ] Semua route sudah terdaftar, walaupun beberapa masih `TODO` implementasi use case.

---

# 4. Fase 3 — PostgreSQL, Elasticsearch & Repository Layer

## 4.1 Domain IDs dan models

- [ ] Implement `FolderId`.
- [ ] Implement `DocumentId`.
- [ ] Implement `JobId`.
- [ ] Implement deterministic UUIDv5 document identity dari `(folder_id, relative_path)`.
- [ ] Implement random job/folder UUID v4 sesuai kebutuhan.
- [ ] Implement value objects untuk status/type/language/filter.

## 4.2 PostgreSQL migration 0001 — folders

- [ ] Buat `folders`.
- [ ] Fields:
  - [ ] `id UUID PK`
  - [ ] `root_path TEXT NOT NULL UNIQUE`
  - [ ] `created_at`
  - [ ] `last_scanned_at`
  - [ ] `status`
- [ ] Index root path.
- [ ] Status allowed: `IDLE`, `SCANNING`, `ERROR`.

## 4.3 PostgreSQL migration 0002 — document registry

- [ ] Buat `document_registry`.
- [ ] Fields:
  - [ ] `id`
  - [ ] `folder_id`
  - [ ] `relative_path`
  - [ ] `content_hash`
  - [ ] `file_size_bytes`
  - [ ] `modified_at`
  - [ ] `status`
  - [ ] `status_reason`
  - [ ] `last_indexed_at`
- [ ] FK folder -> cascade delete.
- [ ] Unique `(folder_id, relative_path)`.
- [ ] Index folder_id.
- [ ] Index status.
- [ ] Status:
  - [ ] `INDEXED`
  - [ ] `SKIPPED`
  - [ ] `FAILED`
  - [ ] `EXCLUDED`

## 4.4 PostgreSQL migration 0003 — indexing jobs

- [ ] Buat `indexing_jobs`.
- [ ] `folder_id` nullable untuk global rebuild.
- [ ] `job_type` = `IMPORT`, `RESCAN`, `REBUILD`.
- [ ] `status` = `PENDING`, `RUNNING`, `COMPLETED`, `FAILED`, `CANCELLED`.
- [ ] Counter:
  - [ ] total
  - [ ] added
  - [ ] updated
  - [ ] deleted
  - [ ] skipped
  - [ ] failed
- [ ] Timestamp start/completed.
- [ ] `error_summary`.
- [ ] Index folder_id.
- [ ] Index status.

## 4.5 PostgreSQL migration 0004 — settings

- [ ] Buat `settings`.
- [ ] `key VARCHAR(64) PK`.
- [ ] `value JSONB NOT NULL`.
- [ ] `updated_at`.
- [ ] Seed default settings bila diperlukan.

## 4.6 SQLx setup

- [ ] Gunakan `sqlx::migrate!`.
- [ ] Gunakan parameterized query.
- [ ] Jangan gunakan `SELECT *`.
- [ ] Konfigurasi pool:
  - [ ] max 20
  - [ ] min 5
  - [ ] acquire timeout 3s
  - [ ] idle timeout 10m
  - [ ] max lifetime 30m
- [ ] Pastikan pool menghandle startup/readiness failure dengan structured error.

## 4.7 Repository ports

- [ ] Implement `SearchRepository`.
- [ ] Implement `FolderRepository`.
- [ ] Implement `DocumentRegistryRepository`.
- [ ] Implement `JobRepository`.
- [ ] Implement `SettingsRepository`.
- [ ] Jangan membuat “God Repository” yang menggabungkan semua aggregate.

## 4.8 PostgreSQL adapters

- [ ] `connection.rs`.
- [ ] `folder_repo.rs`.
- [ ] `registry_repo.rs`.
- [ ] `job_repo.rs`.
- [ ] `settings_repo.rs`.
- [ ] Implement CRUD/upsert/list operation sesuai port.
- [ ] Implement batch upsert.
- [ ] Implement batch deletion.
- [ ] Implement job progress increment.
- [ ] Implement job state transitions.
- [ ] Implement setting get/update/reset support.

## 4.9 Elasticsearch client adapter

- [ ] Buat `infrastructure/elasticsearch/client.rs`.
- [ ] Init client dari `ELASTICSEARCH_URL`.
- [ ] Implement ping.
- [ ] Implement clean abstraction sehingga domain tidak mengetahui client langsung.
- [ ] Gunakan alias `lynx_documents` sebagai search target.

## 4.10 Elasticsearch schema

- [ ] Buat mapping field:
  - [ ] `id`
  - [ ] `folder_id`
  - [ ] `relative_path`
  - [ ] `absolute_path` with `index: false`
  - [ ] `title`
  - [ ] `content`
  - [ ] `tags`
  - [ ] `extension`
  - [ ] `language`
  - [ ] `type`
  - [ ] `project`
  - [ ] `file_size_bytes`
  - [ ] `modified_at`
  - [ ] `indexed_at`
- [ ] `title`:
  - [ ] standard analyzer
  - [ ] `title.code`
  - [ ] `title.suggest`
- [ ] `content`:
  - [ ] standard analyzer
  - [ ] `content.code`
- [ ] keyword fields untuk filter/aggs.

## 4.11 Elasticsearch local index settings

- [ ] `number_of_shards = 1`.
- [ ] `number_of_replicas = 0`.
- [ ] Buat alias `lynx_documents`.
- [ ] Gunakan physical versioned indices:
  - [ ] `lynx_documents_v1`
  - [ ] `lynx_documents_v2`
  - [ ] dst.
- [ ] Pastikan first-run dapat membuat initial index + alias jika belum ada.

## 4.12 Reindex alias contract

- [ ] Implement create-versioned-index.
- [ ] Implement alias inspection.
- [ ] Implement alias swap.
- [ ] Implement old-index cleanup.
- [ ] Pastikan search selalu melalui alias.

## 4.13 Custom analyzer baseline

- [ ] Buat `code_subword_filter`.
- [ ] `word_delimiter_graph`.
- [ ] Split camelCase/snake_case.
- [ ] Preserve original token.
- [ ] Buat `code_analyzer`.
- [ ] Buat `autocomplete_filter` edge-ngram 2..20.
- [ ] Buat `autocomplete_analyzer`.
- [ ] Verifikasi index creation berhasil terhadap ES 8.19.22.
- [ ] Jangan aktifkan fitur code-specific UI sebelum Phase 8, tetapi schema boleh disiapkan.

## 4.14 Repository integration tests

- [ ] `FolderRepository`: create/list/get/delete.
- [ ] Unique root path.
- [ ] Update last scanned.
- [ ] `DocumentRegistryRepository`: get/list/upsert/delete.
- [ ] Unique folder + relative path.
- [ ] `JobRepository`: state transitions.
- [ ] Progress counters.
- [ ] `SettingsRepository`: get/update persistence.
- [ ] `SearchRepository`: ping.
- [ ] `SearchRepository`: initial mapping/index creation.
- [ ] `SearchRepository`: alias behavior.

### Phase 3 Gate

- [ ] `sqlx` migrations clean.
- [ ] Repository integration test lulus.
- [ ] PostgreSQL persistence lulus.
- [ ] Elasticsearch index + alias + mapping dapat dibuat.
- [ ] Elasticsearch client versi konsisten dan dapat ping.
- [ ] Tidak ada SQL `SELECT *`.
- [ ] Search repository dapat beroperasi melalui alias.

---

# 5. Fase 4 — Document Indexing Engine

## 5.1 Domain events

- [ ] Implement:
  - [ ] `IndexingJobStarted`
  - [ ] `DocumentIndexed`
  - [ ] `DocumentSkipped`
  - [ ] `DocumentFailed`
  - [ ] `IndexingJobCompleted`
  - [ ] `IndexRebuilt`
- [ ] Pastikan event tidak melakukan I/O.
- [ ] Gunakan event untuk logging/observability tanpa menaruh infra dependency di domain.

## 5.2 File System adapter

- [ ] Implement `walker.rs`.
- [ ] Gunakan crate `ignore`.
- [ ] Hormati `.gitignore`.
- [ ] Hormati `.ignore`.
- [ ] Hormati global gitignore jika tersedia lewat crate behavior.
- [ ] Skip hidden/dotfiles sesuai aturan traversal.
- [ ] Terapkan ignore patterns dari settings.
- [ ] Pastikan `.git`, `node_modules`, `target`, `dist`, `build` terabaikan secara default.
- [ ] Jangan mengikuti file yang di-blacklist secret.

## 5.3 Reader adapter

- [ ] Implement `reader.rs`.
- [ ] Read file melalui `spawn_blocking`.
- [ ] Batasi concurrent file reads dengan `Semaphore`.
- [ ] Default permits = 50.
- [ ] Hash content SHA-256.
- [ ] Return bytes + hash + metadata yang dibutuhkan extractor.
- [ ] Hindari membuka ribuan file bersamaan.

## 5.4 DocumentExtractor

- [ ] Implement pure `DocumentExtractor`.
- [ ] Input: path + bytes + root context + file metadata.
- [ ] Output canonical `ExtractedDoc` atau `Skipped`.
- [ ] Tentukan title:
  - [ ] Markdown -> H1 pertama.
  - [ ] lainnya -> filename fallback.
- [ ] Extract plain text content.
- [ ] Detect extension.
- [ ] Detect language.
- [ ] Detect type.
- [ ] Detect project.
- [ ] Extract YAML front matter tags.
- [ ] Preserve size.
- [ ] Preserve modified time.
- [ ] Detect binary.
- [ ] Enforce max size.
- [ ] Reject secret filenames/extensions.
- [ ] Unit test semua mapping.

## 5.5 Binary detection

- [ ] Tier 1: extension blacklist.
- [ ] Tier 2: magic bytes via `infer`.
- [ ] Tier 3: null-byte scan 8 KB pertama.
- [ ] Event skip `BinaryFileDetected`.
- [ ] Jangan membuat binary file sebagai search document.

## 5.6 Encoding

- [ ] Try UTF-8 first.
- [ ] Implement safe non-UTF-8 fallback sesuai capability dependency yang benar-benar tersedia.
- [ ] Jika decoding tidak aman, fallback lossily.
- [ ] Emit warning `LossyEncodingDecoded`.
- [ ] Pastikan satu file encoding rusak tidak menggagalkan job.

## 5.7 Metadata project

- [ ] `root/README.md` -> `project = null`.
- [ ] `root/rust/ownership.md` -> `project = rust`.
- [ ] `root/backend/redis.md` -> `project = backend`.
- [ ] Pastikan hanya level pertama di bawah root yang dipakai.

## 5.8 ScanPlanner

- [ ] Implement pure `ScanPlanner`.
- [ ] Input:
  - [ ] current filesystem inventory
  - [ ] DB registry
  - [ ] current settings.
- [ ] Kategori:
  - [ ] `to_add`
  - [ ] `to_update`
  - [ ] `to_delete`
  - [ ] `to_skip`
  - [ ] move/rename candidates
- [ ] `EXCLUDED` -> `UserExcluded` skip.
- [ ] Same mtime + same size -> unchanged fast path.
- [ ] Different metadata -> compute hash.
- [ ] Same hash -> skip as unchanged.
- [ ] Different hash -> update.
- [ ] DB record missing physically -> delete.
- [ ] Physical file missing but DB status already EXCLUDED -> preserve exclusion semantics.
- [ ] Candidate add/delete same content hash -> move candidate.
- [ ] Ensure deterministic IDs.

## 5.9 Index identity

- [ ] Derive document UUID deterministically from folder ID + relative path.
- [ ] Repeat same scan -> same ID.
- [ ] No random document IDs.
- [ ] Rename path -> new deterministic document ID.

## 5.10 IndexOrchestrator

- [ ] Create `application/orchestrator/`.
- [ ] Implement worker queue.
- [ ] Implement queue dispatcher.
- [ ] Implement folder lock manager.
- [ ] Implement in-memory JobTracker.
- [ ] Use `DashMap` for job tracking.
- [ ] Use appropriate Tokio mutex only across await where required.
- [ ] Do not use a single global blocking mutex for all jobs.

## 5.11 Job lifecycle

- [ ] `PENDING`.
- [ ] `RUNNING`.
- [ ] `COMPLETED`.
- [ ] `FAILED`.
- [ ] `CANCELLED`.
- [ ] Progress counters updated consistently.
- [ ] `files_total` equals planning result expected for progress semantics.
- [ ] Final summary persisted.

## 5.12 Regular scan execution

- [ ] Create job.
- [ ] Mark folder `SCANNING`.
- [ ] Traverse files.
- [ ] Build `ScanPlan`.
- [ ] Extract new/updated docs.
- [ ] Bulk index to Elasticsearch.
- [ ] Delete missing documents.
- [ ] Update registry.
- [ ] Update counters.
- [ ] Commit DB progress batch in chunks of 100 docs.
- [ ] Make regular rescan idempotent.

## 5.13 Move/rename execution

- [ ] Index new path using new deterministic ID.
- [ ] Remove old path/index ID.
- [ ] Update registry.
- [ ] Do not leave both copies.
- [ ] Add integration test verifying one logical document remains.

## 5.14 Per-file failure resilience

- [ ] Catch extraction/read/hash errors at file boundary.
- [ ] Mark registry `FAILED` with reason.
- [ ] Increment failed counter.
- [ ] Emit `DocumentFailed`.
- [ ] Continue processing remaining files.
- [ ] Do not fail entire job because one file fails.

## 5.15 Job lock semantics

- [ ] Only one indexing job active per folder.
- [ ] Second trigger returns `409 JOB_CONFLICT`.
- [ ] Lock released on:
  - [ ] completed
  - [ ] failed
  - [ ] cancelled
  - [ ] worker panic.
- [ ] Rebuild uses global lock.
- [ ] While rebuild is active, new scan job is rejected as specified.

## 5.16 Cancel API

- [ ] Implement `POST /api/index/jobs/:id/cancel`.
- [ ] Cancel only active job.
- [ ] Signal `CancellationToken`.
- [ ] Stop new file acquisition.
- [ ] Finish active batch within grace policy.
- [ ] Persist `CANCELLED`.
- [ ] Release folder lock.

## 5.17 Folder APIs

- [ ] `GET /api/folders`.
- [ ] `POST /api/index/folder`.
- [ ] New folder -> register + start initial import.
- [ ] Existing folder -> trigger rescan.
- [ ] Duplicate root -> clear validation/conflict response.
- [ ] `DELETE /api/folders/:id`.
- [ ] Delete folder metadata.
- [ ] Delete corresponding Elasticsearch documents.
- [ ] Rely on PostgreSQL FK cascade for metadata child rows.
- [ ] Update folder status cleanly.

## 5.18 Single-document exclusion/un-exclusion

- [ ] `DELETE /api/documents/:id`.
- [ ] Delete document from Elasticsearch.
- [ ] Keep registry tombstone as `EXCLUDED`.
- [ ] Persist reason.
- [ ] Ensure next rescan does not resurrect it.
- [ ] `POST /api/index` restores a previously EXCLUDED document.
- [ ] Restore must set registry/index state according to contract.

## 5.19 Job status API

- [ ] Implement `GET /api/index/jobs/:id`.
- [ ] Return:
  - [ ] status
  - [ ] counters
  - [ ] timestamps
  - [ ] error summary if applicable.
- [ ] UI polling can consume this response.

## 5.20 Rebuild index

- [ ] `POST /api/index/rebuild`.
- [ ] Acquire global rebuild lock.
- [ ] Create new physical index.
- [ ] Stream all eligible registry docs to new index.
- [ ] Use chunks of 200 for rebuild stream.
- [ ] Keep old alias target active during build.
- [ ] Atomic alias swap.
- [ ] Verify alias points to new index.
- [ ] Remove old index.
- [ ] Mark rebuild job complete.
- [ ] On failure, keep old alias active and mark job failed.
- [ ] Ensure no search downtime during successful swap.

## 5.21 Search availability during indexing

- [ ] Basic searches can continue while indexing background jobs run.
- [ ] Do not block HTTP server while scanning.
- [ ] Verify background worker does not monopolize Tokio runtime.

## 5.22 Indexing tests

- [ ] Unit tests: extractor.
- [ ] Unit tests: scan planner.
- [ ] Unit tests: deterministic IDs.
- [ ] Integration: first import.
- [ ] Integration: rescan unchanged.
- [ ] Integration: changed file.
- [ ] Integration: deleted file.
- [ ] Integration: rename/move.
- [ ] Integration: excluded file.
- [ ] Integration: single failed file.
- [ ] Integration: concurrency lock.
- [ ] Integration: cancellation.
- [ ] Integration: panic recovery.
- [ ] Integration: rebuild alias.
- [ ] Integration: restart recovery.

### Phase 4 Gate

- [ ] First import works end-to-end.
- [ ] Re-scan is incremental and idempotent.
- [ ] Deleted files disappear from search index.
- [ ] Rename/move produces no duplicates.
- [ ] EXCLUDED documents stay excluded.
- [ ] Per-file failure does not kill job.
- [ ] One active job per folder is enforced.
- [ ] Cancellation works.
- [ ] Rebuild uses alias swap and preserves search availability.
- [ ] Job progress can be polled.

---

# 6. Fase 5 — Basic Search Engine

## 6.1 SearchQuery baseline

- [ ] Implement raw query input for free-text search.
- [ ] Normalize case for text matching through analyzer/query strategy.
- [ ] Support multi-word query.
- [ ] Treat quotes/special characters safely.
- [ ] Avoid accidental query-string injection by building structured DSL.

## 6.2 SearchQueryBuilder baseline

- [ ] Implement pure `SearchQueryBuilder`.
- [ ] Build `multi_match`.
- [ ] Default boosts:
  - [ ] `title^3.0`
  - [ ] `tags^2.0`
  - [ ] `content^1.0`
- [ ] Query code subfields when available.
- [ ] Preserve `_score`.
- [ ] Request total hit count.
- [ ] Request highlight fragments.
- [ ] Include relevant source fields only.
- [ ] Build pagination.
- [ ] Keep filters separate from scoring via `bool.filter` when advanced filters arrive.

## 6.3 SearchRepository search

- [ ] Execute generated DSL against `lynx_documents`.
- [ ] Parse response into application result.
- [ ] Capture Elasticsearch execution latency.
- [ ] Map missing/down ES to `503 SEARCH_ENGINE_UNAVAILABLE`.
- [ ] Map malformed DSL/internal errors to structured server errors.

## 6.4 Highlight

- [ ] Configure highlight for multiple fragments.
- [ ] Use `<em>` markers in backend response only if the output contract is fixed.
- [ ] Preserve enough source context for UI.
- [ ] Do not store highlight fragments as source of truth.

## 6.5 Safe highlight rendering

- [ ] Do not inject arbitrary file content into DOM as raw HTML.
- [ ] Parse the controlled highlight marker format into React nodes.
- [ ] Render matched segments as semantic `<mark>` or equivalent UI.
- [ ] Add a test proving HTML-like text inside a document is not interpreted as executable markup.

## 6.6 Line-number algorithm

- [ ] Implement newline counting against original file content.
- [ ] Calculate:
  `1 + count('\n' before match offset)`.
- [ ] Return `line_number` where appropriate.
- [ ] Test first line, middle line, last line, multiline snippet.
- [ ] Ensure line number is based on raw `_source.content`, not transformed rendered output.

## 6.7 Search API

- [ ] Implement `GET /api/search`.
- [ ] Accept:
  - [ ] `q`
  - [ ] `page`
  - [ ] `size`
  - [ ] `sort` parameter reserved/compatible with later advanced sorting.
- [ ] Return:
  - [ ] normalized query
  - [ ] total
  - [ ] took_ms
  - [ ] results
  - [ ] facets when available
  - [ ] warnings when available
- [ ] Empty query handled without error.
- [ ] Special characters handled safely.
- [ ] Case insensitive.
- [ ] Return 200 for normal search.
- [ ] Validate page/size.

## 6.8 Basic result DTO

- [ ] `id`.
- [ ] `title`.
- [ ] `relative_path`.
- [ ] `project`.
- [ ] `tags`.
- [ ] `highlights[]`.
- [ ] `highlights[].snippet`.
- [ ] `highlights[].line_number`.
- [ ] `score`.
- [ ] Add other display-only metadata required by UI.

## 6.9 Basic search tests

- [ ] `rust ownership` matches title/content.
- [ ] Title boost affects rank.
- [ ] Score returned.
- [ ] Total returned.
- [ ] Query latency returned.
- [ ] Pagination works.
- [ ] Empty query does not error.
- [ ] Case insensitive.
- [ ] Special characters do not corrupt query.
- [ ] Highlight appears.
- [ ] Multiple fragments appear.
- [ ] Line number is correct.

### Phase 5 Gate

- [ ] `/api/search` works against real Elasticsearch.
- [ ] BM25 result ordering works with configured boosts.
- [ ] Scores and latency are visible in response.
- [ ] Highlight + line number work.
- [ ] Pagination works.
- [ ] Basic search tests pass.

---

# 7. Fase 6 — Tauri Desktop MVP

## 7.1 Frontend foundation

- [ ] Configure `vite.config.ts`.
- [ ] Add React plugin.
- [ ] Add `@` alias.
- [ ] Add build manual chunks:
  - [ ] vendor-react
  - [ ] vendor-tanstack
  - [ ] vendor-ui
  - [ ] vendor-markdown
- [ ] Add Rollup visualizer.
- [ ] Set dev server to port 5173.
- [ ] Enable strict port.
- [ ] Configure optimizeDeps list.

## 7.2 Tailwind + design tokens

- [ ] Import Tailwind 4.3.
- [ ] Implement `:root` light variables.
- [ ] Implement `.dark` variables.
- [ ] Implement `@theme`.
- [ ] Implement typography tokens.
- [ ] Implement Inter/system font stack.
- [ ] Implement mono stack.
- [ ] Implement 8px default radius.
- [ ] Set dark as default application theme.
- [ ] Preserve light fallback.

## 7.3 shadcn/ui foundation

- [ ] Configure `components.json`.
- [ ] Create `src/lib/utils.ts`.
- [ ] Implement `cn()`.
- [ ] Add required shadcn primitives:
  - [ ] Button
  - [ ] Input
  - [ ] Dialog
  - [ ] Popover
  - [ ] Tooltip
  - [ ] ScrollArea
  - [ ] Skeleton
  - [ ] Alert
  - [ ] Badge
  - [ ] Checkbox
  - [ ] Slider
  - [ ] Select/dropdown as needed.

## 7.4 API client

- [ ] Create central HTTP client in `src/lib`.
- [ ] Centralize backend base URL.
- [ ] Default to `http://127.0.0.1:3001`.
- [ ] Keep configurable.
- [ ] Implement request timeout/cancellation behavior.
- [ ] Parse all critical responses with Zod.
- [ ] Parse structured error responses with Zod.

## 7.5 Type contracts

- [ ] Mirror backend DTOs under `src/types`.
- [ ] Add Zod schemas for:
  - [ ] health
  - [ ] search
  - [ ] search result
  - [ ] folder
  - [ ] job
  - [ ] document
  - [ ] stats
  - [ ] settings
  - [ ] errors
- [ ] Infer TS types from Zod where practical.
- [ ] Keep backend/frontend field names consistent.

## 7.6 State architecture

- [ ] Create `useSearchStore`.
- [ ] Create `useUIStore`.
- [ ] `rawQuery` -> Zustand.
- [ ] `activeFilters` -> Zustand.
- [ ] `selectedDocId` -> Zustand.
- [ ] `sidebarCollapsed` -> Zustand.
- [ ] `theme` -> Zustand + localStorage.
- [ ] Search results -> TanStack Query.
- [ ] Suggestions -> TanStack Query.
- [ ] Health -> TanStack Query polling.
- [ ] Job progress -> TanStack Query polling.
- [ ] Local form state -> React hooks.

## 7.7 TanStack Query setup

- [ ] Create QueryClient provider.
- [ ] Search query hook.
- [ ] Suggest query hook.
- [ ] Health query hook.
- [ ] Folder query/mutations.
- [ ] Job status query.
- [ ] Document preview query.
- [ ] Settings query/mutation.
- [ ] Stats query.
- [ ] Configure search staleTime around 30s as architecture guidance.
- [ ] Health polling around 10s.
- [ ] Job polling around 1s when active.
- [ ] Suggest debounce 150ms.
- [ ] Ensure query cancellation on fast typing.

## 7.8 3-pane workspace shell

- [ ] Create top bar.
- [ ] Add Tauri drag region.
- [ ] Add global search input.
- [ ] Add backend health dot.
- [ ] Add Folder button.
- [ ] Add Settings button.
- [ ] Create left facet pane placeholder.
- [ ] Create center results pane.
- [ ] Create right preview pane.
- [ ] Make preview pane resizable.
- [ ] Constrain preview width to 420–640px.
- [ ] Make sidebar collapsible.
- [ ] Make preview collapsible.
- [ ] Add bottom hotkey/status bar.

## 7.9 SearchBar

- [ ] Input global query.
- [ ] Add `Cmd/Ctrl+K`.
- [ ] Add `/` shortcut.
- [ ] Add debounce.
- [ ] Add active filter chips scaffold.
- [ ] Show `Loader2` while search debounce/query is active where appropriate.
- [ ] Preserve keyboard focus.
- [ ] Do not lose query text when preview changes.

## 7.10 Result list

- [ ] Create `ResultList`.
- [ ] Create `ResultCard`.
- [ ] Display:
  - [ ] title
  - [ ] relative path
  - [ ] snippet
  - [ ] score
  - [ ] line number if present
  - [ ] tags/metadata.
- [ ] Add selected state.
- [ ] Add click-to-preview.
- [ ] Add result keyboard navigation.
- [ ] Prepare pagination controls.
- [ ] Preserve selected document when possible between rerenders.

## 7.11 Preview

- [ ] Create `DocumentPreview`.
- [ ] Create preview header.
- [ ] Add title.
- [ ] Add relative/absolute path information as specified.
- [ ] Add close action.
- [ ] Add `Open in Editor`.
- [ ] Add `Copy Path`.
- [ ] Add Markdown rendering.
- [ ] Add plain/code rendering placeholder that will be upgraded to virtualized viewer in Phase 8.
- [ ] Highlight current query terms safely.

## 7.12 Folder Manager

- [ ] Create `FolderManagerModal`.
- [ ] Use native folder picker.
- [ ] Add `+ Tambah Folder`.
- [ ] Display folder path.
- [ ] Display last scan relative time.
- [ ] Display indexed document count.
- [ ] Display status:
  - [ ] IDLE
  - [ ] INDEXING
  - [ ] ERROR
- [ ] Re-scan action.
- [ ] Delete action.
- [ ] Confirm deletion.
- [ ] Rebuild Index action.
- [ ] Confirm rebuild with stronger warning.

## 7.13 Job progress

- [ ] Create `JobProgressIndicator`.
- [ ] Create banner/drawer.
- [ ] Show progress bar.
- [ ] Show `Processed`.
- [ ] Show `Skipped`.
- [ ] Show `Failed`.
- [ ] Add cancel button.
- [ ] Poll every 1s while running.
- [ ] Stop polling after terminal state.
- [ ] Show final summary.

## 7.14 Settings

- [ ] Create `SettingsModal`.
- [ ] Numeric max file size.
- [ ] Ignore pattern tag input.
- [ ] BM25 title slider.
- [ ] BM25 tag slider.
- [ ] BM25 content slider.
- [ ] Reset to default.
- [ ] Save settings.
- [ ] Persist settings through API.
- [ ] Show saved toast.

## 7.15 Stats

- [ ] Add stats view/modal/panel according to workspace design.
- [ ] Display total documents.
- [ ] Display total size/index size as exposed by backend.
- [ ] Display distribution by type/language.
- [ ] Show loading/error/empty/success states.

## 7.16 Health and offline

- [ ] Create health dot.
- [ ] Green = backend healthy.
- [ ] Yellow = indexing state as defined by UI.
- [ ] Red = backend unavailable.
- [ ] Create `ConnectionBanner`.
- [ ] Create `OfflineFallback`.
- [ ] Keep search UI usable enough to explain backend outage.
- [ ] Retry action triggers query refetch.

## 7.17 Toasts

- [ ] Create `LynxToaster`.
- [ ] Bottom-right position.
- [ ] Dark theme.
- [ ] Rich colors.
- [ ] Close button.
- [ ] Trigger:
  - [ ] copy success
  - [ ] scan started
  - [ ] settings saved
  - [ ] job cancelled
  - [ ] backend connection error

## 7.18 Date helpers

- [ ] Create `formatDateTime`.
- [ ] Create `formatRelativeTime`.
- [ ] Use native `Intl`.
- [ ] Test null/undefined.
- [ ] Test past/future dates.

## 7.19 Micro-states

Implement explicit states on every data-driven component.

- [ ] Loading:
  - [ ] Skeleton.
  - [ ] Same dimensions as final card.
  - [ ] Avoid CLS.
- [ ] Empty:
  - [ ] SearchX/FolderPlus.
  - [ ] explanatory copy.
  - [ ] clear filter action.
- [ ] Error:
  - [ ] alert card.
  - [ ] user-friendly message.
  - [ ] structured error code.
  - [ ] retry.
- [ ] Success:
  - [ ] correct data.
  - [ ] highlight.
  - [ ] relevance indicator.

## 7.20 Keyboard navigation

- [ ] `Cmd/Ctrl+K` focus search.
- [ ] `/` focus search.
- [ ] `Escape` close/clear contextually.
- [ ] `ArrowDown` and `j` next result.
- [ ] `ArrowUp` and `k` previous result.
- [ ] `Enter` preview selected result.
- [ ] `Cmd/Ctrl+O` open file.
- [ ] `Cmd/Ctrl+Shift+C` copy path.
- [ ] `[` collapse sidebar.
- [ ] `]` collapse preview.
- [ ] Ensure shortcuts do not fire while typing in unrelated text fields unless intended.

## 7.21 Tauri native operations

- [ ] Dialog plugin opens native directory picker.
- [ ] Shell plugin opens configured editor/command as specified.
- [ ] Opener plugin opens/reveals file using native OS.
- [ ] Window-state restores size/position.
- [ ] Single-instance prevents multiple windows/processes.
- [ ] Clipboard manager writes absolute path.
- [ ] Keep permissions minimum required.

## 7.22 Accessibility

- [ ] Contrast >= 4.5:1 for normal text.
- [ ] Relevant code/accent contrast >= 3:1.
- [ ] Visible focus rings.
- [ ] Icon buttons have explicit `aria-label`.
- [ ] Correct dialog semantics.
- [ ] Correct button/checkbox/slider labels.
- [ ] Keyboard focus order logical.
- [ ] Reduced-motion media query disables pulse/transition as specified.
- [ ] Do not trap keyboard focus incorrectly.

### Phase 6 Gate

- [ ] Desktop opens as a native Tauri app.
- [ ] Search can be typed and results rendered.
- [ ] Result click opens preview.
- [ ] Folder can be picked natively.
- [ ] Folder list updates.
- [ ] Job progress is visible.
- [ ] Settings and stats are accessible.
- [ ] Health/offline states are understandable.
- [ ] Keyboard workflow works.
- [ ] API responses are Zod-validated.
- [ ] Four micro-states implemented on core data components.

---

# 8. Fase 7 — Advanced Search, Facets, Fuzzy, Prefix & Autocomplete

## 8.1 Query Parser AST

- [ ] Implement pure `QueryParser`.
- [ ] Input:
  ```text
  "tokio runtime" language:rust tag:concurrency project:backend type:code ownership
  ```
- [ ] Output:
  - [ ] free terms
  - [ ] phrase terms
  - [ ] filters
  - [ ] warnings
- [ ] Support `key:value`.
- [ ] Support quoted phrases.
- [ ] Preserve free terms outside filters.
- [ ] Support multiple filters.
- [ ] Normalize known keys.
- [ ] Unknown key -> structured warning.
- [ ] Malformed token -> structured warning.
- [ ] Never silently discard invalid filter.
- [ ] Empty query -> safe result.

## 8.2 Parser edge cases

- [ ] Extra whitespace.
- [ ] Repeated spaces.
- [ ] Upper/lower case keys/values as defined.
- [ ] Quotes.
- [ ] Parentheses.
- [ ] Slashes.
- [ ] punctuation.
- [ ] empty value.
- [ ] unknown field.
- [ ] mixed free-text + filters.
- [ ] multiple filter values if supported by concrete DTO contract.
- [ ] Warning order deterministic for deterministic tests.

## 8.3 SearchQueryBuilder advanced logic

- [ ] Accept structured `SearchQuery`.
- [ ] `bool.must/should` for scoring/free terms.
- [ ] `bool.filter` for exact filters.
- [ ] Fuzzy clauses in lower-boost `should`.
- [ ] `fuzziness = AUTO`.
- [ ] Fuzzy boost around `0.5` as architecture baseline.
- [ ] Prefix clause for incomplete words.
- [ ] Preserve exact-match relevance higher than fuzzy.
- [ ] Add aggregation definitions.
- [ ] Add sort definition.
- [ ] Keep pagination.
- [ ] Include parser warnings in application response.

## 8.4 Filters

Implement exact filtering for:

- [ ] `language:rust`
- [ ] `tag:concurrency`
- [ ] `project:backend`
- [ ] `extension:rs`
- [ ] `type:code`
- [ ] Multiple filters in one query.
- [ ] Ensure filter does not alter BM25 score.
- [ ] Ensure unknown values are reported rather than silently ignored.

## 8.5 Facets

- [ ] `extension` aggregation.
- [ ] `language` aggregation.
- [ ] `type` aggregation.
- [ ] `project` aggregation.
- [ ] `tags` aggregation.
- [ ] Facets reflect active query.
- [ ] Filter clauses stay under `bool.filter`.
- [ ] Return hit counts.
- [ ] Handle missing/empty facet results cleanly.

## 8.6 Facet sidebar UI

- [ ] Create `FacetSidebar`.
- [ ] Group facets by category.
- [ ] Checkbox.
- [ ] Count badge.
- [ ] Active state.
- [ ] Collapse with `[`.
- [ ] Apply filter on click.
- [ ] Remove filter on uncheck.
- [ ] Show current active filters.

## 8.7 Two-way facet/query synchronization

- [ ] Clicking facet adds corresponding token to query.
- [ ] Editing filter token updates checked facet.
- [ ] Removing token unchecks facet.
- [ ] Query and facet state use single source of truth in `useSearchStore`.
- [ ] Avoid duplicate filter tokens.
- [ ] Preserve free-text terms when changing facet.

## 8.8 Fuzzy search

- [ ] `rust ownrship` matches `Rust Ownership`.
- [ ] Exact result gets stronger score.
- [ ] Fuzzy result never overrides exact match merely because it exists.
- [ ] Test rank ordering using real Elasticsearch.
- [ ] Ensure fuzzy does not activate on facet values accidentally.

## 8.9 Prefix search

- [ ] `owner` finds `ownership`.
- [ ] Prefix search is lower-boosted/appropriate to relevance.
- [ ] Test partial term.
- [ ] Test no match.

## 8.10 Autocomplete backend

- [ ] Implement `GET /api/suggest`.
- [ ] Read prefix.
- [ ] Use autocomplete field/analyzer.
- [ ] Return lightweight suggestion list.
- [ ] Enforce reasonable result limit.
- [ ] Avoid full search response shape.
- [ ] Fail gracefully when ES unavailable.

## 8.11 Autocomplete frontend

- [ ] Create `AutocompletePopover`.
- [ ] Debounce 150ms.
- [ ] Avoid request on blank/very-short prefix if desired by contract.
- [ ] Render suggestions near search input.
- [ ] Keyboard navigate suggestions.
- [ ] Enter selects suggestion.
- [ ] Escape closes suggestion list.
- [ ] Cancel stale requests.

## 8.12 Sorting

Implement:

- [ ] `relevance` default.
- [ ] `modified_at`.
- [ ] `name`.
- [ ] Define concrete field used for `name` before implementation.
- [ ] Preserve sort on page change.
- [ ] Reset page when search/filter changes if appropriate.
- [ ] Include sort in query state and request key.
- [ ] Test page 2 under same sort returns consistent order.

## 8.13 BM25 settings

- [ ] Load weights from PostgreSQL settings.
- [ ] Use title/tag/content values at query-builder runtime.
- [ ] Update settings through API.
- [ ] Ensure new weights affect subsequent searches.
- [ ] Persist across restart.
- [ ] Reset-to-default.
- [ ] Avoid invalid negative/zero weights unless contract explicitly allows them.
- [ ] Test ranking difference after weight change.

## 8.14 Advanced query tests

- [ ] Parser unit suite.
- [ ] Warning suite.
- [ ] Filter suite.
- [ ] Facet aggregation suite.
- [ ] Fuzzy integration suite.
- [ ] Prefix integration suite.
- [ ] Autocomplete integration suite.
- [ ] Sort suite.
- [ ] Pagination+sort suite.
- [ ] BM25 tuning suite.
- [ ] Frontend filter synchronization suite.

### Phase 7 Gate

- [ ] Inline filters work.
- [ ] Facets work and stay synchronized with query.
- [ ] Fuzzy and prefix work.
- [ ] Autocomplete is debounced and responsive.
- [ ] Sorting works and persists across pages.
- [ ] BM25 settings persist and change ranking.
- [ ] All parser warnings are visible/understandable.
- [ ] Advanced-search integration tests pass.

---

# 9. Fase 8 — Code Search Mastery

## 9.1 Canonical code mapping

- [ ] Implement all code/config mappings from Phase 0.
- [ ] Ensure code/config documents receive correct `type`.
- [ ] Ensure language matches extension.
- [ ] Ensure secret patterns are never indexed.

## 9.2 Code analyzer

- [ ] Verify `code_subword_filter`.
- [ ] Verify camelCase split.
- [ ] Verify snake_case split.
- [ ] Verify original token preservation.
- [ ] Verify lowercasing.
- [ ] Verify numeric splitting behavior.
- [ ] Verify flattened graph.
- [ ] Test:
  - [ ] `authenticateUser`
  - [ ] `authenticate_user`
  - [ ] `authenticate user`
- [ ] Ensure exact identifier match still has strongest relevance.

## 9.3 Code search query logic

- [ ] Add `title.code`.
- [ ] Add `content.code`.
- [ ] Balance code field boosts with normal text fields.
- [ ] Keep normal free-text behavior.
- [ ] Do not require a separate code-search syntax.
- [ ] Ensure `language:` and `type:` filters continue working.

## 9.4 Code highlight

- [ ] Return source snippet.
- [ ] Preserve indentation.
- [ ] Return accurate line number.
- [ ] Highlight matched identifier/term.
- [ ] Verify line number still points into raw source content.

## 9.5 Shiki integration

- [ ] Integrate Shiki for syntax highlighting.
- [ ] Map language values to Shiki languages.
- [ ] Provide fallback for unsupported language.
- [ ] Keep code as selectable text.
- [ ] Preserve whitespace.
- [ ] Avoid re-highlighting entire document on scroll if cacheable.

## 9.6 Virtualized Code Viewer

- [ ] Create `VirtualizedCodeViewer.tsx`.
- [ ] Use `@tanstack/react-virtual`.
- [ ] `estimateSize = 22px`.
- [ ] `overscan = 20`.
- [ ] Only render viewport + overscan rows.
- [ ] Display line numbers.
- [ ] Auto-scroll to target line.
- [ ] Highlight active line.
- [ ] Preserve horizontal whitespace.
- [ ] Add accessible region role/label.
- [ ] Ensure text selection works.

## 9.7 Preview integration

- [ ] Markdown preview uses `react-markdown` + GFM.
- [ ] Code preview uses virtualized viewer.
- [ ] File type determines renderer.
- [ ] Current search terms are highlighted.
- [ ] Result click passes target `line_number`.
- [ ] Preview automatically centers the target line.

## 9.8 Code performance

- [ ] Test 1,000 lines.
- [ ] Test 5,000 lines.
- [ ] Ensure DOM does not contain all 5,000 line nodes at once.
- [ ] Ensure scrolling remains responsive.
- [ ] Ensure highlight does not cause large rerender.
- [ ] Ensure selected row/preview remains stable.
- [ ] Avoid unnecessary Shiki work during scroll.

## 9.9 Code tests

- [ ] `authenticate user` -> `authenticateUser`.
- [ ] `authenticate user` -> `authenticate_user`.
- [ ] Exact identifier outranks fuzzy alternatives.
- [ ] Language detection.
- [ ] Type detection.
- [ ] Config file indexing.
- [ ] Secret file rejection.
- [ ] Code snippet line number.
- [ ] Virtualization row count.
- [ ] Auto-scroll.
- [ ] Unsupported language fallback.

### Phase 8 Gate

- [ ] Code and config files are searchable.
- [ ] camelCase/snake_case identifier search works.
- [ ] Language/type filters work.
- [ ] Accurate line numbers are displayed.
- [ ] Shiki rendering works.
- [ ] Virtualized viewer works on large files.
- [ ] Code search behaves like normal free-text search.

---

# 10. Cross-Cutting Testing & Quality

## 10.1 Backend unit testing policy

- [ ] Unit tests are pure/in-memory where possible.
- [ ] One test = one behavior.
- [ ] Test names describe user-observable behavior.
- [ ] No dependency on wall-clock time.
- [ ] No dependency on test ordering.
- [ ] No dependency on leftover data.

## 10.2 Backend unit modules

### QueryParser
- [ ] free terms.
- [ ] quoted phrases.
- [ ] multiple filters.
- [ ] known filters.
- [ ] unknown filters.
- [ ] malformed filters.
- [ ] empty query.
- [ ] special characters.
- [ ] whitespace.
- [ ] case behavior.

### DocumentExtractor
- [ ] H1 title.
- [ ] filename fallback.
- [ ] front matter tags.
- [ ] type.
- [ ] language.
- [ ] project.
- [ ] binary detection.
- [ ] max file size.
- [ ] secret file rejection.
- [ ] encoding fallback.

### ScanPlanner
- [ ] add.
- [ ] update.
- [ ] delete.
- [ ] unchanged.
- [ ] skip.
- [ ] EXCLUDED.
- [ ] rename/move.
- [ ] idempotency.

### SearchQueryBuilder
- [ ] title/tag/content boosts.
- [ ] bool filter separation.
- [ ] fuzzy.
- [ ] prefix.
- [ ] highlight.
- [ ] facets.
- [ ] sort.
- [ ] pagination.
- [ ] size bounds.

## 10.3 Backend integration tests

- [ ] Use real PostgreSQL.
- [ ] Use `sqlx::test`.
- [ ] Each test gets isolated DB state.
- [ ] Use real Elasticsearch.
- [ ] Create dynamic index names `test_lynx_<uuid>`.
- [ ] Cleanup through RAII guard or equivalent.
- [ ] Ensure test cleanup runs after panic/failure where feasible.

## 10.4 Fixture tree

Create:

```text
crates/backend/tests/fixtures/knowledge_base/
├── notes/rust/ownership.md
├── src/auth/service.rs
├── configs/app.toml
├── assets/sample.bin
├── .git/HEAD
└── node_modules/dummy.js
```

- [ ] `ownership.md` has front-matter tags `rust`, `memory`.
- [ ] `service.rs` contains `authenticateUser`.
- [ ] `service.rs` contains `validate_token`.
- [ ] `app.toml` is valid config.
- [ ] `sample.bin` is detected as binary.
- [ ] `.git/HEAD` ignored.
- [ ] `node_modules/dummy.js` ignored.

## 10.5 Dynamic fixture generator

- [ ] Create `tests/common/generator.rs`.
- [ ] Use `tempfile::tempdir()`.
- [ ] Generate 1,000–5,000 files.
- [ ] Use deterministic contents when test needs determinism.
- [ ] Use generator for indexing/re-scan stress scenarios.

## 10.6 HTTP contract tests

- [ ] Use `tower::ServiceExt::oneshot`.
- [ ] Test endpoint success codes.
- [ ] Test 400.
- [ ] Test 403.
- [ ] Test 404.
- [ ] Test 409.
- [ ] Test 422.
- [ ] Test 500 mapping.
- [ ] Test 503 for dependency outage.
- [ ] Test structured error body.
- [ ] Test validation.
- [ ] Test CORS local response.
- [ ] Test tracing middleware does not break responses.

## 10.7 Frontend test setup

- [ ] Configure `vitest.config.ts`.
- [ ] Use `happy-dom`.
- [ ] Configure `setupFiles`.
- [ ] Coverage provider = v8.
- [ ] Reports = text/json/html.
- [ ] Exclude test utilities from coverage.

## 10.8 MSW

- [ ] Mock `/api/health`.
- [ ] Mock `/api/search`.
- [ ] Mock empty search.
- [ ] Mock 500 search.
- [ ] Add folder endpoints.
- [ ] Add job endpoints.
- [ ] Add document endpoint.
- [ ] Add settings endpoints.
- [ ] Add stats endpoint.
- [ ] Add suggest endpoint.
- [ ] Keep handler responses aligned with actual backend DTOs.

## 10.9 Four micro-state tests

For every major data component:

- [ ] Loading.
- [ ] Empty.
- [ ] Error.
- [ ] Success.

At minimum test:

- [ ] ResultList.
- [ ] FacetSidebar.
- [ ] DocumentPreview.
- [ ] FolderManager.
- [ ] JobProgress.
- [ ] Settings.
- [ ] Stats.
- [ ] Health/connection banner.

## 10.10 Code viewer tests

- [ ] Virtualizer mounts only viewport + overscan rows.
- [ ] Target line receives highlight class.
- [ ] `scrollToIndex()` called for valid target.
- [ ] Invalid line does not crash.
- [ ] 5,000-line fixture remains interactive.

## 10.11 Accessibility tests

- [ ] Run `vitest-axe`.
- [ ] SearchBar has no violations.
- [ ] Dialogs have proper labels.
- [ ] Icon buttons have labels.
- [ ] Focus ring visible.
- [ ] Keyboard-only interaction test.
- [ ] Reduced motion styles verified.

## 10.12 Coverage gate

- [ ] Domain tests achieve required coverage target.
- [ ] Store/helper coverage achieves required 80% target.
- [ ] Do not game coverage by testing implementation branches without behavior.

---

# 11. Performance & Build Hardening

## 11.1 Backend target measurements

At ~5,000 files, treat these as indicative goals:

- [ ] Typical search backend latency < 200 ms where practical.
- [ ] Import 5,000 files completes in minutes rather than unbounded duration.
- [ ] No-change rescan completes in seconds rather than full reindex.

> Target performance is indicative, not a contractual benchmark.

## 11.2 File ingestion memory safety

- [ ] Verify semaphore prevents uncontrolled `spawn_blocking`.
- [ ] Verify huge folders do not spawn one blocking task per file without limit.
- [ ] Verify batch processing does not hold all file contents in memory.

## 11.3 Search performance

- [ ] Use one local ES shard.
- [ ] Use `bool.filter` for exact facets.
- [ ] Avoid fetching unused `_source` fields.
- [ ] Keep autocomplete endpoint lightweight.
- [ ] Keep pagination bounded.
- [ ] Preserve alias-based search.

## 11.4 Frontend performance

- [ ] Lazy load:
  - [ ] Folder Manager.
  - [ ] Settings.
  - [ ] Document Preview.
- [ ] Keep search shell in initial bundle.
- [ ] Run visualizer.
- [ ] Inspect vendor chunk composition.
- [ ] Ensure gzip JS < 450 KB.
- [ ] Ensure gzip CSS < 50 KB.
- [ ] Verify code viewer does not load all rows.

## 11.5 CLS / layout stability

- [ ] Result skeleton dimensions match result cards.
- [ ] Preview pane does not jump when data loads.
- [ ] Autocomplete overlay does not resize main layout.
- [ ] Health/status changes do not shift search input.

---

# 12. Security & Reliability Hardening

## 12.1 Localhost isolation

- [ ] Backend defaults to `127.0.0.1:3001`.
- [ ] PostgreSQL defaults to `127.0.0.1:5432`.
- [ ] Elasticsearch defaults to `127.0.0.1:9200`.
- [ ] Do not bind backend/DB/ES to `0.0.0.0`.
- [ ] Verify from another LAN host that services are not exposed by default.

## 12.2 Path traversal

For `/api/documents/:id`:

- [ ] Resolve document via UUID.
- [ ] Read absolute path from registry/folder context.
- [ ] Canonicalize document path.
- [ ] Canonicalize root path.
- [ ] Verify document path starts with canonical root.
- [ ] Reject path containing `..`.
- [ ] Reject null bytes.
- [ ] Return `PATH_TRAVERSAL_DETECTED`.
- [ ] Add negative tests for symlink/escape scenarios that are feasible in the primary OS.

## 12.3 Secret exclusion

- [ ] Verify secret filenames never enter registry as indexable documents.
- [ ] Verify `.env`, keys, certificates, private key names are skipped.
- [ ] Verify UI can explain skipped reason in job summary.

## 12.4 API exposure

- [ ] Restrict CORS to local desktop origin/config needed.
- [ ] Validate all input at API boundary.
- [ ] Do not trust path or filename from client.
- [ ] Do not expose absolute disk paths unnecessarily in public-facing result DTOs if not needed.

## 12.5 Log safety

- [ ] Do not log full secret content.
- [ ] Do not log credentials.
- [ ] Do not log full file content.
- [ ] Include request/job/folder IDs.
- [ ] Include query latency.
- [ ] Include useful failure reason.

## 12.6 Log persistence

- [ ] Linux log path under config directory.
- [ ] Windows log path under `%APPDATA%`.
- [ ] macOS log path under Application Support.
- [ ] Daily rolling.
- [ ] Non-blocking appender.
- [ ] Cleanup logs older than 7 days on startup.

---

# 13. Observability

## 13.1 Tracing configuration

- [ ] Development -> pretty logs.
- [ ] Release -> structured JSON.
- [ ] Use `RUST_LOG`.
- [ ] Ensure backend logs include timestamp.

## 13.2 Request spans

- [ ] Trace each request.
- [ ] Include request_id.
- [ ] Search logs include query context where safe.
- [ ] Include `took_ms`.
- [ ] Error logs include structured error code.

## 13.3 Job spans

- [ ] Include job_id.
- [ ] Include folder_id.
- [ ] Log start.
- [ ] Log file skip/failure.
- [ ] Log completion summary.
- [ ] Log cancellation.
- [ ] Log panic recovery.

## 13.4 Internal metrics APIs

- [ ] `/api/health` exposes dependency state/latency.
- [ ] `/api/stats` exposes:
  - [ ] total document count
  - [ ] total/index size as contract defines
  - [ ] distribution by type
  - [ ] distribution by language
- [ ] Keep this internal to local app; no separate Prometheus service is required by scope.

---

# 14. Final API Contract Verification Matrix

## 14.1 Health

- [ ] `GET /api/health` -> `200`.
- [ ] `GET /api/health/live` -> `200`.
- [ ] `GET /api/health/ready` -> `200` when DB+ES ready.
- [ ] `GET /api/health/ready` -> `503` when dependency unavailable.

## 14.2 Folders

- [ ] `GET /api/folders`.
- [ ] `POST /api/index/folder`.
- [ ] `DELETE /api/folders/:id`.

## 14.3 Index jobs

- [ ] `GET /api/index/jobs/:id`.
- [ ] `POST /api/index/jobs/:id/cancel`.
- [ ] `POST /api/index/rebuild`.
- [ ] `POST /api/index` for restoring an excluded single document.

## 14.4 Search

- [ ] `GET /api/search`.
- [ ] Basic query.
- [ ] Advanced filters.
- [ ] Facets.
- [ ] Fuzzy.
- [ ] Prefix.
- [ ] Sorting.
- [ ] Pagination.
- [ ] Highlight.
- [ ] Warning response.

## 14.5 Suggest

- [ ] `GET /api/suggest`.

## 14.6 Documents

- [ ] `GET /api/documents/:id`.
- [ ] `DELETE /api/documents/:id`.

## 14.7 Settings/stats

- [ ] `GET /api/settings`.
- [ ] `PUT /api/settings`.
- [ ] `GET /api/stats`.

---

# 15. Final Desktop UX Verification Matrix

- [ ] 3-pane layout matches design.
- [ ] Top app bar.
- [ ] Global search.
- [ ] Backend status indicator.
- [ ] Folder manager.
- [ ] Settings.
- [ ] Facet sidebar.
- [ ] Results list.
- [ ] Preview panel.
- [ ] Resizable preview.
- [ ] Collapsible sidebar.
- [ ] Collapsible preview.
- [ ] Search result score.
- [ ] Search latency.
- [ ] Highlight.
- [ ] Line number.
- [ ] Open in editor.
- [ ] Copy path.
- [ ] Job progress.
- [ ] Job cancellation.
- [ ] Toasts.
- [ ] Relative timestamps.
- [ ] Empty state.
- [ ] Loading state.
- [ ] Error state.
- [ ] Success state.
- [ ] Keyboard-only navigation.
- [ ] Reduced motion.
- [ ] Accessibility labels.

---

# 16. Final Release Gate

## 16.1 Source hygiene

- [ ] No `.env` tracked.
- [ ] No real credentials in source.
- [ ] No generated build artifacts tracked unintentionally.
- [ ] No debug-only hardcoded localhost overrides that bypass config.
- [ ] No duplicate dependency versions contradicting architecture.
- [ ] No Elasticsearch major-version mismatch comments remain.

## 16.2 Rust gates

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

- [ ] All pass with exit code 0.

## 16.3 Frontend gates

```bash
npm run lint
npm run typecheck
npm run test:coverage
npm run build
```

- [ ] All pass with exit code 0.

## 16.4 Bundle gate

```bash
python3 scripts/bundle-budget-checker.py --max-js 450 --max-css 50
```

- [ ] JS gzip < 450 KB.
- [ ] CSS gzip < 50 KB.

## 16.5 Docker smoke test

- [ ] `docker compose up -d`.
- [ ] PostgreSQL healthy.
- [ ] Elasticsearch healthy.
- [ ] Run migrations.
- [ ] Start backend.
- [ ] Start desktop.
- [ ] Stop backend.
- [ ] Desktop displays offline state.
- [ ] Start backend again.
- [ ] Desktop recovers automatically through polling/refetch.

## 16.6 End-to-end manual smoke flow

- [ ] Add folder.
- [ ] Initial import starts.
- [ ] Progress is shown.
- [ ] Job completes.
- [ ] Search a common keyword.
- [ ] Open result preview.
- [ ] Copy path.
- [ ] Open file in editor.
- [ ] Re-scan without changes.
- [ ] Verify almost everything is skipped.
- [ ] Modify one file.
- [ ] Re-scan.
- [ ] Verify only changed document updates.
- [ ] Delete one file.
- [ ] Re-scan.
- [ ] Verify document disappears.
- [ ] Rename/move one file.
- [ ] Re-scan.
- [ ] Verify no duplicate.
- [ ] Exclude one document.
- [ ] Re-scan.
- [ ] Verify it stays excluded.
- [ ] Restore excluded document through supported flow.
- [ ] Rebuild index.
- [ ] Search during rebuild.
- [ ] Verify search remains available.
- [ ] Verify alias switches after successful rebuild.
- [ ] Try fuzzy typo.
- [ ] Try prefix.
- [ ] Try inline filter.
- [ ] Click facet.
- [ ] Verify facet/query synchronization.
- [ ] Change BM25 weights.
- [ ] Verify ranking changes.
- [ ] Search `authenticate user`.
- [ ] Verify camelCase/snake_case matches.
- [ ] Open large code file.
- [ ] Verify virtualized viewer.
- [ ] Verify target line auto-scroll.

## 16.7 Final documentation

- [ ] README setup from clean clone.
- [ ] Explain architecture.
- [ ] Explain source-of-truth boundaries.
- [ ] Explain local Docker dependencies.
- [ ] Explain how to run backend independently.
- [ ] Explain how to run desktop independently.
- [ ] Explain indexing semantics.
- [ ] Explain EXCLUDED tombstone.
- [ ] Explain alias rebuild.
- [ ] Explain query syntax.
- [ ] Explain filter keys.
- [ ] Explain supported file types.
- [ ] Explain secret exclusion.
- [ ] Explain keyboard shortcuts.
- [ ] Explain test strategy.
- [ ] Explain known out-of-scope items.
- [ ] Document any deviations made from the three source docs.

---

# 17. Traceability Matrix — PRD User Stories → Implementation Tasks

> Semua 79 user story harus mempunyai coverage task. Nomor di bawah mengikuti urutan PRD.

| US | Requirement ringkas | Covered by |
|---:|---|---|
| 01 | Docker Compose menyalakan PostgreSQL + ES | F1.7, Final 16.5 |
| 02 | Backend proses terpisah | F1.4, F2.1 |
| 03 | Desktop status backend | F6.16 |
| 04 | Health DB/ES/backend terpisah | F2.8, F12.3 |
| 05 | Error dependency jelas | F2.4, F6.16 |
| 06 | Backend lokal-only | F2.8, F12.1 |
| 07 | Native folder picker | F6.12, F7.21 |
| 08 | Multiple root folders | F4.17, F6.12 |
| 09 | List folder + last scan + count | F4.17, F6.12 |
| 10 | Index progress | F4.16, F6.13 |
| 11 | Background indexing | F4.10, F4.21 |
| 12 | Ignore `.git`, `node_modules`, `target`, `dist` | F4.2 |
| 13 | Skip binary/oversized | F4.4, F4.5 |
| 14 | Final job summary | F4.11, F6.13 |
| 15 | One bad file doesn't kill job | F4.14 |
| 16 | Re-scan button | F4.17, F6.12 |
| 17 | Incremental rescan | F4.8, F4.12 |
| 18 | Delete missing source from index | F4.12 |
| 19 | Rename/move detection | F4.8, F4.13 |
| 20 | Delete folder + docs from index | F4.17 |
| 21 | Delete single indexed document without deleting file | F4.18 |
| 22 | Rebuild whole index | F4.20 |
| 23 | Rescan idempotent | F4.8, F4.22 |
| 24 | One active indexing job per folder | F4.15 |
| 25 | Relevance-ranked search | F5.2, F5.3 |
| 26 | Search title/tags/content | F5.2 |
| 27 | Title/tag higher boost | F5.2 |
| 28 | Show relevance score | F5.7, F6.10 |
| 29 | Total + search time | F5.7, F6.10 |
| 30 | Pagination | F5.2, F5.7 |
| 31 | Helpful no-results state | F6.19 |
| 32 | Empty/spaces handled | F0.5, F5.1, F5.7 |
| 33 | Special characters safe | F5.1, F5.8 |
| 34 | Case-insensitive search | F5.1 |
| 35 | Title/path/snippet | F5.8, F6.10 |
| 36 | Highlight matches | F5.4, F6.10 |
| 37 | Multiple fragments | F5.4 |
| 38 | Click result -> preview | F6.11 |
| 39 | Markdown + monospace code | F6.11, F8.6 |
| 40 | Highlight in full preview | F6.11, F8.7 |
| 41 | Copy path | F6.11, F7.21 |
| 42 | Open original file | F6.11, F7.21 |
| 43 | Inline filter syntax | F7.1, F7.3 |
| 44 | All filter keys | F7.4 |
| 45 | Combine filters | F7.4 |
| 46 | Invalid filter warning | F7.1, F7.2 |
| 47 | Facet counts | F7.5, F7.6 |
| 48 | Click facet | F7.6 |
| 49 | Two-way facet/query sync | F7.7 |
| 50 | Facets reflect active filters | F7.5 |
| 51 | Fuzzy typo | F7.8 |
| 52 | Exact > fuzzy relevance | F7.8 |
| 53 | Prefix search | F7.9 |
| 54 | Autocomplete | F7.10, F7.11 |
| 55 | Responsive autocomplete | F7.11 |
| 56 | Debounced autocomplete | F7.11 |
| 57 | Sort relevance/modified/name | F7.12 |
| 58 | Sort persists across pagination | F7.12 |
| 59 | Source code indexed | F4.4, F9.1 |
| 60 | camelCase/snake_case | F9.2, F9.3 |
| 61 | Code highlight + line number | F9.4 |
| 62 | Language detection | F4.4, F9.1 |
| 63 | Config files indexed | F9.1 |
| 64 | Code/doc type filter | F9.1, F8.4 |
| 65 | Monospace + indentation | F9.6 |
| 66 | Code search uses normal text input | F9.3 |
| 67 | Index statistics | F3.4, F6.15 |
| 68 | Max file size setting | F1.8, F6.14, F4.4 |
| 69 | Ignore patterns setting | F1.8, F6.14, F4.2 |
| 70 | BM25 field weights setting | F8.13, F6.14 |
| 71 | Persistent settings | F3.5, F8.13 |
| 72 | Rust logic separated from HTTP/ES | F2, F3, F4, architecture gates |
| 73 | ES concepts mapped to real milestones | F5 + F8 |
| 74 | See/compare relevance scores | F5.2, F5.8, F8.13 |
| 75 | Rebuildable ES | F4.20 |
| 76 | Core modules testable without ES | F10.1, F10.2 |
| 77 | Thin Tauri client | F6.4, F7 state/API |
| 78 | Structured backend logs | F13 |
| 79 | Architecture decision documentation | F1.1, F0.4, Final 16.7 |

---

# 18. Traceability Matrix — Architecture Modules → Task Coverage

| Architecture module | Covered by |
|---|---|
| `main.rs` | F2.1 |
| `lib.rs` | F2.1 |
| `config.rs` | F2.2 |
| `error.rs` | F2.4 |
| `api/routes.rs` | F2.6 |
| `api/handlers/*` | F2.6, F4, F5, F8 |
| `api/dtos/*` | F1.6, F2.5, F5.8 |
| `api/middlewares/*` | F2.7 |
| `application/commands/*` | F4, F8 |
| `application/queries/*` | F5, F8 |
| `orchestrator/worker.rs` | F4.10 |
| `orchestrator/queue.rs` | F4.10 |
| `orchestrator/tracker.rs` | F4.10 |
| `domain/events.rs` | F4.1 |
| `domain/models/*` | F3.1, F4 |
| `domain/services/query_parser.rs` | F8.1 |
| `domain/services/document_extractor.rs` | F4.4 |
| `domain/services/scan_planner.rs` | F4.8 |
| `domain/services/query_builder.rs` | F5.2, F8.3 |
| `domain/ports/*` | F3.7 |
| `infrastructure/elasticsearch/*` | F3.8–F3.13 |
| `infrastructure/postgres/*` | F3.6–F3.8 |
| `infrastructure/fs/*` | F4.2–F4.3 |
| Tauri runtime/config | F1.10, F7.21 |
| `SearchBar` | F6.9 |
| `AutocompletePopover` | F8.11 |
| `ResultList` | F6.10 |
| `ResultCard` | F6.10 |
| `CodeSnippet` | F6.10, F9.4 |
| `DocumentPreview` | F6.11 |
| `MarkdownViewer` | F6.11 |
| `VirtualizedCodeViewer` | F9.6 |
| `FolderManagerModal` | F6.12 |
| `JobProgressIndicator` | F6.13 |
| `SettingsModal` | F6.14 |
| `BM25WeightSliders` | F6.14 |
| `ErrorBoundary` | F6.16 |
| `ConnectionBanner` | F6.16 |
| `OfflineFallback` | F6.16 |
| `useSearchStore` | F6.6, F8.7 |
| `useUIStore` | F6.6 |
| `useDebounce` | F6.9, F8.11 |
| `useHotkeys` | F6.20 |
| API client | F6.4 |
| Zod types/schema | F6.5 |
| Bundle visualizer/budget | F2.9, F11 |

---

# 19. Definition of Done Global

Sistem hanya boleh dianggap **selesai** ketika seluruh kondisi berikut benar:

- [ ] 79 user stories memiliki implementation/test coverage.
- [ ] Tidak ada blocking reconciliation issue yang belum diselesaikan.
- [ ] Backend dan desktop tetap terpisah sebagai proses.
- [ ] PostgreSQL + Elasticsearch adalah satu-satunya container runtime.
- [ ] Domain layer murni dan bisa diuji tanpa I/O.
- [ ] Indexing incremental, idempotent, resilient, cancellable.
- [ ] Rename/move tidak membuat duplikasi.
- [ ] EXCLUDED tombstone mencegah resurrection.
- [ ] Rebuild memakai alias swap.
- [ ] Search basic + advanced + code search seluruhnya berjalan.
- [ ] Facet dan filter inline sinkron.
- [ ] Autocomplete ter-debounce.
- [ ] Code search memahami camelCase/snake_case.
- [ ] Preview code memakai line number + virtualization.
- [ ] UI memenuhi 4 micro-states.
- [ ] Keyboard-first flow berfungsi.
- [ ] Accessibility checks lulus.
- [ ] Structured logging aktif.
- [ ] Startup recovery aktif.
- [ ] Worker supervisor aktif.
- [ ] Path traversal defense aktif.
- [ ] Secret file rejection aktif.
- [ ] Tests backend dan frontend lulus.
- [ ] Coverage gate lulus.
- [ ] Bundle budgets lulus.
- [ ] Dokumentasi setup dan decision record tersedia.
- [ ] Manual smoke test final lulus dari clean start.

---

# 20. Urutan Eksekusi Singkat

```text
FASE 0
  └── Reconcile + Freeze Contracts
        ↓
FASE 1
  └── Workspace + Docker + Tooling
        ↓
FASE 2
  └── Backend Core + API Skeleton + Health + Errors
        ↓
FASE 3
  └── PostgreSQL + Elasticsearch + Repositories
        ↓
FASE 4
  └── Extractor + Planner + Orchestrator + Jobs + Rebuild
        ↓
FASE 5
  └── Basic Search + BM25 + Highlight + Pagination
        ↓
FASE 6
  └── Tauri UI + Preview + Folder Manager + Settings + Stats
        ↓
FASE 7
  └── Parser + Filters + Facets + Fuzzy + Prefix + Autocomplete + Sort + BM25 tuning
        ↓
FASE 8
  └── Code Analyzer + Code Search + Shiki + Virtualized Viewer
        ↓
FINAL RELEASE GATE
  └── Tests + Security + Performance + Bundle + Smoke Test + Docs
```

---

# 21. Catatan Eksekusi

- [ ] Setiap kali satu subsection selesai, commit perubahan dengan pesan yang menjelaskan **behavior**, bukan hanya file yang berubah.
- [ ] Jangan lanjut karena “kode sudah banyak”; lanjut hanya karena gate lulus.
- [ ] Jika test gagal akibat perubahan kontrak, perbaiki implementasi lebih dulu sebelum melemahkan test.
- [ ] Jika perubahan spesifikasi diperlukan, update dokumen sumber + task traceability sebelum implementasi baru.
- [ ] Simpan hasil command penting pada issue/decision note bila ada kegagalan environment.
- [ ] Saat debugging Elasticsearch, selalu periksa alias, mapping, analyzer dan actual `_source` sebelum menyalahkan query builder.
- [ ] Saat debugging indexing, pisahkan masalah traversal → extraction → planning → bulk indexing → registry update → job state.
- [ ] Saat debugging UI, pisahkan masalah API client → query state → TanStack Query → component render → native Tauri integration.
