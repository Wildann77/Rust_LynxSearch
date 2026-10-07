# Arsitektur Sistem: LynxSearch
**Local Developer Knowledge Search Engine**

> **Status:** Approved Architecture Specification v2.0 (Comprehensive Audit Edition)  
> **Dasar PRD:** [PRD-LynxSearch.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/PRD-LynxSearch.md)  
> **Spesifikasi UI & Design System:** [DESIGN.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md)  
> **Cakupan Fase:** Fase 1 – 8 (MVP + Advanced Search + Code Search)  

---

### Tech Stack Hierarchy
```text
LynxSearch
│
├── Desktop (Tauri v2 Native Host & Webview)
│   ├── Tauri 2.12 Host & Runtime
│   │   ├── @tauri-apps/api (Core IPC, window & app lifecycle)
│   │   ├── @tauri-apps/plugin-dialog (Native folder picker)
│   │   ├── @tauri-apps/plugin-shell (Open in external editor)
│   │   ├── @tauri-apps/plugin-window-state (Window size/position persistence)
│   │   ├── @tauri-apps/plugin-single-instance (Single instance lock)
│   │   └── @tauri-apps/plugin-clipboard-manager (Native OS clipboard integration)
│   ├── Frontend Architecture
│   │   ├── React 19.3 & TypeScript 5.8+
│   │   ├── Vite 8.1 (ESBuild, HMR, manual chunking)
│   │   ├── Tailwind CSS 4.3 (@theme tokens, CSS variables)
│   │   ├── shadcn/ui & Radix UI primitives (@radix-ui/react-dialog, react-popover, react-scroll-area, react-tooltip, react-slider)
│   │   ├── Zod (Runtime schema & form validation)
│   │   ├── clsx, tailwind-merge, class-variance-authority (UI class utilities)
│   │   ├── lucide-react (Official icon set)
│   │   ├── sonner (Lightweight toast notification system)
│   │   ├── TanStack Query v5 (Server state, caching, polling)
│   │   ├── @tanstack/react-virtual (Virtualized code viewer & long results)
│   │   ├── Zustand 5 (Client UI & search state)
│   │   └── Native Intl.DateTimeFormat & RelativeTime (Zero-bundle date formatting)
│
├── Backend (Independent Native Rust Service)
│   ├── Rust 1.98.1 (Edition 2024)
│   ├── Web & Middleware
│   │   ├── Axum 0.8 (HTTP routing, extractors)
│   │   └── tower & tower-http (CORS, TraceLayer, compression, timeout)
│   ├── Search Engine Client
│   │   └── elasticsearch 8.19 (Official type-safe Elasticsearch 8.x client)
│   ├── Async & Concurrency
│   │   ├── Tokio 1.x (Async runtime, MPSC bounded worker queue, signal handling)
│   │   ├── tokio-util (CancellationToken for graceful drain)
│   │   ├── async-trait (Asynchronous repository contracts)
│   │   ├── futures & futures-util (Stream processing)
│   │   └── parking_lot & dashmap (Fast locks & concurrent in-memory job tracker)
│   ├── Data Persistence & Drivers
│   │   ├── SQLx 0.8.x (PostgreSQL driver, compile-time checked SQL, migrations)
│   │   └── chrono (UTC timestamps & PostgreSQL TIMESTAMPTZ mapping)
│   ├── File Ingestion & Parsing
│   │   ├── ignore (Directory traversal respecting .gitignore, .ignore, and hidden files)
│   │   ├── infer (Magic number binary file inspection)
│   │   ├── encoding_rs (Non-UTF-8 detection & safe decoding fallback)
│   │   ├── pulldown-cmark (Markdown AST parsing, H1 & text extractor)
│   │   └── serde_yaml (YAML front-matter tag parser)
│   ├── Hashing & Identifiers
│   │   ├── uuid (Deterministic v5 & random v4 with serde feature)
│   │   └── sha2 & hex (SHA-256 file content hashing)
│   ├── Serialization & Validation
│   │   ├── serde & serde_json (Optimized serialization, Box<RawValue>)
│   │   └── validator (Strict DTO validation & canonical path checks)
│   └── Observability & Configuration
│       ├── dirs (Cross-platform OS configuration & log path resolution)
│       ├── dotenvy (Zero-boilerplate .env environment loader)
│       ├── tracing & tracing-subscriber (Structured JSON/Pretty logging)
│       ├── tracing-appender (7-day daily rolling file log persistence)
│       └── thiserror (Hierarchical typed domain error taxonomy)
│
├── Database (Docker Compose)
│   └── PostgreSQL 18.6 (Metadata, Document Registry, Job History, Settings)
│
├── Search Engine (Docker Compose)
│   └── Elasticsearch 8.19.22 (Inverted Index, Custom Code Analyzer, BM25, Facets)
│
├── Document Rendering
│   ├── react-markdown 10.x & remark-gfm (Markdown AST parsing & GitHub Flavored Markdown)
│   └── Shiki (Fine-grained syntax highlighting with line numbers)
│
└── Infrastructure & Tooling
    ├── Docker Compose (PostgreSQL 18.6-alpine + Elasticsearch 8.19.22)
    ├── Git (.gitignore & .env.example)
    ├── tempfile (Temporary filesystem fixture creation for integration benchmarks)
    ├── cargo-nextest (Next-generation high-speed test runner)
    └── rollup-plugin-visualizer (Bundle size analysis)
```

---

## 1. Ringkasan Eksekutif & Prinsip Desain

### 1.1 Visi Sistem
LynxSearch adalah search engine desktop lokal berkinerja tinggi yang dirancang khusus untuk knowledge base developer (Markdown notes, dokumentasi teknis, file konfigurasi, dan source code). Sistem membedakan secara tegas antara **pencarian teks biasa** dan **code search** (mengenali konvensi `camelCase` dan `snake_case`, highlight baris kode akurat, toleransi salah ketik tanpa merusak peringkat relevansi BM25).

### 1.2 Batasan Arsitektural Kunci
1. **Dua Proses Mandiri (Two Independent Processes)**: Backend Rust (Axum 0.8) dan Desktop Client (Tauri 2.12) berjalan sebagai proses terpisah di OS pengguna. Backend bukan embedded sidecar, sehingga dapat di-debug, di-restart, dan diuji secara independen.
2. **Docker Khusus Data Store**: Docker Compose hanya digunakan untuk mengorkestrasi PostgreSQL 18.6 (metadata, registry, job history) dan Elasticsearch 8.19.22 (inverted index, search, facet aggregation). Backend dan Tauri berjalan native di host OS.
3. **Pemisahan Sumber Kebenaran (Single Source of Truth)**:
   - **File di Disk**: Sumber kebenaran mutlak isi dokumen (konten tidak diduplikasi penuh ke PostgreSQL).
   - **PostgreSQL 18.6**: Sumber kebenaran metadata (folder terdaftar, hash isi SHA-256 dokumen, registry status, riwayat job, konfigurasi bobot).
   - **Elasticsearch 8.19.22**: Index pencarian yang *disposable* (dapat dibangun ulang kapan saja secara idempoten dari file di disk + PostgreSQL).
4. **Clean Hexagonal Architecture / Ports & Adapters**: Lapisan domain murni bebas dari framework HTTP, database driver, dan client search engine.
5. **Keamanan Ketat Lokal**: Default bind ke `127.0.0.1:3001`, sanitasi path transversal, zero hardcoded credentials, validasi env via `.env`.

```mermaid
graph TD
    subgraph Host Machine [Host Machine: Linux / Windows / macOS]
        subgraph DesktopUI [apps/desktop: Tauri 2.12 + Webview]
            ReactApp[React 19.3 + TypeScript + Vite 8.1]
            ZustandStore[Zustand 5: Client UI State]
            TanStackQuery[TanStack Query v5: Server State & Polling]
            ShadcnUI[shadcn/ui + Tailwind CSS 4.3]
            VirtualViewer[@tanstack/react-virtual: Virtualized Lines]
            DocPreview[Document Preview: react-markdown 10.x + Shiki]
            ReactApp --> ZustandStore
            ReactApp --> TanStackQuery
            ReactApp --> ShadcnUI
            DocPreview --> VirtualViewer
            ReactApp --> DocPreview
        end

        subgraph BackendService [crates/backend: Rust 1.98.1 Service]
            AxumAPI[API Layer: Axum 0.8 Router & Handlers]
            AppUseCases[Application Layer: CQS Commands & Queries]
            DomainCore[Domain Layer: Pure Logic, Entities, Domain Events, Ports]
            WorkerQueue[Background Job Queue: Tokio 1.x MPSC Worker]
            
            AxumAPI --> AppUseCases
            AppUseCases --> DomainCore
            AppUseCases --> WorkerQueue
        end

        DesktopUI -- "HTTP REST (JSON) / 127.0.0.1:3001" --> AxumAPI
        DesktopUI -- "Tauri 2.12 Capabilities / IPC" --> TauriPlugin[Tauri Plugins: Dialog, Shell, WindowState, SingleInstance]
        TauriPlugin --> LocalFS[(Local File System)]
        BackendService -- "File Walker, Read & SHA-256" --> LocalFS
    end

    subgraph DockerContainer [Docker Compose Services]
        Postgres[(PostgreSQL 18.6: Metadata, Registry, Jobs)]
        Elastic[(Elasticsearch 8.19.22: Inverted Index, BM25, Facets)]
    end

    BackendService -- "SQLx 0.8.x Pool (TCP)" --> Postgres
    BackendService -- "Elasticsearch REST Client (HTTP)" --> Elastic
```

---

### 1.3 Blueprint Resmi Docker Compose (`docker/docker-compose.yml`)

Docker Compose secara eksklusif mengorkestrasi PostgreSQL 18.6-alpine dan Elasticsearch 8.19.22:
- **PostgreSQL 18.6-alpine**: Memanfaatkan peningkatan async I/O engine dan native UUID storage modern yang stabil.
- **Elasticsearch 8.19.22**: Menggunakan rilis stabil 8.19.x untuk menjaga aturan kompatibilitas ketat (*major version client = major version server*) dengan official Rust crate `elasticsearch = "8.19"` (versi crate 9.x saat ini masih berstatus alpha dengan risiko breaking changes).
- **Alokasi Heap**: Terkendali (512 MB) agar tidak membebani memori mesin host developer:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:18.6-alpine
    container_name: lynx_postgres
    restart: unless-stopped
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-lynx}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?Error: POSTGRES_PASSWORD environment variable is required}
      POSTGRES_DB: ${POSTGRES_DB:-lynxsearch}
    ports:
      - "127.0.0.1:5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-lynx} -d ${POSTGRES_DB:-lynxsearch}"]
      interval: 5s
      timeout: 5s
      retries: 5

  elasticsearch:
    image: docker.elastic.co/elasticsearch/elasticsearch:8.19.22
    container_name: lynx_elasticsearch
    restart: unless-stopped
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=false
      - "ES_JAVA_OPTS=-Xms512m -Xmx512m" # Batas heap eksplisit agar PC developer tidak lag
    ports:
      - "127.0.0.1:9200:9200"
    volumes:
      - esdata:/usr/share/elasticsearch/data
    healthcheck:
      test: ["CMD-SHELL", "curl -s http://localhost:9200/_cluster/health | grep -q '\"status\":\"green\"\\|\"status\":\"yellow\"'"]
      interval: 10s
      timeout: 5s
      retries: 6

volumes:
  pgdata:
    name: lynx_pgdata
  esdata:
    name: lynx_esdata
