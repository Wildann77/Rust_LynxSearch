# LynxSearch 🔍

> **Mesin Pencari Desktop Lokal Mandiri Berkinerja Tinggi untuk Knowledge Base Pengembang.**  
> Dirancang dengan Rust (Axum 0.8), Desktop Tauri 2.12 + React 19, Elasticsearch 8.19 (BM25), dan PostgreSQL 18.6.

---

## 1. Quick Start & Setup (Clean Clone)

### Prasyarat Sistem
- **Rust Toolchain:** Version 1.98+ (Edition 2024).
- **Node.js:** Version 20+ & **npm** 10+.
- **Docker & Docker Compose:** Docker Desktop atau Docker Engine Linux.
- **OS:** Linux (X11 / Wayland), macOS, atau Windows (WSL2 / Native).

### Langkah Instalasi

```bash
# 1. Clone repositori
git clone https://github.com/Wildann77/Rust_LynxSearch.git
cd Rust_LynxSearch

# 2. Siapkan file konfigurasi environment
cp .env.example .env

# 3. Jalankan container dependensi (PostgreSQL 18.6 & Elasticsearch 8.19.22)
docker compose -f docker/docker-compose.yml up -d

# 4. Verifikasi status kesehatan container
docker ps

# 5. Pasang dependensi frontend
cd apps/desktop
npm install
cd ../..

# 6. Kompilasi dan verifikasi backend Rust
cargo check
```

---

## 2. Arsitektur & Desain Sistem

LynxSearch menerapkan **Clean Hexagonal Architecture (Ports and Adapters)** secara ketat dengan pemisahan proses mutlak antara Backend dan Frontend:

```text
       ┌────────────────────────────────────────────────────────┐
       │                       API Layer                        │
       │           (Axum 0.8 Handlers, DTOs, Routing)           │
       └───────────────────────────┬────────────────────────────┘
                                   │
                                   ▼
       ┌────────────────────────────────────────────────────────┐
       │                   Application Layer                    │
       │    (Use Cases, Commands, Queries, Index Orchestrator)   │
       └──────────────┬──────────────────────────┬──────────────┘
                      │                          │
                      ▼                          ▼
       ┌───────────────────────────┐ ┌──────────────────────────┐
       │       Domain Layer        │ │       Output Ports       │
       │ (Entities, Value Objects, │ │ (Traits: SearchRepo,     │
       │  ScanPlanner, Extractor)  │ │  MetadataRepo, FsDriver) │
       └───────────────────────────┘ └───────────▲──────────────┘
                                                 │ Implements
       ┌─────────────────────────────────────────┴──────────────┐
       │                  Infrastructure Layer                  │
       │     (SQLx Postgres, Elasticsearch 8.x, WalkDir)        │
       └────────────────────────────────────────────────────────┘
```

- **Backend (Rust Service):** Berjalan sebagai proses mandiri terpisah pada `127.0.0.1:3001`.
- **Desktop (Tauri 2 / React 19):** Proses desktop independen. Tidak memuat logika bisnis pencarian atau indexing lokal; seluruh interaksi dilakukan via HTTP REST Client ke backend.
- **Domain Layer:** 100% bebas dari Axum, SQLx, client Elasticsearch, maupun I/O filesystem langsung.

---

## 3. Batasan Sumber Kebenaran (Source-of-Truth Boundaries)

1. **Isi Dokumen (File di Disk):**  
   Berkas fisik di filesystem pengguna adalah satu-satunya sumber kebenaran isi. Isi dokumen tidak diduplikasi atau disimpan di database relasional.
2. **Metadata & State (PostgreSQL 18.6):**  
   PostgreSQL adalah sumber kebenaran mutlak untuk data folder, registry dokumen (path, SHA-256 hash, status `INDEXED`/`EXCLUDED`), riwayat progress job, dan bobot BM25.
3. **Indeks Pencarian (Elasticsearch 8.19.22):**  
   Elasticsearch diperlakukan sebagai indeks *disposable*. Indeks dapat dihapus atau dibangun ulang kapan saja secara idempoten dari PostgreSQL dan berkas di disk.

---

## 4. Dependensi Docker Lokal

Dijalankan via `docker/docker-compose.yml`:
- **PostgreSQL 18.6 (`lynx_postgres`):**
  - Host/Port: `127.0.0.1:5432`
  - Database: `lynxsearch`
  - Migrasi skema otomatis dijalankan oleh backend saat bootstrap.
- **Elasticsearch 8.19.22 (`lynx_elasticsearch`):**
  - Host/Port: `127.0.0.1:9200`
  - Alokasi memori heap dibatasi ketat: `-Xms512m -Xmx512m`.
  - Single-node discovery (`discovery.type=single-node`).

