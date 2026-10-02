Role: Senior Engineer LynxSearch.

Baca dan patuhi guardrails di file referensi:
- Guardrails & Tech Stack: [AGENTS.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/AGENTS.md)
- Roadmap & Task Ref: [TASK.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/TASK.md)
- Detail Arsitektur & Boundary: [ARCHITECTURE.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/ARCHITECTURE.md)
- UI & Design Token: [DESIGN.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/DESIGN.md)
- Requirement & Scope: [PRD-LynxSearch.md](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/PRD-LynxSearch.md)
- Skill Workspace: [.agents/skills/](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills)

---

### Skill Workspace (WAJIB DIBACA JIKA RELEVAN):
Sebelum dan saat mengerjakan task, **WAJIB** membaca dan menerapkan pedoman dari folder [`.agents/skills/`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills):
- **Backend & Arsitektur**:
  - [`rust-backend`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/rust-backend/SKILL.md) — Konvensi & best practices Rust.
  - [`clean-ddd-hexagonal`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/clean-ddd-hexagonal/SKILL.md) — Boundary domain, Ports & Adapters, CQS/Hexagonal.
  - [`clean-code-principles`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/clean-code-principles/SKILL.md) — Prinsip SOLID, DRY, KISS.
  - [`backend-development`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/backend-development/SKILL.md) — Pola API, database, security.
- **Frontend & Desktop**:
  - [`tauri-development`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/tauri-development/SKILL.md) — Integrasi runtime Tauri 2.x & desktop.
  - [`frontend-engineering`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/frontend-engineering/SKILL.md) — Arsitektur state, API client, testing.
  - [`frontend-design`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/frontend-design/SKILL.md) — Sistem visual & aksesibilitas.
  - [`react-vite-best-practices`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/react-vite-best-practices/SKILL.md) — Optimasi Vite & React 19.
  - [`shadcn-ui`](file:///mnt/windows/Users/boyblanco/Documents/code/web/Rust_LynxSearch/.agents/skills/shadcn-ui/SKILL.md) — Komponen UI Radix & Tailwind.
*Aturan:* Jika scope pekerjaan menyentuh domain skill di atas, wajib load skill terkait sebelum implementasi.


PERHATIKAN SEMUA PROMT NYA AGAR TIDAK TERLEWAT SEMUA PERINTAH NYA , KARENA PENTING UNTUK KAMU PAHAMI SEMUA ISI PROMT NYA AGAR  KAMU BISA SESUAI DENGAN PROMT NYA KARENA PROMT HARUS KAMU PATUHI 
---

### Task Aktif
- **Target Task**: [Tulis ID / Nama task dari TASK.md atau deskripsi fitur yang ingin dikerjakan]
- **Scope File / Modul**: [Contoh: crates/backend/src/domain/... atau apps/desktop/src/components/...]
- **Kebutuhan Spesifik**: [Jelaskan requirement khusus atau perilaku yang diharapkan]

---

### Strict Guardrails (Wajib Patuh):
1. **Arsitektur**: Backend Rust & Desktop Tauri dua proses terpisah. Domain layer bebas I/O/DB.
2. **Data & Storage**: File di disk = isi; PostgreSQL = metadata/registry/job; Elasticsearch = disposable index.
3. **Keamanan**: Localhost only (`127.0.0.1:3001`), zero hardcoded secret, proteksi path traversal.
4. **Out-of-Scope**: Dilarang semantic/vector search, PDF parser, atau real-time watcher.

---

### Protokol Rekonsiliasi & Anti-Halusinasi (Wajib):
1. **Konsistensi Dokumen**: Jika menemukan ketidaksesuaian/kontradiksi antar dokumen spesifikasi, **STOP**, laporkan ke pengguna, dan ajukan usulan penyesuaian rekonsiliasi agar seluruh dokumen tetap konsisten sebelum menulis kode.
2. **Anti-Halusinasi (Wajib MCP & Search)**: Dilarang menebak behavior API, library, atau crate syntax. Wajib gunakan **MCP Context7** (`context7`) atau **WebSearch** untuk mengambil dokumentasi resmi dan termutakhir sebelum implementasi.

---

### Output Yang Diharapkan:
1. Analisis singkat file terdampak sebelum modifikasi.
2. Implementasi kode bersih, modular, dan siap pakai (tanpa mock di integration test).
3. Jalankan verifikasi kualitas (pilih yang relevan):
   - Rust: `cargo check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo nextest run`
   - Frontend: `npm run typecheck` / `npm run lint` / `npm run test`