```

---

## 2. Struktur Monorepo & Topologi Modul

Sistem menggunakan **Cargo Workspace Monorepo** untuk menyatukan backend Rust dan desktop frontend dalam satu repositori yang kohesif dengan dependensi terpusat:

```text
Rust_LynxSearch/
├── .agents/                        # Konfigurasi skill dan guidelines AI
├── docker/
│   └── docker-compose.yml          # PostgreSQL 18.6 + Elasticsearch 8.19.22
├── crates/
│   └── backend/                    # Core Axum 0.8 backend service (Rust 1.98.1, Edition 2024)
│       ├── Cargo.toml
│       ├── migrations/             # SQLx database migrations
│       │   ├── 0001_create_folders.sql
│       │   ├── 0002_create_document_registry.sql
│       │   ├── 0003_create_indexing_jobs.sql
│       │   └── 0004_create_settings.sql
│       └── src/
│           ├── main.rs             # Composition root & graceful shutdown handler
│           ├── config.rs           # Environment & configuration loader
│           ├── error.rs            # Application-wide error taxonomy (ThisError)
│           ├── api/                # Driving Adapter: HTTP Web Layer
│           │   ├── mod.rs
│           │   ├── routes.rs       # Axum router configuration
│           │   ├── handlers/       # Request handlers (destructured extractors)
│           │   │   ├── health.rs   # /api/health, /api/health/live, /api/health/ready
│           │   │   ├── search.rs
│           │   │   ├── suggest.rs
│           │   │   ├── folder.rs
│           │   │   ├── index.rs
│           │   │   ├── document.rs
│           │   │   └── settings.rs
│           │   ├── dtos/           # Request/Response DTOs (Serde optimized)
│           │   └── middlewares/    # Tracing, CORS, error interceptor
│           ├── application/        # Application Layer: CQS Commands & Queries
│           │   ├── mod.rs
│           │   ├── commands/       # State mutation use cases
│           │   │   ├── register_folder.rs
│           │   │   ├── delete_folder.rs
│           │   │   ├── trigger_scan.rs
│           │   │   ├── cancel_job.rs
│           │   │   ├── rebuild_index.rs
│           │   │   └── update_settings.rs
│           │   ├── queries/        # Read-only query use cases
│           │   │   ├── search_documents.rs
│           │   │   ├── suggest_queries.rs
│           │   │   ├── list_folders.rs
│           │   │   ├── get_job_status.rs
│           │   │   ├── get_document_content.rs
│           │   │   └── get_stats.rs
│           │   └── orchestrator/   # Job background execution
│           │       ├── mod.rs
│           │       ├── worker.rs   # Tokio MPSC worker consumer with cancellation token
│           │       ├── queue.rs    # Job dispatcher & folder lock manager
│           │       └── tracker.rs  # In-memory progress tracking (DashMap + Mutex)
│           ├── domain/             # Core Domain Layer: Pure Business Logic
│           │   ├── mod.rs
│           │   ├── events.rs       # Domain Events (IndexingJobStarted, DocumentIndexed, etc.)
│           │   ├── models/         # Entities & Value Objects
│           │   │   ├── folder.rs
│           │   │   ├── document.rs
│           │   │   ├── job.rs
│           │   │   ├── query.rs
│           │   │   └── settings.rs
│           │   ├── services/       # Pure domain algorithms (No I/O)
│           │   │   ├── query_parser.rs       # Raw text -> Structured AST
│           │   │   ├── document_extractor.rs # File bytes -> ExtractedDoc
│           │   │   ├── scan_planner.rs       # Diff FS vs Registry -> ScanPlan
│           │   │   └── query_builder.rs      # Structured Query -> ES Query DSL
│           │   └── ports/          # Trait definitions (ISP segregated)
│           │       ├── search_repository.rs
│           │       ├── folder_repository.rs
│           │       ├── registry_repository.rs
│           │       ├── job_repository.rs
│           │       ├── settings_repository.rs
│           │       └── file_system.rs
│           └── infrastructure/     # Driven Adapters: External Tech Implementations
│               ├── mod.rs
│               ├── elasticsearch/  # Elasticsearch 8.x client adapter
│               │   ├── client.rs
│               │   ├── schema.rs   # Mapping & analyzer definitions
│               │   └── repository.rs
│               ├── postgres/       # SQLx PostgreSQL adapter (split repositories)
│               │   ├── connection.rs
│               │   ├── folder_repo.rs
│               │   ├── registry_repo.rs
│               │   ├── job_repo.rs
│               │   └── settings_repo.rs
│               └── fs/             # Local file system adapter
│                   ├── walker.rs   # Ignore patterns & traversal
│                   └── reader.rs   # Content reading & SHA-256 hashing (spawn_blocking)
├── apps/
│   └── desktop/                    # Tauri 2.12 Desktop App
│       ├── package.json
│       ├── vite.config.ts          # Vite 8.1 config with manual chunks & React 19.3
│       ├── tsconfig.json
│       ├── components.json         # shadcn/ui configuration
│       ├── src-tauri/              # Tauri 2.12 Rust Host
│       │   ├── Cargo.toml
│       │   ├── tauri.conf.json     # Window, plugins, security config
│       │   ├── capabilities/       # Tauri v2 ACL capabilities
│       │   │   └── default.json    # Explicit plugin permissions
│       │   └── src/
│       │       └── main.rs         # Tauri runtime entry point
│       └── src/                    # React 19.3 Frontend
│           ├── main.tsx
│           ├── App.tsx
│           ├── index.css           # Tailwind CSS 4.3 @import & @theme tokens
│           ├── components/
│           │   ├── ui/             # shadcn/ui primitives (button, dialog, skeleton, etc.)
│           │   ├── layout/         # Header, FacetSidebar, 3-Pane ResizableSplitView
│           │   ├── search/         # SearchBar, AutocompletePopover
│           │   ├── results/        # ResultList, ResultCard, CodeSnippet
│           │   ├── preview/        # DocumentPreview, MarkdownViewer, VirtualizedCodeViewer
│           │   ├── folders/        # FolderManagerModal, JobProgressIndicator
│           │   ├── settings/       # SettingsModal, BM25WeightSliders
│           │   └── common/         # ErrorBoundary, ConnectionBanner, OfflineFallback
│           ├── hooks/              # Custom React hooks (useDebounce, useHotkeys)
│           ├── lib/                # API client, cn() helper, formatters
│           ├── stores/             # Zustand 5 stores (useSearchStore, useUIStore)
│           └── types/              # TypeScript interfaces mirror backend DTOs
├── Cargo.toml                      # Root Cargo Workspace definition
├── .env.example                    # Sample environment variables
├── .gitignore                      # Git ignore rules (build artifacts, .env)
└── PRD-LynxSearch.md
```

---

## 3. Desain Backend: Clean Hexagonal Architecture & Prinsip SOLID

Mengikuti prinsip **SOLID**, **Clean Architecture**, dan **Ports & Adapters**, lapisan dependency hanya boleh mengarah ke dalam (Inward Dependency Rule):

```mermaid
graph TD
    subgraph Driving Adapters
        HTTP[Axum 0.8 HTTP Handlers & Routes]
    end

    subgraph Application Layer [CQS Architecture]
        Commands[Commands: RegisterFolder, TriggerScan, CancelJob, Rebuild]
        Queries[Queries: SearchDocuments, GetContent, ListFolders, GetJobStatus]
        Orchestrator[Job Worker & Queue Orchestrator]
    end

    subgraph Domain Layer [Pure Rust - No Framework Dependencies]
        DomainModels[Entities: Folder, Document, Job, Query]
        DomainEvents[Domain Events: JobStarted, DocIndexed, DocSkipped, ScanFinished]
        DomainServices[Domain Services: QueryParser, ScanPlanner, Extractor, QueryBuilder]
        Ports[Ports / Traits: SearchRepo, FolderRepo, RegistryRepo, JobRepo, SettingsRepo]
    end

    subgraph Driven Adapters
        ESAdapter[Elasticsearch Adapter: REST Client]
        PGAdapter[PostgreSQL Adapter: SQLx 0.8.x]
        FSAdapter[File System Adapter: Walkdir & SHA-256]
    end

    HTTP --> Commands
    HTTP --> Queries
    Commands --> DomainServices
    Commands --> DomainModels
    Commands --> Ports
    Queries --> Ports
    Orchestrator --> Ports
    Orchestrator --> DomainServices
    Orchestrator --> DomainEvents

    ESAdapter -. implements .-> Ports
    PGAdapter -. implements .-> Ports
    FSAdapter -. implements .-> Ports
```

### 3.1 Domain Layer (Murni, Bebas I/O)

#### 1. Models & Value Objects
- **`DocumentId`**: UUIDv5 atau SHA-256 terderivasi secara deterministik dari `(FolderId, RelativePath)`. Menjamin idempotensi index.
- **`Folder`**: Root folder yang didaftarkan, path absolut, scan status, tanggal dibuat.
- **`DocumentMetadata`**: Path relatif, ukuran file, waktu modifikasi, SHA-256 hash isi, tipe file (`doc`, `code`, `config`), bahasa (`rust`, `typescript`, `markdown`, dll.), project name, tags front-matter.
- **`SearchQuery`**: Query terstruktur hasil parsing:
  ```rust
  pub(crate) struct SearchQuery {
      pub free_terms: Vec<String>,
      pub phrase_terms: Vec<String>,
      pub filters: HashMap<FilterKey, String>, // language, tag, project, extension, type
      pub warnings: Vec<String>,
  }
  ```
- **`FilterKey`**: Enum filter eksplisit:
  - `language`: `language == "rust"` (mencocokkan field `language` hasil deteksi ekstensi file).
  - `tag`: `tags contains "rust"` (mencocokkan array `tags` dari front-matter Markdown).
  - `project`: `project == "backend"` (mencocokkan field `project` hasil aturan domain folder).
  - `extension`: `extension == "md"` (mencocokkan field `extension`).
  - `type`: `type == "doc"` (mencocokkan field `type`: `doc`, `code`, `config`).

> [!IMPORTANT]
> **Aturan Domain Ekstraksi `project` & Fallback Root Files:**  
> Penentuan `project = nama direktori tingkat pertama di bawah folder root` adalah aturan domain murni aplikasi LynxSearch yang dieksekusi oleh `DocumentExtractor`. Elasticsearch hanya menerima field `project` sebagai keyword yang sudah diekstrak.
> - **File di Sub-direktori**: `root/rust/ownership.md` --> `project = Some("rust")`
> - **Fallback File di Root Direktori**: File yang berada tepat di root (contoh: `root/README.md` atau `root/LICENSE`) dipetakan menjadi `project = None` (direpresentasikan sebagai `null` di JSON dan Elasticsearch). Ini mencegah false matching saat pengguna memfilter pencarian `project:xyz`. File hidden/dotfiles (seperti `.gitignore`) secara bawaan dilewati oleh traversal engine `ignore`.
> Contoh struktur:
> ```text
> root/
> ├── README.md         --> project = None (null)
> ├── LICENSE           --> project = None (null)
> ├── rust/
> │   └── ownership.md  --> project = Some("rust")
> ├── react/
> │   └── hooks.md      --> project = Some("react")
> └── backend/
>     └── redis.md      --> project = Some("backend")
> ```

- **`IndexingJob`**: Entity status job (`id`, `folder_id: Option<FolderId>`, `job_type`, `status: Pending | Running | Completed | Failed | Cancelled`, `counters: total, added, updated, deleted, skipped, failed`, timestamp).

#### 2. Domain Events (Pemisahan Concern & Observabilitas)
```rust
pub(crate) enum DomainEvent {
    IndexingJobStarted { job_id: JobId, folder_id: Option<FolderId> },
    DocumentIndexed { job_id: JobId, path: String },
    DocumentSkipped { job_id: JobId, path: String, reason: String },
    DocumentFailed { job_id: JobId, path: String, error: String },
    IndexingJobCompleted { job_id: JobId, summary: JobSummary },
    IndexRebuilt { old_index: String, new_index: String },
}
```

#### 3. Segregated Repository Ports (Kepatuhan SOLID: Interface Segregation Principle)
Sesuai hasil audit ISP, port dipisahkan per aggregate boundary agar use case hanya bergantung pada kontrak yang dibutuhkannya:

```rust
#[async_trait::async_trait]
pub(crate) trait SearchRepository: Send + Sync {
    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), DomainError>;
    async fn bulk_index_documents(&self, docs: &[IndexedDocument]) -> Result<BulkIndexReport, DomainError>;
    async fn delete_document(&self, id: &DocumentId) -> Result<(), DomainError>;
    async fn delete_documents_by_folder(&self, folder_id: &FolderId) -> Result<u64, DomainError>;
    async fn search(&self, query_dsl: &serde_json::value::RawValue) -> Result<SearchRawResponse, DomainError>;
    async fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>, DomainError>;
    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), DomainError>;
    async fn ping(&self) -> Result<(), DomainError>;
}

#[async_trait::async_trait]
pub(crate) trait FolderRepository: Send + Sync {
    async fn create_folder(&self, folder: &Folder) -> Result<(), DomainError>;
    async fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, DomainError>;
    async fn list_folders(&self) -> Result<Vec<Folder>, DomainError>;
    async fn delete_folder(&self, id: &FolderId) -> Result<(), DomainError>;
    async fn update_last_scanned(&self, id: &FolderId) -> Result<(), DomainError>;
}

#[async_trait::async_trait]
pub(crate) trait DocumentRegistryRepository: Send + Sync {
    async fn get_entry(&self, doc_id: &DocumentId) -> Result<Option<RegistryEntry>, DomainError>;
    async fn list_by_folder(&self, folder_id: &FolderId) -> Result<Vec<RegistryEntry>, DomainError>;
    async fn upsert_entry(&self, entry: &RegistryEntry) -> Result<(), DomainError>;
    async fn delete_entries(&self, ids: &[DocumentId]) -> Result<u64, DomainError>;
}

#[async_trait::async_trait]
pub(crate) trait JobRepository: Send + Sync {
    async fn create_job(&self, job: &IndexingJob) -> Result<(), DomainError>;
    async fn update_progress(&self, id: &JobId, update: &JobProgressUpdate) -> Result<(), DomainError>;
    async fn get_job(&self, id: &JobId) -> Result<Option<IndexingJob>, DomainError>;
    async fn mark_cancelled(&self, id: &JobId) -> Result<(), DomainError>;
}