---

## 5. Cara Menjalankan Layanan

### Menjalankan Backend Secara Mandiri
```bash
# Jalankan backend service (Axum 0.8 listening pada 127.0.0.1:3001)
cargo run --bin backend

# Atau jalankan dengan log debug
RUST_LOG=debug cargo run --bin backend
```

Endpoint verifikasi:
```bash
curl http://127.0.0.1:3001/api/health
```

### Menjalankan Desktop UI Secara Mandiri
```bash
cd apps/desktop

# Mode Desktop Native (Tauri 2 dev window)
npm run tauri dev

# Atau mode browser preview
npm run dev
```

---

## 6. Semantik Indexing & Sinkronisasi

- **Pemindaian Inkremental (SHA-256):**  
  File traversal menghormati `.gitignore` melalui crate `ignore`. Setiap berkas dihitung hash SHA-256 kontennya dan dibandingkan dengan data di `document_registry`.
- **Perencanaan Perubahan (`ScanPlanner`):**  
  Mengklasifikasikan setiap file ke dalam status:
  - `ADD`: Berkas baru belum terdaftar.
  - `UPDATE`: Hash konten berubah dari pemindaian sebelumnya.
  - `DELETE`: Berkas fisik telah dihapus dari disk.
  - `SKIP`: Ukuran berkas, mtime, dan hash tidak berubah (pemindaian instan).
- **Pemrosesan Batch & Ketahanan Error:**  
  Dokumen dikirim ke Elasticsearch dalam batch 100 dokumen. Kegagalan satu file tidak membatalkan keseluruhan job.
- **Pembatalan Aman:**  
  Setiap job indexing mendukung `CancellationToken` (Tokio) untuk pembatalan instan tanpa merusak konsistensi metadata.

---

## 7. Semantik Tombstone EXCLUDED

Ketika pengguna mengecualikan dokumen tertentu dari hasil pencarian:
1. Status dokumen pada PostgreSQL `document_registry` diubah menjadi `EXCLUDED`.
2. Dokumen dihapus dari indeks Elasticsearch.
3. Saat folder dipindai ulang (*re-scan*), `ScanPlanner` mendeteksi status `EXCLUDED` dan **tidak** akan mengindeks kembali dokumen tersebut (*no resurrection*).
4. Dokumen dapat dipulihkan kapan saja melalui endpoint restore.

---

## 8. Zero-Downtime Rebuild Index (Alias Swap)

Rebuild indeks penuh (`POST /api/index/rebuild`) menerapkan strategi alias pointer atomik:
1. Membuat indeks versi baru (misal: `lynx_documents_v9`) dengan mapping dan custom analyzer terbaru.
2. Membaca seluruh berkas aktif dari `document_registry` dan melakukan bulk indexing ke indeks baru.
3. Melakukan atomic alias swap pada Elasticsearch (`actions: [add: new_index, remove: old_index]`).
4. Pencarian tetap 100% tersedia tanpa downtime selama proses rebuild berlangsung.
5. Menghapus indeks lama setelah alias berhasil dialihkan.

---

## 9. Sintaks Pencarian & Code Search

- **Pencarian Teks Penuh BM25:**  
  Perankingan relevansi terbobot secara dinamis: `Title (3.0)` > `Tags (2.0)` > `Content (1.0)`. Bobot dapat diubah secara langsung via UI Settings.
- **Identifier Code Matching:**  
  Custom analyzer TextMate / Word Delimiter mengenali identifier:
  - Mencari `authenticateUser` cocok dengan `authenticate_user`.
  - Mencari `get_user_by_id` cocok dengan `getUserById`.
- **Highlighting Presisi:**  
  Fragmen snippet hasil pencarian menyertakan tag `<mark>` dan estimasi nomor baris kemunculan kode.
- **Toleransi Typo (Fuzzy) & Prefix Search:**  
  Mendukung pencarian toleran kesalahan ketik (Levenshtein distance terkalibrasi) dan prefix search instan.

---

## 10. Filter Inline & Facet Sidebar

Sintaks filter inline didukung langsung di bilah pencarian dan tersinkronisasi dua arah dengan facet sidebar:

| Kunci Filter | Contoh Penggunaan | Keterangan |
|---|---|---|
| `type:` | `type:code` atau `type:doc` | Menyaring berdasarkan tipe dokumen (`code`, `doc`, `config`). |
| `language:` / `lang:` | `language:rust` | Menyaring bahasa pemrograman atau format berkas. |
| `tag:` | `tag:guide` | Menyaring berdasarkan tag YAML front-matter catatan Markdown. |
| `ext:` | `ext:rs` | Menyaring ekstensi file tertentu. |
| `project:` | `project:lynx` | Menyaring berdasarkan folder root / nama proyek. |