#[async_trait::async_trait]
pub(crate) trait SettingsRepository: Send + Sync {
    async fn get_settings(&self) -> Result<AppSettings, DomainError>;
    async fn update_settings(&self, settings: &AppSettings) -> Result<(), DomainError>;
}
```

#### 4. Domain Services: ScanPlanner & DocumentExtractor

##### A. `ScanPlanner` (Two-Phase Diffing & Rename/Move Detection - US #19)
Membandingkan status file pada disk dengan entri di PostgreSQL `document_registry` untuk mendeteksi perubahan secara presisi:
- **Phase 1 (Relative Path Matching & Exclusion Check)**:
  - File ada di disk dan ada di DB:
    - Jika status entri di DB adalah `EXCLUDED` $\rightarrow$ lewati (`to_skip` dengan alasan `UserExcluded`). Menjamin dokumen yang sengaja dihapus pengguna via API tidak ter-index ulang (*anti-ghost resurrection*).
    - Jika `modified_at` dan `file_size` sama $\rightarrow$ lewati (`Unchanged`).
    - Jika berbeda $\rightarrow$ hitung hash baru; jika hash berubah $\rightarrow$ jadwalkan `to_update`.
  - File ada di disk tetapi belum ada di DB $\rightarrow$ kandidat `to_add`.
  - File ada di DB (status `INDEXED`) tetapi tidak ditemukan di disk $\rightarrow$ kandidat `to_delete`.
- **Phase 2 (Content-Hash Matching & Move Detection dengan Deterministik UUIDv5)**:
  - Membandingkan `content_hash` antara kandidat `to_add` dengan kandidat `to_delete`.
  - Jika `hash(to_add) == hash(to_delete)`:
    - Mengingat `DocumentId` terderivasi deterministik sebagai `UUIDv5(FolderId, RelativePath)`, perubahan path menghasilkan `new_doc_id = UUIDv5(folder_id, new_path)`.
    - Petakan sebagai operasi atomik `to_move(old_doc_id, new_doc_id, old_path, new_path)`.
    - **Eksekusi**: Karena `document_registry` hanya menyimpan metadata jejak fisik (path, hash, ukuran, timestamp) dan tidak menyimpan teks lengkap atau AST, maka file di `new_path` dibaca ulang dari disk untuk ekstraksi metadata segar. Di database dan Elasticsearch, ID lama (`old_doc_id`) dihapus dan entri baru (`new_doc_id`) disimpan secara atomik.

##### B. `DocumentExtractor` (Robust Binary Detection & Multi-Encoding Fallback)
1. **Deteksi File Biner Multi-Lapisan (Multi-Tier Binary Detection)**:
   - *Tingkat 1 (Ekstensi Terdaftar)*: Cek daftar ekstensi biner (`.exe`, `.dll`, `.bin`, `.png`, `.jpg`, `.zip`, `.wasm`, `.pdf`, dll.) $\rightarrow$ langsung abaikan.
   - *Tingkat 2 (Magic Number Signatures via `infer`)*: Menggunakan crate `infer` untuk memeriksa 512 byte pertama file guna mendeteksi header biner (ELF, Mach-O, ZIP, SQLite, executable, dll.).
   - *Tingkat 3 (Null Byte Scan)*: Jika lolos dari tingkat 1 & 2, periksa 8 KB pertama file; jika mengandung karakter null byte (`\0`), file diklasifikasikan sebagai biner $\rightarrow$ lewati dengan event `DocumentSkipped { reason: "BinaryFileDetected" }`.
2. **Penanganan Multi-Encoding (Non-UTF-8 Fallback via `encoding_rs`)**:
   - File kode sumber atau dokumen lawas mungkin tersimpan dalam format encoding lokal (Windows-1252, GBK, atau Shift-JIS). Membaca file secara naif sebagai `String::from_utf8` di Rust akan memicu panic atau error fatal.
   - **Alur Decoding Aman**:
     1. Coba decode langsung sebagai UTF-8 murni (`std::str::from_utf8`).
     2. Jika gagal, gunakan crate `encoding_rs` untuk mendeteksi charset dan melakukan konversi bytes ke UTF-8 string secara aman (`encoding_rs::Encoding::decode_without_bom_handling`).
     3. Jika decoding tetap tidak valid atau terjadi korupsi karakter parah, lakukan fallback ke `String::from_utf8_lossy` dan catat peringatan `IndexingWarning::LossyEncodingDecoded` tanpa menghentikan worker indexing.

3. **Tabel Pemetaan Ekstensi Kanonikal (Type & Language Normalization)**:
   | Kategori (`type`) | Ekstensi File | Nilai `language` Terpetakan |
   |---|---|---|
   | `doc` | `.md`, `.markdown`, `.txt` | `markdown`, `text` |
   | `config` | `.json`, `.toml`, `.yaml`, `.yml`, `.ini` | `json`, `toml`, `yaml`, `ini` |
   | `code` | `.rs` | `rust` |
   | `code` | `.ts`, `.tsx`, `.js`, `.jsx` | `typescript`, `javascript` |
   | `code` | `.py` | `python` |
   | `code` | `.go` | `go` |
   | `code` | `.java`, `.kt` | `java`, `kotlin` |
   | `code` | `.c`, `.cpp`, `.h` | `c`, `cpp` |
   | `code` | `.cs` | `csharp` |
   | `code` | `.rb` | `ruby` |
   | `code` | `.php` | `php` |
   | `code` | `.swift` | `swift` |
   | `code` | `.sh` | `shell` |
   | `code` | `.sql` | `sql` |
   | `code` | `.html`, `.css`, `.scss` | `html`, `css` |
   | `code` | `.vue`, `.svelte`, `.lua` | `vue`, `svelte`, `lua` |
   | *Diblok Default* | `.env*`, `*.pem`, `*.key`, `id_rsa`, `*.p12` | *(Ditolak demi keamanan credential/secret)* |

---

### 3.2 Rust Backend Patterns & Concurrency Rules

Sesuai aturan `rust-backend` dan `clean-code-principles`:

1. **Aturan Mutex & Sinkronisasi**:
   - Gunakan `parking_lot::Mutex` untuk struktur data sinkron in-memory murni (misal: metrik, status flags).
   - Gunakan `tokio::sync::Mutex` **hanya** ketika lock harus dipertahankan melintasi titik `.await` (khususnya pada `folder_locks` di `JobTracker`).
2. **Penanganan JSON & Box<RawValue>**:
   - Gunakan `Box<serde_json::value::RawValue>` ketika mem-pass query DSL ke Elasticsearch atau membaca `settings.value` (JSONB) tanpa perlu modifikasi in-memory.
   - Hindari parsing penuh ke `serde_json::Value` kecuali saat melakukan inspeksi atau mutasi field.
3. **Optimasi Serde pada DTO**:
   ```rust
   #[derive(serde::Serialize, serde::Deserialize)]
   pub(crate) struct SearchResultDto {
       pub id: Uuid,
       pub title: String,
       pub relative_path: String,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub project: Option<String>,
       #[serde(skip_serializing_if = "Vec::is_empty", default)]
       pub tags: Vec<String>,
       #[serde(skip_serializing_if = "Vec::is_empty", default)]
       pub highlights: Vec<HighlightDto>,
       pub score: f32,
   }
   ```
4. **Signature Axum Handlers**:
   - Destrukturisasi extractor langsung di parameter fungsi:
   ```rust
   pub(crate) async fn search_documents(
       State(app_state): State<AppState>,
       Query(params): Query<SearchRequestDto>,
   ) -> Result<Json<SearchResponseDto>, AppError> { ... }
   ```
5. **CPU-Bound Offloading & I/O Semaphore Throttling**:
   - Jika pengguna memindai folder berukuran besar (misal: 100.000 file), memanggil `tokio::task::spawn_blocking` secara tidak terkontrol akan membanjiri thread pool Tokio dan memicu lonjakan memori drastis (*Out of Memory*) akibat terlalu banyak file dibaca secara bersamaan.
   - Gunakan `Arc<tokio::sync::Semaphore>` (default 50 permits) untuk membatasi konkurensi pembacaan file dan hashing SHA-256 secara ketat:
   ```rust
   let permit = semaphore.clone().acquire_owned().await
       .map_err(|_| AppError::Internal("Semaphore closed".into()))?;

   let file_content = tokio::task::spawn_blocking(move || {
       let _permit = permit; // Permit otomatis dilepas saat closure selesai
       std::fs::read(&file_path)
   }).await.map_err(|e| AppError::Internal(e.to_string()))??;
   ```
6. **Official Elasticsearch Client Adapter & Core Crate Standards**:
   - Menghindari inkonsistensi antara raw HTTP dan typed client. Menggunakan official crate `elasticsearch = "=8.19.0-alpha.1"` dengan `TransportBuilder`:
   ```rust
   let transport = Transport::single_node("http://127.0.0.1:9200")?;
   let client = Elasticsearch::new(transport);
   ```
   - Operasi search mengirimkan query DSL terstruktur via:
   ```rust
   let response = client
       .search(SearchParts::Index(&["lynx_documents"]))
       .body(query_dsl)
       .send()
       .await?;
   ```
   - **Struktur Pemisahan `lib.rs` dan `main.rs`**:
     - `crates/backend/src/lib.rs`: Menyediakan seluruh domain model, services, repositories, Axum router, middleware, handlers, dan config. Memungkinkan integration tests dijalankan secara langsung via `axum::serve` atau `tower::oneshot` tanpa perlu spawn child binary terpisah.
     - `crates/backend/src/main.rs`: Entrypoint tipis (thin entrypoint) yang hanya memuat konfigurasi (`dotenvy`), menginisialisasi logging/tracing, membuat listener socket TCP, dan meneruskan sinyal shutdown ke `lib.rs`.
   - **Standar Dependensi Crate Backend**:
     - *Hashing & Identifiers*: `sha2 = "0.10"`, `hex = "0.4"`, `uuid = { version = "1.10", features = ["v4", "v5", "serde"] }`.
     - *Waktu & Timestamp*: `chrono = { version = "0.4", features = ["serde"] }` (kompatibel penuh dengan SQLx TIMESTAMPTZ).
     - *Konfigurasi Environment*: `dotenvy = "0.15"` (memuat `.env` saat startup tanpa boilerplate).
     - *File Traversal & Gitignore*: `ignore = "0.4"` (menghormati `.gitignore`, `.ignore`, global gitignore, dan hidden files secara native).
     - *Parser Markdown & Front-Matter*: `pulldown-cmark = "0.12"` (ekstraksi heading H1 dan konten teks mentah), `serde_yaml = "0.9"` (ekstraksi front-matter tags).
     - *Deteksi Biner & Encoding*: `infer = "0.19"` (magic bytes file header), `encoding_rs = "0.8"` (safe multi-encoding conversion).
     - *Middleware HTTP & Async*: `tower = "0.5"`, `tower-http = { version = "0.6", features = ["cors", "trace", "fs", "timeout"] }`, `tokio-util = { version = "0.7", features = ["sync"] }` (`CancellationToken`), `async-trait = "0.1"`, `futures = "0.3"`, `dashmap = "6.1"` (in-memory lockless tracker).

---

### 3.3 Graceful Shutdown & In-Flight Job Drain

Backend mengimplementasikan penghentian bertahap yang aman saat menerima sinyal `SIGINT` (Ctrl+C) atau `SIGTERM`:

```mermaid
sequenceDiagram
    participant OS as OS Signal (Ctrl+C)
    participant Main as main.rs (Shutdown Coordinator)
    participant Axum as Axum HTTP Server
    participant Worker as Background Job Worker
    participant DB as PostgreSQL / Elasticsearch

    OS->>Main: SIGINT / SIGTERM Triggered
    Main->>Axum: Stop accepting new HTTP requests
    Main->>Worker: Trigger CancellationToken
    Worker->>Worker: Finish processing current batch (max 10s grace)
    Worker->>DB: Commit current indexed batch & set status = 'CANCELLED'
    Worker-->>Main: Batch drained & worker loop exited
    Main->>DB: Close SQLx Pool & HTTP connection pools
    Main-->>OS: Process Exit (Code 0)
```

#### 3.3.1 Startup Crash Recovery (Dangling Jobs Healing)
Saat proses backend pertama kali menyala (booting):
1. **Query Dangling Jobs**: Sistem mengeksekusi pembersihan pada database PostgreSQL:
   ```sql
   UPDATE indexing_jobs
   SET status = 'FAILED',
       completed_at = NOW(),
       error_summary = 'Server di-restart mendadak saat proses job berjalan'
   WHERE status IN ('RUNNING', 'PENDING');
   ```
2. **Reset Folder Status**: Seluruh folder yang statusnya tertinggal dalam kondisi `SCANNING` di-reset kembali menjadi `IDLE`. Ini mencegah deadlock in-memory di mana folder terkunci selamanya akibat crash backend sebelumnya.

1. **Step 1**: Server HTTP Axum berhenti menerima koneksi baru (`with_graceful_shutdown`).
2. **Step 2**: Koordinator memicu `CancellationToken` pada `JobTracker`.
3. **Step 3**: Background worker menghentikan pengambilan file baru, menyelesaikan commit batch dokumen yang sedang aktif berjalan ke Elasticsearch dan PostgreSQL (dibatasi grace period 10 detik).
4. **Step 4**: Status job yang belum selesai ditandai sebagai `CANCELLED` di database agar tidak berstatus `RUNNING` selamanya.
5. **Step 5**: Database connection pool ditutup secara rapi.

---

### 3.4 Background Worker Panic Resilience & Supervisor Loop

Jika terjadi bug atau kondisi tak terduga yang menyebabkan panic di dalam worker thread `IndexOrchestrator`, tanpa proteksi worker akan mati senyap dan status job di database akan terjebak selamanya sebagai `RUNNING`.

Sistem menerapkan arsitektur **Supervisor Task** yang memantau thread worker:

```mermaid
flowchart TD
    Supervisor[Worker Supervisor Loop] -->|Spawn| WorkerTask[Worker Tokio Task: Job Processor]
    WorkerTask -->|Active Processing| Process[Read File, Hash, Bulk Index]
    WorkerTask -.->|Uncaught Panic!| Panicked[Task Panics]
    Panicked -->|JoinHandle Err| Supervisor
    Supervisor -->|1. Catch Error| Catch[Catch Panic Payload & Log Trace]
    Supervisor -->|2. Mark Job FAILED| DB[(PostgreSQL: Update Job Status = FAILED)]
    Supervisor -->|3. Release Lock| Lock[JobTracker: Unlock Folder Mutex]
    Supervisor -->|4. Auto Restart| Restart[Restart Worker Task with Backoff]
```

1. **Panic Catching via Tokio JoinHandle**:
   Worker task dieksekusi di bawah pengawasan supervisor:
   ```rust
   let handle = tokio::spawn(worker_loop(rx, app_state.clone()));
   if let Err(join_err) = handle.await {
       if join_err.is_panic() {
           tracing::error!(
               error = ?join_err,
               "CRITICAL: Background worker panicked! Recovering state..."
           );
           // Eksekusi pemulihan
           recover_panicked_job(&app_state).await;
       }
   }
   ```
2. **State Recovery**:
   - Menandai job yang sedang aktif menjadi status `FAILED` di database PostgreSQL dengan pesan: `error_summary = "Internal Worker Panic: Process terminated unexpectedly"`.
   - Melepaskan lock `folder_locks` pada aggregate `JobTracker` agar folder tidak terkunci selamanya.
3. **Worker Auto-Restart**:
   - Supervisor secara otomatis menginisialisasi loop worker baru setelah jeda exponential backoff (1 detik) untuk terus melayani antrean job berikutnya.

---

### 3.5 Multi-Folder Concurrency & Worker Queue Architecture

Untuk mencegah antrean pemindaian terblokir saat satu folder berukuran besar sedang dipindai, worker queue menerapkan **Multi-Folder Concurrency Terkendali**:

1. **Konkurensi Multi-Folder Terbatas (Maksimal 2 Folder Bersamaan)**:
   - Worker pool membatasi pemindaian hingga 2 folder berbeda secara paralel menggunakan `tokio::sync::Semaphore` (`MAX_CONCURRENT_FOLDER_SCANS = 2`).
   - Setiap command `WorkerCommand::IndexFolder` dieksekusi dalam Tokio task terpisah yang mengakuisisi permit konkurensi.
2. **Eksklusivitas Kunci per Folder**:
   - Aggregate `JobTracker` secara mutlak mengunci folder (`try_lock_folder(&folder_id)`). Folder yang sama **DILARANG** dipindai lebih dari satu kali secara bersamaan (mencegah *race condition* dan data korup).
   - Folder ke-3 (atau folder yang terkunci) tetap mengantre di channel MPSC / berstatus `PENDING` di database sampai slot pemindaian tersedia.
3. **Throttling I/O Global**:
   - Seluruh folder task yang aktif berbagi `Arc<tokio::sync::Semaphore>` (`file_io_semaphore`, default 50 permits) untuk pembacaan berkas & hashing disk, mencegah lonjakan *disk thrashing* atau OOM.

---

## 4. Desain Database & Skema Penyimpanan

### 4.1 PostgreSQL 18.6 Schema (Metadata & Registry)

Semua migrasi dikelola menggunakan `sqlx::migrate!`. Penulisan query mematuhi aturan ketat: **tidak ada `SELECT *`**, selalu parameterisasi input `$1, $2`, dan gunakan batch operations.

```sql
-- Migration 0001: Folders
CREATE TABLE folders (
    id UUID PRIMARY KEY,
    root_path TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_scanned_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'IDLE' -- IDLE, SCANNING, ERROR
);
CREATE INDEX idx_folders_root_path ON folders(root_path);

-- Migration 0002: Document Registry
CREATE TABLE document_registry (
    id UUID PRIMARY KEY,
    folder_id UUID NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    content_hash VARCHAR(64), -- Nullable untuk file SKIPPED/FAILED yang tidak dibaca/dihash
    file_size_bytes BIGINT NOT NULL,
    modified_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'INDEXED', -- INDEXED, SKIPPED, FAILED, EXCLUDED
    status_reason TEXT,
    last_indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_folder_relative_path UNIQUE (folder_id, relative_path)
);
CREATE INDEX idx_doc_registry_folder_id ON document_registry(folder_id);
CREATE INDEX idx_doc_registry_status ON document_registry(status);

-- Migration 0003: Indexing Jobs
CREATE TABLE indexing_jobs (
    id UUID PRIMARY KEY,
    folder_id UUID REFERENCES folders(id) ON DELETE CASCADE, -- Nullable untuk global job seperti REBUILD
    job_type VARCHAR(32) NOT NULL, -- IMPORT, RESCAN, REBUILD
    status VARCHAR(32) NOT NULL,   -- PENDING, RUNNING, COMPLETED, FAILED, CANCELLED
    files_total INT NOT NULL DEFAULT 0, -- Total file teridentifikasi untuk progress bar
    files_added INT NOT NULL DEFAULT 0,
    files_updated INT NOT NULL DEFAULT 0,
    files_deleted INT NOT NULL DEFAULT 0,
    files_skipped INT NOT NULL DEFAULT 0,
    files_failed INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_summary TEXT
);
CREATE INDEX idx_jobs_folder_id ON indexing_jobs(folder_id);
CREATE INDEX idx_jobs_status ON indexing_jobs(status);