Contoh kombinasi query:
```text
authenticateUser type:code language:rust tag:auth
```

---

## 11. Format Berkas Yang Didukung

- **Source Code:** `.rs`, `.ts`, `.tsx`, `.js`, `.jsx`, `.py`, `.go`, `.c`, `.cpp`, `.cs`, `.java`, `.kt`, `.rb`, `.php`, `.swift`, `.sh`, `.sql`, `.lua`.
- **Catatan & Dokumentasi:** `.md`, `.markdown` (dengan dukungan parsing YAML front-matter).
- **Konfigurasi & Data:** `.json`, `.yaml`, `.yml`, `.toml`, `.xml`, `.html`, `.css`, `.scss`.

---

## 12. Proteksi Keamanan & Eksklusi Berkas Rahasia

- **Bind Localhost Ketat:** Backend Axum hanya melayani koneksi loopback lokal `127.0.0.1:3001`.
- **Proteksi Path Traversal:** Validasi ketat DTO menolak komponen path `..`, null byte, dan symlink keluar root folder.
- **Secret & Sensitive File Exclusion:**  
  Sistem secara otomatis menolak dan mengabaikan berkas kredensial sensitif:
  - `.env`, `.env.*`
  - `id_rsa`, `id_ed25519`, `*.pem`, `*.key`
  - `credentials.json`, `token.json`
- **Biner & Batas Ukuran:**  
  Deteksi nomor ajaib binary (`infer`) dan batas ukuran maksimum berkas (default 2 MB) mencegah pemborosan memori.

---

## 13. Pintasan Keyboard (Keyboard Shortcuts)

| Pintasan Keyboard | Aksi |
|---|---|
| `/` atau `Ctrl + K` | Fokus langsung ke bilah pencarian global. |
| `Esc` | Hapus teks pencarian / tutup popover autocomplete / tutup panel. |
| `↑` / `↓` | Navigasi vertikal pada daftar hasil pencarian. |
| `Enter` | Buka dokumen terpilih pada panel preview virtual. |
| `Ctrl + O` | Buka modal Kelola Folder (Folder Manager). |
| `Ctrl + ,` | Buka modal Pengaturan Sistem & Bobot BM25. |
| `Ctrl + Shift + C` | Salin seluruh isi berkas aktif di panel preview. |

---

## 14. Strategi Pengujian & Kualitas (Quality Gates)

Semua gate kualitas wajib lulus dengan exit code 0:

```bash
# 1. Rust Formatting Check
cargo fmt --check

# 2. Rust Clippy Strict Linting
cargo clippy --all-targets --all-features -- -D warnings

# 3. Rust Nextest Suite (Unit + Integration + Security Gates: 373 Tests)
CI=1 cargo nextest run --all-features

# 4. Frontend TypeScript Typecheck
npm run --prefix apps/desktop typecheck

# 5. Frontend ESLint
npm run --prefix apps/desktop lint

# 6. Frontend Vitest Coverage (286 Tests, > 80% Coverage)
npm run --prefix apps/desktop test:coverage

# 7. Frontend Production Build & Bundle Budget Gate (JS < 450 KB, CSS < 50 KB)
npm run --prefix apps/desktop build
python3 scripts/bundle-budget-checker.py --max-js 450 --max-css 50
```

---

## 15. Item di Luar Cakupan (Out-of-Scope)

Sesuai dokumen spesifikasi PRD & Architecture:
- ❌ **Tidak ada Vector / Semantic / KNN Search** (Fokus pada BM25 berkinerja tinggi).
- ❌ **Tidak ada Parser PDF / OCR / Word / Gambar**.
- ❌ **Tidak ada Real-time Filesystem Watcher** (Pemindaian dilakukan secara manual atau inkremental terjadwal demi efisiensi resource).
- ❌ **Tidak ada Autentikasi Multi-User / Cloud Sync** (100% lokal pribadi).

---

## 16. Rekonsiliasi & Deviasi

Seluruh implementasi berjalan selaras tanpa deviasi negatif terhadap spesifikasi resmi ([PRD](PRD-LynxSearch.md), [ARCHITECTURE](ARCHITECTURE.md), [DESIGN](DESIGN.md), [TASK](TASK.md)).
- Client library Elasticsearch dibekukan pada crate versi `8.19` untuk menjamin kompatibilitas 100% terhadap Elasticsearch server `8.19.22`.
- Highlighting Shiki dioptimasi via `@shikijs/core` dan JavaScript Regex Engine untuk mempertahankan bundle gzip di bawah anggaran 450 KB.