-- Migration 0004: Application Settings
CREATE TABLE settings (
    key VARCHAR(64) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### 1. Analisis Normalisasi Database (3NF) & Mekanisme Tombstone `EXCLUDED`
Skema PostgreSQL mengadopsi **Bentuk Normal Ketiga (3NF)**:
- **Pemisahan Entitas**: Tabel `folders`, `document_registry`, `indexing_jobs`, dan `settings` berdiri sendiri dengan primary key UUID yang unik.
- **Bebas Ketergantungan Transitif**: Seluruh atribut non-kunci di `document_registry` (`content_hash`, `file_size_bytes`, `modified_at`) bergantung fungsional penuh pada primary key `id` dan natural key `(folder_id, relative_path)`.
- **Zero Content Redundancy**: Isi dokumen teks **tidak disimpan di PostgreSQL**, melainkan tetap berada di sistem file lokal (disk) dan di-index ke Elasticsearch. Hal ini mencegah write amplification, menghemat konsumsi storage relasional, dan menjaga ukuran tabel database tetap ramping.
- **Tombstone `status = 'EXCLUDED'` (Anti-Ghost Resurrection)**:
  Saat pengguna menghapus satu dokumen spesifik dari index via `DELETE /api/documents/:id`, dokumen dihapus dari Elasticsearch dan barisnya di `document_registry` diperbarui ke `status = 'EXCLUDED'`. Ketika pengguna memicu Re-scan folder di kemudian hari, `ScanPlanner` mendeteksi bahwa file ini berstatus `EXCLUDED` dan langsung memasukkannya ke kategori `to_skip (reason: "UserExcluded")`, sehingga dokumen yang sengaja disingkirkan tidak akan bangkit kembali (*resurrect*) ke hasil pencarian.

#### 2. Analisis Database Indexes & Trade-offs
Indeks B-Tree dirancang strategis untuk mempercepat query scan planning dan status tracking:
- `idx_folders_root_path`: Mempercepat validasi duplikasi saat pendaftaran folder baru O(log N).
- `uq_folder_relative_path` (Composite Unique Index): Fondasi penentu identitas dokumen untuk operasi `upsert` registry dan diffing file O(log N).
- `idx_doc_registry_folder_id` & `idx_doc_registry_status`: Mempercepat query pemfilteran daftar file per folder saat Scan Planner menghitung diff.
- `idx_jobs_folder_id` & `idx_jobs_status`: Mempercepat polling status job aktif oleh frontend Tauri.
- **Trade-off Indeks**: Setiap operasi penulisan (`INSERT` / `UPDATE` pada registry) memiliki sedikit overhead karena B-Tree index harus diperbarui. Namun, trade-off ini sangat sepadan karena frekuensi pembacaan dan pembandingan ribuan file saat Re-scan menjadi instan tanpa *full table scan*.

#### 3. Batas Transaksi & Penjaminan ACID
Operasi mutasi data multi-langkah dijamin oleh properti **ACID** via `sqlx::Transaction`:
- **Atomic Batch Commit**: Saat proses indexing berlangsung, setiap batch chunk (100 dokumen) di-commit ke tabel `document_registry` bersamaan dengan pembaruan counter di `indexing_jobs` dalam satu transaksi database:
  ```rust
  let mut tx = pool.begin().await?;
  // 1. Bulk upsert registry entries
  // 2. Update job progress counters
  tx.commit().await?;
  ```
  Jika proses backend crash di tengah jalan, transaksi batch yang belum selesai otomatis di-rollback (Atomicity & Consistency), sehingga tidak meninggalkan data registry yatim piatu.
- **Cascading Folder Deletion**: Penghapusan folder dari sistem mengeksekusi penghapusan atomic `folders` yang secara cascading menghapus seluruh `document_registry` dan `indexing_jobs` terkait.

#### 4. Pengaturan Connection Pool SQLx (`crates/backend/src/infrastructure/postgres/connection.rs`)
- `max_connections`: 20
- `min_connections`: 5
- `acquire_timeout`: 3 detik
- `idle_timeout`: 10 menit
- `max_lifetime`: 30 menit

---

### 4.2 Elasticsearch 8.19.22 Mapping & Code Analyzer

Elasticsearch 8.19.22 menggunakan arsitektur index alias `lynx_documents` yang mengarah ke index fisik terversi (misal: `lynx_documents_v1`).

#### Custom Code Analyzer (`code_identifier_analyzer`)
Memecah identifier camelCase dan snake_case menjadi sub-kata, sambil mempertahankan token asli (`preserve_original: true`) agar pencarian nama fungsi persis tetap mendapat prioritas relevansi tertinggi.

```json
{
  "settings": {
    "number_of_shards": 1,
    "number_of_replicas": 0,
    "analysis": {
      "filter": {
        "code_subword_filter": {
          "type": "word_delimiter_graph",
          "generate_word_parts": true,
          "generate_number_parts": true,
          "catenate_words": false,
          "split_on_case_change": true,
          "split_on_numerics": true,
          "preserve_original": true
        },
        "autocomplete_filter": {
          "type": "edge_ngram",
          "min_gram": 2,
          "max_gram": 20
        }
      },
      "analyzer": {
        "code_analyzer": {
          "type": "custom",
          "tokenizer": "whitespace",
          "filter": [
            "code_subword_filter",
            "lowercase",
            "flatten_graph"
          ]
        },
        "autocomplete_analyzer": {
          "type": "custom",
          "tokenizer": "standard",
          "filter": [
            "lowercase",
            "autocomplete_filter"
          ]
        }
      }
    }
  },
  "mappings": {
    "properties": {
      "id": { "type": "keyword" },
      "folder_id": { "type": "keyword" },
      "relative_path": { "type": "keyword" },
      "absolute_path": { "type": "keyword", "index": false },
      "title": {
        "type": "text",
        "analyzer": "standard",
        "fields": {
          "code": { "type": "text", "analyzer": "code_analyzer" },
          "suggest": { "type": "text", "analyzer": "autocomplete_analyzer", "search_analyzer": "standard" }
        }
      },
      "content": {
        "type": "text",
        "analyzer": "standard",
        "fields": {
          "code": { "type": "text", "analyzer": "code_analyzer" }
        }
      },
      "tags": { "type": "keyword" },
      "extension": { "type": "keyword" },
      "language": { "type": "keyword" },
      "type": { "type": "keyword" },
      "project": { "type": "keyword" },
      "file_size_bytes": { "type": "long" },
      "modified_at": { "type": "date" },
      "indexed_at": { "type": "date" }
    }
  }
}
```

---

## 5. Pipeline Search & Algoritma Information Retrieval

### 5.1 Query Parsing & AST
Pengguna dapat mengetik:
`"tokio runtime" language:rust tag:concurrency project:backend type:code ownership`

`QueryParser` memproduksi AST:
```rust
pub(crate) struct SearchQuery {
    pub free_terms: Vec<String>,
    pub phrase_terms: Vec<String>,
    pub filters: HashMap<FilterKey, String>, // language, tag, project, extension, type
    pub warnings: Vec<String>,             // Peringatan jika ada filter format salah
}
```

### 5.2 Search Query DSL & Scoring Logic
Query dikirimkan ke Elasticsearch dengan konfigurasi:
1. **Multi-match dengan Field Boost**:
   - `title^3.0` (Memberi prioritas lebih tinggi terhadap kecocokan di title dibanding field dengan boost lebih rendah)
   - `tags^2.0` (Tag kategori eksplisit)
   - `content^1.0` (Isi teks dasar)
   > [!NOTE]
   > Boost adalah pengali kontribusi scoring, dan skor keseluruhan tetap bergantung pada query dan dokumen (bukan jaminan mutlak title selalu mengalahkan content dalam segala skenario).
2. **Code Search Booster**:
   - Pencarian juga mengecek subfield `title.code` dan `content.code` dengan bobot yang seimbang, memungkinkan kecocokan parsial camelCase/snake_case.
3. **Fuzzy & Prefix Tolerance**:
   - Istilah bebas menerapkan `fuzziness: "AUTO"` pada klausa `should` dengan boost lebih rendah (`0.5`) agar hasil cocok persis selalu mengungguli hasil typo.
4. **Exact Filtering via `bool.filter`**:
   - Nilai filter (`language:rust`, `tag:concurrency`, `project:backend`, `type:code`, `extension:rs`) ditaruh di dalam blok `bool.filter`.
   - **Rasional**: `bool.filter` digunakan untuk exact filtering karena filter tidak memengaruhi `_score` dan Elasticsearch dapat mengoptimalkan/caching klausa filter.
5. **Dynamic Facet Aggregation**:
   - Menggunakan `terms` aggregations pada keyword fields (`extension`, `language`, `type`, `project`, `tags`) yang dievaluasi berdasarkan query aktif saat ini.

### 5.3 Algoritma Perhitungan Nomor Baris Highlight
Elasticsearch mengembalikan cuplikan `highlight` yang dibungkus tag `<em>...</em>`.
Backend menghitung **nomor baris akurat** untuk file code:
1. Elasticsearch mengembalikan offset fragmen atau string fragmen kecocokan.
2. Backend mencocokkan kemunculan substring fragmen pada file asli di memori/disk.
3. Hitung jumlah karakter baris baru `\n` sebelum posisi indeks karakter substring tersebut:
   $$\text{Line Number} = 1 + \text{count\_char}(text[0..\text{offset}], '\backslash n')$$
4. DTO hasil pencarian ke frontend sudah menyertakan `line_number` siap render di UI:
```json
{
  "snippet": "...pub async fn <em>start_server</em>()...",
  "line_number": 42
}
```

### 5.4 Strategi Zero-Downtime Reindexing (Blue-Green Index Alias Swap)

Ketika skema analyzer Elasticsearch diperbarui atau pengguna memicu Rebuild Index (`POST /api/index/rebuild`), pencarian dokumen tidak boleh mengalami *downtime* atau kegagalan query. LynxSearch menerapkan strategi **Blue-Green Deployment** berbasis Elasticsearch Index Alias:

```mermaid
sequenceDiagram
    participant Client as UI / Search Client
    participant Backend as Backend (IndexOrchestrator)
    participant ES as Elasticsearch Cluster

    Note over ES: Alias 'lynx_documents' -> lynx_documents_v1 (Active)
    Client->>Backend: POST /api/index/rebuild
    Backend->>ES: 1. Create Index 'lynx_documents_v2' (New Settings & Mappings)
    Backend->>ES: 2. Stream Bulk Index from PostgreSQL Registry to 'lynx_documents_v2'
    Note over Client,ES: Pencarian tetap berjalan normal membaca 'lynx_documents' (v1)
    Backend->>ES: 3. POST /_aliases (Atomic Swap: remove v1, add v2)
    Note over ES: Alias 'lynx_documents' -> lynx_documents_v2 (Active)
    Backend->>ES: 4. DELETE /lynx_documents_v1
    Backend-->>Client: Rebuild Completed (100% Zero Downtime)
```

1. **Pembuatan Index Fisik Baru (`lynx_documents_v2`)**:
   - Backend membaca versi index fisik aktif dari Elasticsearch (misal: `v1`).
   - Membuat index baru `lynx_documents_v2` lengkap dengan custom code analyzer dan mapping terbaru.
2. **Bulk Indexing Latar Belakang**:
   - Worker membaca seluruh entri dokumen dari PostgreSQL `document_registry` dan mengindeks dokumen ke `lynx_documents_v2` dalam chunk batch 200 dokumen.
   - Selama proses reindex berlangsung, seluruh query pencarian pengguna tetap diarahkan oleh Elasticsearch ke alias `lynx_documents` (yang masih mengarah ke `v1`). Tidak ada hasil hilang atau downtime.
3. **Atomic Alias Swap (`POST /_aliases`)**:
   - Setelah 100% dokumen berhasil diindeks ke `v2`, backend mengeksekusi operasi atomik alias swap dalam satu HTTP request:
   ```json
   POST /_aliases
   {
     "actions": [
       { "remove": { "index": "lynx_documents_v1", "alias": "lynx_documents" } },
       { "add":    { "index": "lynx_documents_v2", "alias": "lynx_documents" } }
     ]
   }
   ```
   - Operasi ini bersifat atomik di tingkat cluster Elasticsearch: tidak ada celah mikrodetik di mana alias tidak mengarah ke index manapun.
4. **Housekeeping (Pembersihan Index Lama)**:
   - Backend memverifikasi bahwa alias swap berhasil, lalu menghapus index lama `DELETE /lynx_documents_v1` untuk membebaskan ruang disk.

---

## 6. Kontrak HTTP REST API (Axum 0.8)

Seluruh response API menggunakan format seragam yang aman dan terstruktur.

### 6.1 Ringkasan Endpoint

| Method | Endpoint | Fungsi | Status Sukses |
|---|---|---|---|
| `GET` | `/api/health` | Health status summary (backend, DB, ES) | `200 OK` |
| `GET` | `/api/health/live` | Liveness probe (proses backend aktif) | `200 OK` |
| `GET` | `/api/health/ready` | Readiness probe (koneksi DB & ES siap layani traffic) | `200 OK` / `503` |
| `GET` | `/api/stats` | Statistik index (total dokumen, total ukuran, kategori) | `200 OK` |
| `GET` | `/api/folders` | Daftar folder terdaftar & status scan | `200 OK` |
| `POST` | `/api/index/folder` | Daftarkan folder baru / picu Re-scan | `202 Accepted` |
| `POST` | `/api/index` | Index atau un-exclude 1 file dokumen tunggal (reset status EXCLUDED ke INDEXED) | `200 OK` |
| `DELETE` | `/api/folders/:id` | Hapus folder beserta dokumennya dari index | `200 OK` |
| `GET` | `/api/index/jobs/:id` | Polling progres & status background job | `200 OK` |
| `POST` | `/api/index/jobs/:id/cancel` | Batalkan background job yang sedang berjalan | `200 OK` |
| `POST` | `/api/index/rebuild` | Trigger pembangunan ulang index Elasticsearch via alias (global lock: tolak jika ada job aktif, return 409 pada scan baru) | `202 Accepted` |
| `GET` | `/api/search` | Search: query, filter, sort, pagination, facets | `200 OK` |
| `GET` | `/api/suggest` | Autocomplete judul & terms (debounced) | `200 OK` |
| `GET` | `/api/documents/:id` | Konten penuh dokumen untuk preview panel | `200 OK` |
| `DELETE` | `/api/documents/:id` | Hapus 1 dokumen dari index (set status EXCLUDED di DB) | `200 OK` |
| `GET` | `/api/settings` | Ambil pengaturan (max file size, weights, ignore list) | `200 OK` |
| `PUT` | `/api/settings` | Update pengaturan sistem | `200 OK` |

### 6.2 Spesifikasi DTO Request & Response Konkret (17 Endpoint)

#### 1. System Health & Observability
- **`GET /api/health`**
  - **Response (200 OK)**:
    ```json
    {
      "status": "ok",
      "version": "1.0.0",
      "timestamp": "2026-10-02T12:00:00Z",
      "database": { "status": "up", "latency_ms": 3 },
      "elasticsearch": { "status": "up", "latency_ms": 5 }
    }
    ```
- **`GET /api/health/live`**
  - **Response (200 OK)**: `{ "status": "alive" }`
- **`GET /api/health/ready`**
  - **Response (200 OK / 503 Service Unavailable)**: `{ "status": "ready" }`

#### 2. System Statistics
- **`GET /api/stats`**
  - **Response (200 OK)**:
    ```json
    {
      "total_documents": 1420,
      "total_size_bytes": 4829104,
      "types": { "code": 980, "doc": 320, "config": 120 },
      "languages": { "rust": 650, "typescript": 330, "markdown": 320, "toml": 120 },
      "indexed_folders": 3
    }
    ```

#### 3. Folder Management
- **`GET /api/folders`**
  - **Response (200 OK)**:
    ```json
    [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "root_path": "/home/developer/projects/backend",
        "status": "READY",
        "document_count": 450,
        "last_scanned_at": "2026-10-02T10:00:00Z",
        "created_at": "2026-10-01T08:00:00Z"
      }
    ]
    ```
- **`POST /api/index/folder`**
  - **Request Body**: `{ "root_path": "/home/developer/projects/backend" }`
  - **Response (202 Accepted)**:
    ```json
    {
      "job_id": "8a0e8400-e29b-41d4-a716-446655440001",
      "folder_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "RUNNING",
      "message": "Background scanning job created successfully."
    }
    ```
- **`DELETE /api/folders/:id`**
  - **Response (200 OK)**:
    ```json
    {
      "success": true,
      "folder_id": "550e8400-e29b-41d4-a716-446655440000",
      "deleted_documents": 450
    }
    ```

#### 4. Indexing & Job Operations
- **`POST /api/index`** (Index / un-exclude file tunggal)
  - **Request Body**:
    ```json
    {
      "folder_id": "550e8400-e29b-41d4-a716-446655440000",
      "relative_path": "src/main.rs"
    }
    ```
  - **Response (200 OK)**:
    ```json
    {
      "document_id": "990e8400-e29b-41d4-a716-446655440002",
      "status": "INDEXED",
      "message": "Document indexed successfully."
    }
    ```
- **`GET /api/index/jobs/:id`**
  - **Response (200 OK)**:
    ```json
    {
      "job_id": "8a0e8400-e29b-41d4-a716-446655440001",
      "folder_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "RUNNING",
      "processed_files": 120,
      "skipped_files": 10,
      "failed_files": 0,
      "total_files": 450,
      "error": null,
      "started_at": "2026-10-02T10:00:00Z",
      "finished_at": null
    }
    ```
- **`POST /api/index/jobs/:id/cancel`**
  - **Response (200 OK)**:
    ```json
    {
      "job_id": "8a0e8400-e29b-41d4-a716-446655440001",
      "status": "CANCELLED",
      "message": "Job cancellation signal sent."
    }
    ```
- **`POST /api/index/rebuild`**
  - **Response (202 Accepted)**:
    ```json
    {
      "job_id": "3b0e8400-e29b-41d4-a716-446655440003",
      "target_index": "lynx_documents_v2",
      "status": "RUNNING",
      "message": "Zero-downtime index rebuild initiated."
    }
    ```

#### 5. Search & Suggestion Engine
- **`GET /api/search`**
  - **Query Parameters**:
    - `q` (optional): Query teks atau token inline (`language:rust`, `tag:cli`, dll.)
    - `page` (optional, default `1`): Nomor halaman (min 1, max 1000)
    - `size` (optional, default `20`): Ukuran halaman (min 1, max 100)
    - `type` (optional): Filter tipe dokumen (`doc`, `code`, `config`)
    - `language` (optional): Filter bahasa (`rust`, `typescript`, dll.)
    - `tag` (optional): Filter tag
    - `project` (optional): Filter sub-proyek
    - `sort` (optional, default `relevance`): `relevance` | `modified_desc` | `modified_asc` | `size_desc` | `size_asc`
  - **Response (200 OK)**:
    ```json
    {
      "query": "authenticate",
      "page": 1,
      "size": 20,
      "total": 1,
      "took_ms": 14,
      "items": [
        {
          "id": "990e8400-e29b-41d4-a716-446655440002",
          "title": "Auth Middleware",
          "relative_path": "src/middleware/auth.rs",
          "project": "backend",
          "type": "code",
          "language": "rust",
          "tags": ["auth", "security"],
          "highlights": [
            {
              "snippet": "pub fn <em>authenticate_user</em>(token: &str) -> bool",
              "line_number": 42
            }
          ],
          "score": 4.82,
          "file_size": 2048,
          "updated_at": "2026-10-01T15:30:00Z"
        }
      ],
      "facets": {
        "types": [{ "key": "code", "doc_count": 1 }],
        "languages": [{ "key": "rust", "doc_count": 1 }],
        "tags": [{ "key": "auth", "doc_count": 1 }, { "key": "security", "doc_count": 1 }],
        "projects": [{ "key": "backend", "doc_count": 1 }]
      },
      "warnings": []
    }
    ```
- **`GET /api/suggest`**
  - **Query Parameters**: `q` (string wajib), `limit` (optional, default `5`)
  - **Response (200 OK)**:
    ```json
    {
      "suggestions": ["authenticate", "authentication", "authorize"]
    }
    ```

#### 6. Document Viewer & Exclusion
- **`GET /api/documents/:id`**
  - **Response (200 OK)**:
    ```json
    {
      "id": "990e8400-e29b-41d4-a716-446655440002",
      "folder_id": "550e8400-e29b-41d4-a716-446655440000",
      "folder_root_path": "/home/developer/projects/backend",
      "relative_path": "src/middleware/auth.rs",
      "title": "Auth Middleware",
      "content": "// Full document text content loaded safely from disk\npub fn authenticate_user() { ... }\n",
      "type": "code",
      "language": "rust",
      "tags": ["auth", "security"],
      "project": "backend",
      "file_size": 2048,
      "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "updated_at": "2026-10-01T15:30:00Z"
    }
    ```
- **`DELETE /api/documents/:id`**
  - **Response (200 OK)**:
    ```json
    {
      "success": true,
      "id": "990e8400-e29b-41d4-a716-446655440002",
      "status": "EXCLUDED",
      "message": "Document removed from search index and marked as EXCLUDED."
    }
    ```

#### 7. System Settings
- **`GET /api/settings`**
  - **Response (200 OK)**:
    ```json
    {
      "max_file_size_bytes": 2097152,
      "weights": {
        "title": 3.0,
        "tags": 2.0,
        "content": 1.0
      },
      "ignore_patterns": [".git", "node_modules", "target", "dist", "build"]
    }
    ```
- **`PUT /api/settings`**
  - **Request Body**:
    ```json
    {
      "max_file_size_bytes": 4194304,
      "weights": {
        "title": 4.0,
        "tags": 2.5,
        "content": 1.0
      },
      "ignore_patterns": [".git", "node_modules", "target", "dist", "build", ".cache"]
    }
    ```
  - **Response (200 OK)**: (Mengembalikan object `Settings` teraktual)

> **Mekanisme Fallback & Reset-to-Default**:
> - Persistence settings disimpan dalam tabel PostgreSQL `settings` (`key = 'app_settings'`, `value JSONB NOT NULL`).
> - Jika record belum ada di database, sistem menginisialisasi secara otomatis dengan nilai default konstan (`max_file_size_bytes = 2097152`, bobot `3.0`/`2.0`/`1.0`, ignore `[".git", "node_modules", "target", "dist", "build"]`).
> - Mengirim payload kosong `{}` atau field bernilai `null` pada `PUT /api/settings` mereset setting terkait kembali ke nilai default konstan.

#### 8. Standard Error Response
Seluruh error 4xx dan 5xx dikembalikan dalam skema flat seragam:
```json
{
  "code": "RESOURCE_NOT_FOUND",
  "message": "Dokumen dengan ID tersebut tidak ditemukan pada database.",
  "details": null
}
```
HTTP status code utama: `200`, `202`, `400`, `403`, `404`, `409`, `422`, `500`, `503`.


---

## 7. Desain Frontend Desktop: Tauri 2.12 + React 19.3 + Vite 8.1 + shadcn/ui

Frontend dirancang mengikuti metodologi **Frontend Engineering**, pedoman **React + Vite Best Practices**, panduan **Tauri Development**, serta standar integrasi **shadcn/ui**.

### 7.1 Prinsip Desain & Arsitektur Frontend
1. **Local-First Native Feel**: Berjalan di dalam Webview Tauri 2.12 dengan integrasi OS native (dialog file picker, shell execution untuk membuka default editor, dan copy-paste clipboard).
2. **Strict State Separation**:
   - **Server State (TanStack Query v5)**: Caching hasil query, background job polling, periodic health status check, dan query cancellation.
   - **Client UI State (Zustand 5)**: State filter aktif, query input teks, selected document ID, layout sidebar/preview collapse, dan visual theme.
   - **Component Local State (React hooks)**: State lokal yang tidak perlu dibagikan (hover, open popover, transient form inputs).
3. **4 Micro-States Contract**: Setiap komponen yang bergantung pada data wajib mengimplementasikan 4 state (Loading, Empty, Error, Success). *Spesifikasi visual, animasi pulse, dan perilaku pemulihan didokumentasikan di [DESIGN.md Section 5](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md#5-kontrak-4-micro-states-frontend-engineering-standard).*
4. **Keyboard-First Workflow**: Seluruh navigasi kritis dapat diakses tanpa mouse (`Cmd/Ctrl + K`, `Escape`, `j`/`k`, `Enter`, `Cmd+O`, `Cmd+Shift+C`, `[`/`]`). *Peta pintasan keyboard lengkap dan aturan fokus WCAG 2.2 AA didokumentasikan di [DESIGN.md Section 8](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md#8-navigasi-keyboard--aksesibilitas-wcag-22-aa).*

---

### 7.2 Integrasi Tauri 2.12, Security & Native Plugins

#### 1. Tauri 2.12 Configuration (`apps/desktop/src-tauri/tauri.conf.json`)
```json
{
  "productName": "LynxSearch",
  "version": "1.0.0",
  "identifier": "com.boyblanco.lynxsearch",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "LynxSearch",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 640,
        "decorations": true,
        "center": true,
        "transparent": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' http://127.0.0.1:*; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:;"
    }
  },
  "plugins": {
    "dialog": {},
    "shell": {},
    "opener": {},
    "window-state": {},
    "single-instance": {}
  }
}
```

#### 2. Tauri 2.12 Capabilities & Permissions (`apps/desktop/src-tauri/capabilities/default.json`)
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions for LynxSearch desktop frontend",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "dialog:allow-open",
    "shell:default",
    "shell:allow-open",
    "opener:default",
    {
      "identifier": "opener:allow-open-path",
      "allow": [{ "path": "**" }]
    },
    "opener:allow-reveal-item-in-dir",
    "window-state:default",
    "clipboard-manager:default",
    "clipboard-manager:allow-write-text",
    "clipboard-manager:allow-read-text"
  ]
}
```

#### 3. Native Plugins & Desktop Bridge Integration
- **`@tauri-apps/plugin-dialog` & Native `pick_folder` Command**: Membuka native directory picker secara non-blocking (async via oneshot channel callback) dengan window parenting langsung ke `main` window (`builder.set_parent(&window)`) sehingga modal file picker selalu muncul di atas aplikasi utama tanpa tertutup atau memicu deadlock event loop GTK / Mutter; ukuran dialog GTK di-clamp otomatis ke `(900, 560)` dan saat dibatalkan mengembalikan `null` seketika tanpa dialog kedua.
- **Window Focus on Click & Stacking**: Integrasi native command `focus_window` (`unminimize`, `show`, `set_focus`), penegakan `set_always_on_top(false)`, resolusi konflik GNOME Forge tiling (`float-always-on-top-enabled=false`), serta capture listener `mousedown` memastikan klik pada jendela aplikasi langsung menaikkannya ke foreground, sementara jendela aplikasi lain (IDE, terminal) tetap dapat menutupinya saat diklik (perilaku standar desktop).
- **`@tauri-apps/plugin-shell`**: Eksekusi perintah command line atau spawn proses editor spesifik.
- **`@tauri-apps/plugin-opener` & `open_file_in_editor` Command**: Membuka file atau reveal folder di sistem file manager / default viewer native OS (US #6 & #7) melalui direct Tauri invoke command dengan fallback plugin.
- **`@tauri-apps/plugin-window-state`**: Menyimpan ukuran dan posisi jendela saat ditutup dan memulihkannya saat aplikasi dibuka kembali.
- **`@tauri-apps/plugin-single-instance`**: Memastikan hanya ada satu proses jendela desktop yang berjalan; otomatis memfokuskan jendela aktif jika aplikasi dijalankan ulang dengan enforcement `set_always_on_top(false)`.
- **Adaptive Backend Reconnect**: `useHealthQuery` beralih ke interval adaptif agresif 2.000ms ketika backend offline, dengan revalidasi fokus jendela dan dukungan click-to-retry instan pada badge status TopBar.
- **Webview Trackpad Gesture Safety & Keyboard Zoom**: Blokir total touchpad pinch gesture pada level widget GTK via `webview.connect_event` (intercept `gdk::EventType::TouchpadPinch` -> `glib::Propagation::Stop`) serta `connect_zoom_level_notify` guard dan JS capture event listeners guna mencegah glitch layout, didukung pengatur zoom keyboard desktop native via command `set_desktop_zoom` (`Ctrl + +`, `Ctrl + -`, `Ctrl + 0`).

---

### 7.3 Integrasi Design System UI & Plugin Desktop

LynxSearch mengadopsi tema **OpenAI Dark Minimalist** dengan aksen ChatGPT Teal (`#10a37f`) yang diintegrasikan ke dalam shadcn/ui dan Tailwind CSS 4.3 (@theme tokens).

> **Pemisahan Tanggung Jawab Dokumen UI:**  
> Seluruh rincian visual UI, konfigurasi CSS variables (`index.css`), registri `components.json`, helper `cn()`, implementasi Toaster (`sonner`), format tanggal native `Intl`, serta tata letak 3-Pane dan modal dialog dipusatkan secara eksklusif di **[DESIGN.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md)**.

Arsitektur integrasi desktop layer tetap mempertahankan:
- **`@tauri-apps/api`**: Akses core API Tauri untuk manipulasi ukuran window, custom titlebar draggable area (`data-tauri-drag-region`), dan lifecycle runtime.
- **`@tauri-apps/plugin-clipboard-manager`**: Menyalin isi dokumen yang sedang dipratinjau langsung ke clipboard sistem operasi (`writeText(doc.content)`) dengan shortcut keyboard `Cmd/Ctrl + Shift + C`, serta mendukung penyalinan path file dengan mengklik teks path di header preview, tanpa memicu browser clipboard permission warning.

---

### 7.4 Optimasi Build & Kinerja (React 19.3 + Vite 8.1)

#### 1. `apps/desktop/vite.config.ts`
```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { visualizer } from 'rollup-plugin-visualizer';

export default defineConfig({
  plugins: [
    react(),
    visualizer({ filename: 'stats.html', open: false, gzipSize: true, brotliSize: true }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    target: 'baseline-widely-available',
    sourcemap: false,
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (/[\\/]node_modules[\\/](react|react-dom|scheduler)[\\/]/.test(id)) {
              return 'vendor-react';
            }
            if (/[\\/]node_modules[\\/]@tanstack[\\/]/.test(id)) {
              return 'vendor-tanstack';
            }
            if (
              /[\\/]node_modules[\\/](@radix-ui|lucide-react|sonner)[\\/]/.test(id)
            ) {
              return 'vendor-ui';
            }
            if (
              /[\\/]node_modules[\\/](react-markdown|shiki|remark-gfm|micromark)[\\/]/.test(id)
            ) {
              return 'vendor-markdown';
            }
          }
        },
      },
    },
  },
  optimizeDeps: {
    include: ['react', 'react-dom', '@tanstack/react-query', '@tanstack/react-virtual', 'zustand', 'zod'],
  },
  server: {
    port: 5173,
    strictPort: true,
    hmr: { overlay: true },
  },
});
```

#### 2. Code Splitting & Dynamic Imports
```typescript
import { lazy } from 'react';

export const FolderManagerDialog = lazy(() => import('@/components/folders/FolderManagerModal'));
export const SettingsDialog = lazy(() => import('@/components/settings/SettingsModal'));
export const DocumentPreviewPanel = lazy(() => import('@/components/preview/DocumentPreview'));
```

---

### 7.5 Manajemen State & Alur Data (Zustand 5 + TanStack Query v5)

#### 1. Component State Ownership Matrix (Frontend Engineering Standard)
| State Name | Ownership | Storage | Rationale |
|---|---|---|---|
| `rawQuery` | `useSearchStore` | Zustand | Sinkron dengan input dan shortcut global |
| `activeFilters` | `useSearchStore` | Zustand | Sinkron dengan klik facet dan token query |
| `selectedDocId` | `useSearchStore` | Zustand | Dokumen yang sedang aktif di panel preview |
| `searchResults` | `useSearchQuery` | TanStack Query | Server cache (staleTime 30s), pagination cache |
| `suggestions` | `useSuggestQuery` | TanStack Query | Autocomplete dropdown (debounce 150ms) |
| `backendHealth` | `useHealthQuery` | TanStack Query | Status polling setiap 10s |
| `jobProgress` | `useJobProgressQuery`| TanStack Query | Polling status 1s saat job aktif |
| `theme` | `useUIStore` | Zustand + LocalStorage | Tema light/dark persist across reloads |
| `sidebarCollapsed` | `useUIStore` | Zustand | UI toggle panel facet |

---

### 7.6 Strategi Rendering Virtualisasi Dokumen & Kode (@tanstack/react-virtual + Shiki)

Untuk menangani file source code dan catatan dengan ribuan baris tanpa penurunan frame rate (menjaga 60 FPS dan CLS = 0), aplikasi menerapkan arsitektur virtualisasi baris:
1. **Row Virtualization Engine**: Menggunakan `@tanstack/react-virtual` dengan estimasi tinggi baris tetap 22px dan overscan 20 baris di luar viewport. Hanya elemen DOM yang terlihat yang dirender di layar.
2. **Auto-Scroll ke Baris Target (US #40, #61)**: Viewport otomatis meluncur mulus ke baris kode yang cocok (`rowVirtualizer.scrollToIndex(highlightLine - 1, { align: 'center', behavior: 'smooth' })`).
3. **Keyword Highlighting & AST Transformer**: Token pencarian dibungkus tag `<mark>` pada baris kode virtualized dan via custom AST text-node transformer pada Markdown viewer (`react-markdown`).

> **Implementasi Lengkap Komponen UI:**  
> Kode sumber lengkap komponen `VirtualizedCodeViewer`, fungsi penanda `highlightCodeTokens`, dan integrasi AST viewer dipindahkan dan didokumentasikan di **[DESIGN.md Section 6.1](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md#61-virtualized-code-viewer-appsdesktopcomponentspreviewvirtualizedcodeviewertsx)**.

---

## 8. Aspek Lintas Batas (Cross-Cutting Concerns) & Keamanan

### 8.1 Structured Error Handling & Katalog Kode Error
Sistem menerapkan penanganan error terstruktur yang ketat: tidak ada string error generik mentah yang bocor ke klien. Setiap error memetakan kode domain unik (`code`), pesan ramah pengguna (`message`), dan rincian opsional (`details`):

```rust
#[derive(thiserror::Error, Debug)]
pub(crate) enum AppError {
    #[error("Folder not found: {0}")]
    FolderNotFound(Uuid),
    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),
    #[error("Conflict: Indexing job is currently active for folder {0}")]
    JobConflict(Uuid),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
    #[error("Invalid search query: {0}")]
    InvalidQuery(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Search engine error: {0}")]
    SearchEngine(String),
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

#### Katalog Pemetaan Error ke HTTP Status Code
| Domain Error Code | HTTP Status | Keterangan & Penanganan Klien |
|---|---|---|
| `FOLDER_NOT_FOUND` | `404 Not Found` | Folder ID tidak terdaftar di database |
| `DOCUMENT_NOT_FOUND` | `404 Not Found` | File tidak ditemukan di registry atau disk |
| `JOB_CONFLICT` | `409 Conflict` | Folder sedang di-scan; tolak trigger job baru |
| `VALIDATION_FAILED` | `422 Unprocessable` | Input request tidak lolos validasi DTO |
| `PATH_TRAVERSAL_DETECTED`| `403 Forbidden` | Upaya akses file di luar root folder yang sah |
| `INVALID_QUERY` | `400 Bad Request` | Sintaks query rusak atau tidak dapat di-parse |
| `DATABASE_UNAVAILABLE` | `503 Service Unavailable`| Gagal koneksi ke PostgreSQL 18.6 |
| `SEARCH_ENGINE_UNAVAILABLE`| `503 Service Unavailable`| Gagal koneksi ke Elasticsearch 8.19.22 |
| `INTERNAL_SERVER_ERROR` | `500 Internal Error` | Bug internal tak terduga (rincian hanya dicatat di log) |

Format respons error seragam ke desktop UI:
```json
{
  "code": "VALIDATION_FAILED",
  "message": "Parameter input tidak valid.",
  "details": {
    "size": ["Ukuran halaman maksimal adalah 100"]
  }
}
```

---

### 8.2 Input Validation & Path Traversal Sanitization
Menggunakan `validator` crate dengan custom Axum extractor (`ValidatedJson` dan `ValidatedQuery`) untuk menjamin semua request divalidasi sebelum menyentuh use case domain:

#### 1. Validasi DTO Deklaratif
```rust
use validator::Validate;

#[derive(serde::Deserialize, Validate)]
pub(crate) struct SearchRequestDto {
    #[validate(length(max = 500, message = "Query maksimal 500 karakter"))]
    pub q: Option<String>,
    #[validate(range(min = 1, max = 1000, message = "Nomor halaman minimal 1"))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100, message = "Ukuran halaman antara 1 hingga 100"))]
    pub size: Option<u32>,
}
```

> **Normalisasi & Kontrak Query Kosong**:
> - Parameter `q` dinormalisasi dengan trimming whitespace: `let clean_q = q.as_deref().map(str::trim).filter(|s| !s.is_empty());`.
> - Input `None`, `""`, dan `"   "` diperlakukan sebagai query kosong yang valid (tidak memicu HTTP 400).
> - Respons query kosong mengembalikan status `200 OK` dengan payload: `total: 0`, `items: []`, `took_ms: 0`, dan facets kosong.


#### 2. Sanitasi Path Traversal
Untuk endpoint pembacaan dokumen `/api/documents/:id`, sistem menerapkan pertahanan berlapis terhadap serangan Path Traversal:
1. Ambil path absolut dokumen dari database `document_registry` berdasarkan UUID.
2. Lakukan canonicalize path menggunakan `std::fs::canonicalize`.
3. Verifikasi secara tegas bahwa canonical path dokumen diawali oleh canonical `root_path` dari folder induk yang terdaftar di database (`doc_path.starts_with(&folder_root_path)`).
4. Tolak request yang mengandung karakter `..` atau null-byte `\0` dengan error `PATH_TRAVERSAL_DETECTED`.

#### 3. Validasi Skema Klien & Form Runtime (Frontend Zod)
Untuk menjamin type-safety ujung-ke-ujung antara Rust DTOs dan TypeScript, klien webview desktop menggunakan library `zod` untuk memvalidasi response API dan payload form sebelum dikonsumsi oleh store/komponen:
```typescript
import { z } from 'zod';

export const SearchResultSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  relative_path: z.string(),
  project: z.string().nullable().optional(),
  tags: z.array(z.string()).default([]),
  highlights: z.array(z.object({
    snippet: z.string(),
    line_number: z.number().int().positive().optional(),
  })).default([]),
  score: z.number(),
});

export const FolderFormSchema = z.object({
  root_path: z.string().min(1, "Path direktori wajib diisi"),
});

export type SearchResult = z.infer<typeof SearchResultSchema>;
export type FolderFormData = z.infer<typeof FolderFormSchema>;
```

---

### 8.3 Structured Logging & Observability (Tracing Spans)
Menggunakan framework `tracing` dan `tracing-subscriber` untuk observabilitas menyeluruh tanpa memerlukan server Prometheus eksternal yang berat:

1. **Format Log Berbasis Lingkungan**:
   - **Mode Release / Production**: Format JSON terstruktur (`tracing_subscriber::fmt().json()`) yang siap diindeks atau diparsing oleh log collector.
   - **Mode Development**: Format pretty dengan ANSI color (`tracing_subscriber::fmt().pretty()`) untuk kenyamanan developer.
2. **Contextual Tracing Spans**:
   Setiap request dan background task dibungkus dalam span kontekstual:
   ```rust
   #[tracing::instrument(skip(repo), fields(query = %params.q.as_deref().unwrap_or(""), took_ms))]
   pub(crate) async fn search_documents(...) {
       // Catat latensi eksekusi
   }
   ```
   Setiap baris log otomatis menyertakan `request_id`, `job_id`, `folder_id`, dan waktu proses.
3. **Metrik Performa Internal (`/api/stats` & `/api/health`)**:
   Alih-alih mengekspos endpoint Prometheus terpisah, metrik operasional utama (total dokumen, sebaran per bahasa/ekstensi, ukuran index, dan status latensi koneksi DB & ES) diekspos langsung melalui endpoint `GET /api/stats` dan `GET /api/health` yang dapat diakses langsung oleh UI desktop.
4. **Persistensi Log Sistem & Log Rotation (`tracing-appender`)**:
   Agar aktivitas aplikasi desktop dapat diaudit tanpa memenuhi kapasitas hard disk pengguna:
   - **Direktori Log Sistem Terisolasi**:
     - Linux: `~/.config/LynxSearch/logs/` (atau `$XDG_DATA_HOME/LynxSearch/logs/`)
     - Windows: `%APPDATA%\LynxSearch\logs\`
     - macOS: `~/Library/Application Support/LynxSearch/logs/`
   - **Non-Blocking Daily Rolling Appender**:
     ```rust
     let log_dir = dirs::config_dir()
         .unwrap_or_else(|| PathBuf::from("."))
         .join("LynxSearch")
         .join("logs");

     let file_appender = tracing_appender::rolling::daily(log_dir, "lynx-backend.log");
     let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

     tracing_subscriber::registry()
         .with(env_filter)
         .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
         .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking_appender))
         .init();
     ```
   - **Kebijakan Retensi**: Sistem melakukan rotasi harian dan secara otomatis membersihkan file log yang berusia lebih dari 7 hari saat backend startup.

---

### 8.4 Protokol Keamanan & Secrets Management
- **Localhost Isolation**: Backend Axum 0.8 hanya mengikat ke `127.0.0.1:3001` secara default (tidak pernah `0.0.0.0`), memastikan knowledge base tidak terekspos ke LAN/Wi-Fi publik.
- **Zero Secret Leaks**: Tidak ada credential atau token hardcoded di kode sumber.
- **Manajemen Lingkungan (.env & .env.example)**:
  - `.env` diabaikan oleh `.gitignore`.
  - `.env.example` disiapkan sebagai template konfigurasi resmi:
  ```ini
  DATABASE_URL=postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch
  ELASTICSEARCH_URL=http://127.0.0.1:9200
  BACKEND_BIND_ADDR=127.0.0.1:3001
  RUST_LOG=info,backend=debug
  ```

---

## 9. Strategi Pengujian Komprehensif (Testing Strategy)

Mengikuti metodologi **Backend Development Testing**, **Frontend Engineering Methodology**, **React + Vite Best Practices**, dan pedoman **Rust Backend Testing**, sistem mengadopsi model **Piramida Testing 80-20 (Lean & Deterministic)**:
- **80% Unit Tests**: Murni in-memory, deterministic, ultra-fast (durasi eksekusi < 1 ms per test), zero network/disk I/O. Mencakup domain logic Rust (QueryParser, ScanPlanner, Extractor, QueryBuilder) serta unit test Zustand stores dan formatting helpers frontend via Vitest.
- **20% Integration Tests**: Menguji batas sistem nyata (PostgreSQL 18.6 via `sqlx::test`, Elasticsearch 8.19.22 dengan isolated dynamic index, HTTP Axum via `tower::oneshot`, dan komponen React via React Testing Library + MSW).
- *Catatan Lingkup*: Sesuai batas PRD dan efisiensi portofolio, end-to-end (E2E) UI otomatis yang berat (seperti Playwright Webview) ditiadakan guna mencegah flakiness dan overhead CI, digantikan sepenuhnya oleh pengujian integrasi komponen via Vitest + MSW.

```mermaid
graph TD
    subgraph TestingPyramid [Piramida Testing 80-20]
        Integration[20% Integration Tests: sqlx::test, Elasticsearch Test Index, MSW + RTL Component Tests]
        Unit[80% Unit Tests: Pure Rust Domain, QueryParser, Extractor, ScanPlanner, Zustand Stores, Formatters]
    end
    Integration --> Unit
```

---

### 9.1 Pengujian Backend (Rust, Tokio, SQLx, Elasticsearch)

#### 1. Unit Tests (Pure Domain Services, No I/O)
Diletakkan berdampingan dengan kode modul (`#[cfg(test)] mod tests`):
- **`QueryParser`**:
  - Validasi pemecahan terms bebas, kutipan frase persis (`"borrow checker"`).
  - Validasi filter valid (`language:rust`, `tag:concurrency`, `project:backend`, `extension:rs`, `type:code`).
  - Penanganan kasus edge: filter tidak dikenal (menghasilkan structured warning tanpa panik), string kosong, karakter non-alphanumeric, spasi berlebih.
- **`DocumentExtractor`**:
  - Penentuan judul: heading H1 pertama vs nama file fallback.
  - Ekstraksi YAML front-matter pada file Markdown (key `tags`).
  - Deteksi tipe file (`doc`, `code`, `config`) dan pemetaan bahasa pemrograman dari ekstensi.
  - Aturan domain nama `project`: ekstraksi nama folder tingkat pertama di bawah folder root.
  - Deteksi file biner (null byte scan) dan file melebihi batas ukuran (default 2 MB) -> menghasilkan `ExtractedDoc::Skipped`.
- **`ScanPlanner`**:
  - Determinisme rencana scan: kalkulasi akurat kategori `to_add`, `to_update` (hash berubah), `to_delete` (file fisik hilang), dan `to_skip` (hash & mtime identik).
  - Idempotensi: eksekusi kedua berturut-turut pada state yang sama wajib menghasilkan `ScanPlan` kosong.
- **`SearchQueryBuilder`**:
  - Validasi pembentukan JSON Elasticsearch Query DSL: pembobotan field (`title^3.0`, `tags^2.0`, `content^1.0`), isolasi filter di `bool.filter`, klausa `fuzzy` dan `prefix`, serta definisi `aggs`.

#### 2. Integration Tests (PostgreSQL 18.6 & Elasticsearch 8.19.22 Nyata)
Diletakkan di direktori `crates/backend/tests/`:
- **Isolasi Database PostgreSQL (`sqlx::test`)**:
  - Setiap fungsi test menggunakan atribut `#[sqlx::test(migrations = "./migrations")]`.
  - SQLx secara otomatis membuat database/schema terisolasi sementara per fungsi test dan me-rollback transaksi setelah pengujian selesai, memungkinkan pengujian berjalan **paralel tanpa tabrakan data**.
- **Isolasi Index Elasticsearch (RAII Dynamic Prefix Guard)**:
  - Setiap suite integrasi Elasticsearch membuat index terisolasi dengan nama dinamis ber-UUID: `test_lynx_<uuid>`.
  - Menerapkan struktur RAII `TestIndexGuard` yang mengimplementasikan trait `Drop`:
    ```rust
    pub(crate) struct TestIndexGuard<'a> {
        pub client: &'a Elasticsearch,
        pub index_name: String,
    }

    impl<'a> Drop for TestIndexGuard<'a> {
        fn drop(&mut self) {
            let client = self.client.clone();
            let index = self.index_name.clone();
            tokio::task::spawn(async move {
                let _ = client.indices().delete(IndicesDeleteParts::Index(&[&index])).send().await;
            });
        }
    }
    ```
- **Cakupan Pengujian Integrasi Repository**:
  - `FolderRepository`: CRUD folder, validasi unique root path, update timestamp scan.
  - `DocumentRegistryRepository`: Deduplikasi path relatif, batch upsert, batch deletion saat file hilang.
  - `JobRepository`: Transisi state job (`PENDING` -> `RUNNING` -> `COMPLETED` / `CANCELLED`), pembaruan counter inkremental.
  - `SearchRepository`: Verifikasi analyzer kode (`authenticateUser` mencocokkan `authenticate_user`), verifikasi skor BM25 title > content, verifikasi facet aggregation.
  - `IndexOrchestrator`: Alur end-to-end import folder, penanganan kegagalan 1 file tanpa menggugurkan job (resilience), dan jaminan strictly 1 job aktif per folder (lock mutex).

#### 3. API Contract & Middleware Tests
- Menggunakan `tower::ServiceExt::oneshot` pada router Axum tanpa perlu membuka port TCP socket nyata.
- Memverifikasi response status code HTTP (`200`, `202`, `400`, `404`, `409`, `500`), format JSON error seragam, header CORS lokal, dan tracing middleware.

#### 4. Manajemen Test Fixtures (Hybrid Approach)
- **Static Fixtures (`crates/backend/tests/fixtures/knowledge_base/`)**:
  - `notes/rust/ownership.md` (Markdown dengan YAML front-matter tags `[rust, memory]`).
  - `src/auth/service.rs` (Kode Rust berisi identifier `authenticateUser` dan `validate_token`).
  - `configs/app.toml` (Konfigurasi TOML).
  - `assets/sample.bin` (Dummy binary file untuk uji file skip).
  - `.git/HEAD` dan `node_modules/dummy.js` (Untuk menguji aturan ignore bawaan).
- **Dynamic Scale Fixture Generator (`crates/backend/tests/common/generator.rs`)**:
  - Helper fungsi menggunakan `tempfile::tempdir()` untuk men-generate 1.000–5.000 file catatan teks secara programatik guna pengujian performa bulk indexing dan re-scan benchmark.

---

### 9.2 Pengujian Desktop Frontend (React 19.3, Vite 8.1, Tauri 2.12)

#### 1. Tooling Frontend Testing
- **Test Runner**: `Vitest` (terintegrasi langsung dengan konfigurasi `vite.config.ts`, super cepat via in-memory Happy-DOM / JSDOM).
- **Component Testing**: `@testing-library/react` dan `@testing-library/user-event` (menguji perilaku berbasis interaksi pengguna, bukan detail implementasi internal).
- **API Mocking**: `Mock Service Worker (MSW)` v2 (meng-intercept network request `http://127.0.0.1:3001` pada level HTTP client, menjamin pengujian realistis tanpa backend nyata).
- **Accessibility Testing**: `vitest-axe` (audit otomatis standar kepatuhan WAI-ARIA dan kontras warna).

#### 2. Konfigurasi `apps/desktop/vitest.config.ts`
```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'happy-dom',
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules/', 'src/test/'],
    },
  },
});
```

#### 3. Setup MSW Server (`apps/desktop/src/test/mocks/handlers.ts`)
```typescript
import { http, HttpResponse } from 'msw';

export const handlers = [
  // Mock Health API
  http.get('http://127.0.0.1:3001/api/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      services: {
        backend: { status: 'up' },
        database: { status: 'up', latency_ms: 2 },
        elasticsearch: { status: 'up', latency_ms: 5 },
      },
    });
  }),

  // Mock Search API
  http.get('http://127.0.0.1:3001/api/search', ({ request }) => {
    const url = new URL(request.url);
    const query = url.searchParams.get('q');

    if (query === 'empty') {
      return HttpResponse.json({ total: 0, results: [], facets: {} });
    }

    return HttpResponse.json({
      total: 1,
      results: [
        {
          id: 'test-doc-1',
          title: 'Rust Ownership',
          relative_path: 'notes/ownership.md',
          score: 12.5,
          language: 'rust',
          highlights: [{ snippet: 'Dalam <em>Rust</em> ownership...', line_number: 10 }],
        },
      ],
      facets: { languages: [{ key: 'rust', count: 1 }] },
    });
  }),
];
```

#### 4. Pengujian 4 Micro-States pada Komponen
Setiap komponen data wajib diuji perilakunya terhadap 4 state:
```typescript
describe('ResultList Component', () => {
  it('menampilkan Skeleton saat loading', () => {
    // Render dengan query status pending -> pastikan skeleton tampil
  });

  it('menampilkan pesan Empty kontekstual saat hasil 0', async () => {
    // Render dengan query 'empty' -> pastikan pesan "Tidak ada hasil" & tombol Reset muncul
  });

  it('menampilkan Alert Error saat API 500 dengan tombol Retry', async () => {
    // Override handler ke status 500 -> pastikan banner error tampil
  });

  it('menampilkan daftar kartu hasil dan highlight saat berhasil', async () => {
    // Render dengan data -> verifikasi judul, path, badge baris muncul
  });
});
```

#### 5. Pengujian Virtualized Code Viewer
- Memverifikasi `@tanstack/react-virtual` hanya me-mount elemen baris DOM yang berada dalam viewport + overscan buffer.
- Memverifikasi baris kode dengan highlight memiliki class styling aktif dan auto-scroll ke baris target saat dokumen pertama kali dibuka.

#### 6. Pengujian Aksesibilitas (a11y)
```typescript
import { render } from '@testing-library/react';
import { axe } from 'vitest-axe';
import { SearchBar } from '@/components/search/SearchBar';

it('SearchBar bebas dari pelanggaran aksesibilitas WAI-ARIA', async () => {
  const { container } = render(<SearchBar />);
  const results = await axe(container);
  expect(results).toHaveNoViolations();
});
```

---

### 9.3 Matriks Penelusuran Pengujian (Traceability Matrix)

| Modul / Komponen | Tipe Pengujian | Tooling Utama | Aspek Kritis yang Diverifikasi |
|---|---|---|---|
| **Query Parser** | Unit | Rust built-in | Operator kutipan, toleransi typo, parsing `language:` & `tag:`. |
| **Document Extractor** | Unit | Rust built-in | Ekstraksi H1, metadata project, skip binary & file > 2MB. |
| **Scan Planner** | Unit | Rust built-in | Deteksi diff file (add, update, delete, skip), idempotensi. |
| **Search Query Builder**| Unit | Rust built-in | Validasi Elasticsearch JSON Query DSL, `bool.filter`, BM25 boost. |
| **Database Repositories**| Integrasi | `sqlx::test` | Skema berversi, query parameterisasi (tanpa `SELECT *`), transaksi. |
| **Search Repository** | Integrasi | Elasticsearch + RAII | Analyzer camelCase/snake_case, ranking BM25, alias rebuild. |
| **Index Orchestrator** | Integrasi | Tokio + MPSC | Background worker, folder mutex lock, per-file error resilience. |
| **HTTP API Handlers** | API Contract | Axum + `tower::oneshot`| Status code HTTP, validasi input, error JSON seragam. |
| **Zustand Stores** | Unit | Vitest | Mutasi state filter, sinkronisasi query, reset filter. |
| **React Components** | Integrasi UI | RTL + MSW | 4 micro-states (loading, empty, error, success), debounce keyboard. |
| **Code Viewer** | Performa UI | Vitest + Virtualizer | DOM recycling, rendering lancar pada 5.000+ baris kode. |
| **Desktop UI Integration** | Integrasi UI | Vitest + RTL + MSW | Alur interaksi pencarian, preview, facet selection, dan error banner. |

---

### 9.4 CI/CD Quality Gates & Performance Budgets

Sebelum kode dapat di-merge atau dirilis, seluruh gate otomatis berikut wajib lulus (exit code 0):

```bash
# 1. Rust Backend Quality Gates
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features

# 2. Frontend Quality Gates
npm run lint
npm run typecheck       # tsc --noEmit
npm run test:coverage   # Vitest (Threshold: Minimal 80% coverage pada domain & store)

# 3. Bundle Budget Check (Max limits)
# Total JS bundle gzipped < 450 KB, CSS gzipped < 50 KB
npm run build
python3 scripts/bundle-budget-checker.py --max-js 450 --max-css 50
```

> **Target Lingkungan Verifikasi OS (Q9):**  
> Seluruh kode backend Rust dan frontend Tauri dirancang netral lintas-platform (tidak ada hardcode path delimiter seperti `\` atau `/`, selalu gunakan `std::path::PathBuf`). Untuk pengujian dan verifikasi perintah otomatis pada rencana kerja `TASK.md`, Linux dijadikan target acuan primer lingkungan eksekusi.

---

## 10. Roadmap Implementasi Bertahap (Fase 1 – 8)

| Fase | Komponen Utama | Output Konkret |
|---|---|---|
| **Fase 1** | Project & Workspace Setup | Root `Cargo.toml`, layout workspace, `docker-compose.yml` (Postgres 18.6 + ES 8.19.22), tooling setup (clippy, rustfmt, cargo-nextest). |
| **Fase 2** | Backend Core & Axum API | Routing Axum 0.8, logging `tracing`, error handling taxonomy, health check endpoint (`/api/health`, `/api/health/live`, `/api/health/ready`). |
| **Fase 3** | Repositories & Migrations | SQLx migrations, Elasticsearch mapping & alias, `FolderRepository`, `RegistryRepository`, `JobRepository`, `SettingsRepository`, `SearchRepository`. |
| **Fase 4** | Document Indexing Engine | `DocumentExtractor`, `ScanPlanner`, `IndexOrchestrator` dengan Tokio MPSC worker, graceful cancellation token, job progress polling API. |
| **Fase 5** | Basic Search Engine | `SearchQueryBuilder`, multi-match BM25 scoring, highlight generator, line number offset calculator, pagination, `/api/search`. |
| **Fase 6** | Tauri Desktop MVP | Tauri 2.12 shell, React 19.3 search bar, basic result list, native folder selector dialog, document preview panel. |
| **Fase 7** | Advanced Search & Facets | `QueryParser` (inline `key:val` termasuk `tag:`), dynamic facet sidebar (`aggs`), fuzzy + prefix search, `/api/suggest` autocomplete. |
| **Fase 8** | Code Search Mastery | Code identifier analyzer (camelCase & snake_case), highlight snippet dengan nomor baris, virtualized code viewer, filter bahasa & tipe. |

---

*Dokumen arsitektur ini merupakan cetak biru resmi untuk implementasi kode proyek LynxSearch.*
