# PRD: LynxSearch — Local Developer Knowledge Search Engine

> Status: Draft v1.0 · Cakupan: **Fase 1–8** (MVP + pencarian lanjutan + code search)
> Stack: Rust (Axum, Tokio) · Elasticsearch 8.19.22 · PostgreSQL 18.6 · Tauri + React + TypeScript

---

## Problem Statement

Seorang developer mengumpulkan pengetahuan dalam bentuk ribuan file yang tersebar di banyak folder: catatan belajar Markdown, README, dokumentasi API, snippet kode, file konfigurasi, dan catatan proyek. Ketika butuh sesuatu, misalnya "bagaimana saya mencatat konsep ownership di Rust", developer itu hanya punya dua pilihan buruk:

- **Pencarian bawaan OS/editor** yang mencocokkan teks mentah tanpa peringkat relevansi, tidak toleran terhadap salah ketik, tidak mengerti bahwa `authenticateUser` dan `authenticate_user` adalah hal yang sama, dan tidak memberi tahu *mengapa* sebuah dokumen dianggap relevan.
- **Grep/ripgrep** yang cepat dan akurat untuk pola persis, tetapi tidak punya peringkat, tidak punya facet, tidak punya preview yang nyaman, dan sulit dipakai untuk pertanyaan yang kabur.

Akibatnya, pengetahuan yang sudah dikumpulkan susah ditemukan kembali, dan developer sering menulis ulang atau mencari ulang hal yang sebenarnya sudah ia punya.

Selain masalah pengguna tersebut, ada masalah **pembelajaran**: pemilik proyek ingin belajar Rust, Elasticsearch, dan Tauri secara praktis. Proyek CRUD biasa tidak menyentuh konsep inti search engine (analyzer, inverted index, BM25, aggregation, highlighting), sedangkan "Mini Google" atau "clone Elasticsearch" terlalu besar untuk selesai. Dibutuhkan proyek dengan skala yang selesai-able tetapi cukup dalam untuk benar-benar mengajarkan information retrieval dan arsitektur backend.

## Solution

**LynxSearch** adalah aplikasi desktop untuk mencari *knowledge base developer* secara lokal. Developer mendaftarkan satu atau lebih folder; sistem membaca file Markdown, TXT, source code, dan konfigurasi di dalamnya, lalu mengindeksnya ke Elasticsearch. Developer kemudian mengetik query di aplikasi desktop dan mendapat:

- hasil **terurut berdasarkan relevansi** (BM25 dengan pembobotan field), lengkap dengan skor,
- **highlight** pada potongan teks atau kode yang cocok, termasuk **nomor baris**,
- **filter** lewat sintaks inline (`rust ownership language:rust`) dan lewat **sidebar facet** dengan jumlah per kategori,
- **toleransi salah ketik** (fuzzy), pencarian prefix, dan **autocomplete**,
- **code search** yang mengerti penamaan identifier (camelCase dan snake_case),
- **preview** dokumen penuh.

Sistem terdiri dari **dua service terpisah**: backend Rust (Axum) yang memegang seluruh logika bisnis, dan aplikasi desktop Tauri yang hanya menjadi klien HTTP. Hanya **PostgreSQL dan Elasticsearch** yang berjalan di Docker. PostgreSQL menyimpan metadata, status job, dan pengaturan; Elasticsearch menyimpan konten yang bisa dicari dan dapat dibangun ulang kapan saja dari file + metadata.

Pembaruan index dilakukan **manual dan inkremental**: developer menekan Import atau Re-scan, dan sistem hanya memproses file yang baru, berubah (berdasarkan hash), atau terhapus.

---

## User Stories

### A. Menjalankan dan memantau sistem

1. Sebagai developer, saya ingin menyalakan Elasticsearch dan PostgreSQL dengan satu perintah Docker Compose, agar saya tidak perlu memasang keduanya secara manual.
2. Sebagai developer, saya ingin menjalankan backend Rust sebagai proses terpisah dari aplikasi desktop, agar saya bisa menghentikan, memulai ulang, atau men-debug-nya sendiri.
3. Sebagai developer, saya ingin aplikasi desktop menampilkan status koneksi ke backend, agar saya tahu kapan backend belum jalan.
4. Sebagai developer, saya ingin endpoint kesehatan yang melaporkan status backend, Elasticsearch, dan PostgreSQL secara terpisah, agar saya tahu komponen mana yang bermasalah.
5. Sebagai developer, saya ingin pesan error yang jelas ketika Elasticsearch atau PostgreSQL tidak tersedia, agar saya tidak melihat layar kosong tanpa penjelasan.
6. Sebagai developer, saya ingin backend hanya menerima koneksi dari mesin lokal secara default, agar knowledge base pribadi saya tidak terbuka ke jaringan.

### B. Mendaftarkan folder dan melakukan import pertama

7. Sebagai developer, saya ingin memilih folder lewat dialog folder native, agar saya tidak perlu mengetik path secara manual.
8. Sebagai developer, saya ingin mendaftarkan lebih dari satu folder root, agar catatan belajar dan proyek kode saya yang terpisah bisa dicari sekaligus.
9. Sebagai developer, saya ingin melihat daftar folder yang sudah terdaftar beserta waktu scan terakhir dan jumlah dokumennya, agar saya tahu apa saja yang sudah di-index.
10. Sebagai developer, saya ingin melihat progres indexing (jumlah file diproses, dilewati, gagal), agar saya tahu proses sedang berjalan dan kapan selesai.
11. Sebagai developer, saya ingin indexing berjalan di latar belakang, agar aplikasi tetap bisa dipakai mencari saat import besar berlangsung.
12. Sebagai developer, saya ingin sistem otomatis mengabaikan folder yang bukan pengetahuan (misalnya `.git`, `node_modules`, `target`, `dist`), agar index tidak penuh sampah dan indexing cepat.
13. Sebagai developer, saya ingin sistem melewati file biner dan file yang terlalu besar, agar indexing tidak macet atau menghasilkan hasil aneh.
14. Sebagai developer, saya ingin mendapat ringkasan akhir job (berhasil, dilewati beserta alasannya, gagal beserta alasannya), agar saya bisa memperbaiki masalah tertentu.
15. Sebagai developer, saya ingin satu file yang rusak atau tidak terbaca tidak menggagalkan seluruh job, agar import ribuan file tetap selesai.

### C. Menjaga index tetap segar

16. Sebagai developer, saya ingin menekan tombol Re-scan pada sebuah folder, agar perubahan terbaru masuk ke index.
17. Sebagai developer, saya ingin Re-scan hanya memproses file yang baru atau isinya berubah, agar proses cepat meski folder besar.
18. Sebagai developer, saya ingin dokumen otomatis dihapus dari index ketika file aslinya sudah tidak ada saat Re-scan, agar saya tidak mendapat hasil untuk file hantu.
19. Sebagai developer, saya ingin file yang hanya berganti nama atau pindah dikenali dengan benar, agar tidak muncul dokumen ganda.
20. Sebagai developer, saya ingin menghapus sebuah folder dari daftar beserta seluruh dokumennya dari index, agar saya bisa membersihkan knowledge base.
21. Sebagai developer, saya ingin menghapus satu dokumen dari index tanpa menghapus file aslinya, agar saya bisa menyingkirkan hasil yang mengganggu dan dokumen tersebut tidak muncul kembali saat Re-scan berikutnya.
22. Sebagai developer, saya ingin membangun ulang seluruh index Elasticsearch dari metadata dan file, agar saya bisa pulih dari kerusakan index atau setelah mengubah mapping/analyzer.
23. Sebagai developer, saya ingin menjalankan Re-scan dua kali berturut-turut tanpa efek samping (idempoten), agar saya tidak takut menekannya berulang kali.
24. Sebagai developer, saya ingin hanya satu job indexing aktif per folder pada satu waktu, agar tidak terjadi balapan yang merusak data.

### D. Pencarian dasar

25. Sebagai developer, saya ingin mengetik query di kotak pencarian dan langsung mendapat hasil terurut berdasarkan relevansi, agar dokumen paling berguna muncul di atas.
26. Sebagai developer, saya ingin query beberapa kata (misalnya `rust ownership`) dicocokkan di judul, tag, dan isi dokumen, agar saya tidak perlu tahu persis di field mana kata itu berada.
27. Sebagai developer, saya ingin kecocokan di judul dan tag dinilai lebih tinggi daripada kecocokan di isi, agar dokumen yang memang berjudul sesuai query muncul lebih dulu.
28. Sebagai developer, saya ingin melihat skor relevansi setiap hasil, agar saya bisa memahami dan membandingkan peringkat.
29. Sebagai developer, saya ingin melihat total hasil dan waktu pencarian, agar saya tahu skala dan kecepatan pencarian.
30. Sebagai developer, saya ingin hasil dibagi per halaman (pagination), agar daftar panjang tetap cepat dan mudah dijelajahi.
31. Sebagai developer, saya ingin pesan "tidak ada hasil" yang membantu (misalnya saran memeriksa ejaan atau melepas filter), agar saya tahu langkah berikutnya.
32. Sebagai developer, saya ingin query kosong atau hanya berisi spasi ditangani dengan sopan, agar aplikasi tidak error.
33. Sebagai developer, saya ingin karakter khusus di query (tanda kutip, kurung, garis miring) tidak merusak pencarian, agar saya bisa mencari teks apa adanya.
34. Sebagai developer, saya ingin pencarian tidak membedakan huruf besar-kecil, agar `Rust` dan `rust` sama saja.

### E. Hasil, highlight, dan preview

35. Sebagai developer, saya ingin setiap hasil menampilkan judul, path relatif, dan potongan teks, agar saya bisa menilai relevansi tanpa membuka dokumen.
36. Sebagai developer, saya ingin kata yang cocok di-highlight pada potongan teks, agar mata saya langsung tertuju pada konteks yang relevan.
37. Sebagai developer, saya ingin beberapa potongan terbaik per dokumen, agar saya melihat berbagai konteks kemunculan kata.
38. Sebagai developer, saya ingin mengklik hasil untuk membuka preview isi dokumen penuh di dalam aplikasi, agar saya tidak perlu berpindah ke editor.
39. Sebagai developer, saya ingin Markdown dirender dengan rapi pada preview dan kode ditampilkan dengan font monospace, agar mudah dibaca.
40. Sebagai developer, saya ingin kata yang dicari di-highlight juga pada preview, agar saya bisa menemukan bagian relevan di dokumen panjang.
41. Sebagai developer, saya ingin menyalin path file dari hasil atau preview, agar saya bisa membukanya di editor favorit.
42. Sebagai developer, saya ingin membuka file asli dengan aplikasi default sistem dari preview, agar saya bisa langsung mengedit.

### F. Filter dan facet (Fase 7)

43. Sebagai developer, saya ingin menulis `rust ownership language:rust` atau `concurrency tag:rust` untuk membatasi hasil, agar saya bisa mempersempit pencarian tanpa meninggalkan keyboard.
44. Sebagai developer, saya ingin filter inline `extension`, `language`, `type`, `project`, dan `tag`, agar saya bisa menyaring dari berbagai sudut pandang.
45. Sebagai developer, saya ingin beberapa filter digabung dalam satu query, agar saya bisa menyaring dengan presisi.
46. Sebagai developer, saya ingin filter dengan nilai tidak dikenal atau salah format diberi tahu dengan jelas, bukan diam-diam diabaikan, agar saya tidak salah menafsirkan hasil kosong.
47. Sebagai developer, saya ingin sidebar facet yang menampilkan jumlah hasil per kategori (misalnya Markdown 42, Rust 21), agar saya melihat sebaran hasil sekilas.
48. Sebagai developer, saya ingin mengklik facet untuk menerapkan atau melepas filter, agar menyaring semudah mengklik.
49. Sebagai developer, saya ingin klik facet dan filter inline saling tersinkron (klik facet menambah token ke query, mengetik token mencentang facet), agar keduanya konsisten.
50. Sebagai developer, saya ingin jumlah facet mencerminkan query yang sedang aktif (semua filter berada pada `bool.filter`, sehingga angka facet menyempit secara konsisten mengikuti filter aktif).

### G. Pencarian toleran dan autocomplete (Fase 7)

51. Sebagai developer, saya ingin `rust ownrship` tetap menemukan "Rust Ownership", agar salah ketik tidak menghentikan saya.
52. Sebagai developer, saya ingin hasil yang cocok persis tetap berperingkat di atas hasil yang hanya cocok secara fuzzy, agar toleransi salah ketik tidak mengacaukan relevansi.
53. Sebagai developer, saya ingin mengetik awal sebuah kata (`owner`) dan tetap menemukan `ownership`, agar saya bisa mencari tanpa mengetik lengkap.
54. Sebagai developer, saya ingin saran autocomplete muncul saat saya mengetik, agar saya bisa memilih judul atau istilah yang ada di knowledge base saya.
55. Sebagai developer, saya ingin autocomplete responsif (terasa instan), agar tidak mengganggu alur mengetik.
56. Sebagai developer, saya ingin autocomplete tidak memicu terlalu banyak request ke backend (debounce), agar sistem tetap ringan.

### H. Pengurutan

57. Sebagai developer, saya ingin mengurutkan hasil berdasarkan relevansi (default), tanggal modifikasi, atau nama, agar saya bisa menemukan dokumen terbaru atau mencari secara alfabetis.
58. Sebagai developer, saya ingin pengurutan tetap berlaku saat berpindah halaman, agar hasil konsisten.

### I. Code search (Fase 8)

59. Sebagai developer, saya ingin file source code ikut ter-index, agar saya bisa mencari fungsi atau implementasi di proyek saya.
60. Sebagai developer, saya ingin `authenticate user` menemukan `authenticate_user` maupun `authenticateUser`, agar saya tidak perlu menebak gaya penamaan.
61. Sebagai developer, saya ingin hasil kode menampilkan potongan kode dengan highlight dan **nomor baris** (dihitung dari index dokumen mentah `_source.content`), agar saya tahu persis letak kecocokan.
62. Sebagai developer, saya ingin bahasa pemrograman terdeteksi otomatis dari ekstensi file, agar saya bisa memfilter `language:rust` atau `language:typescript`.
63. Sebagai developer, saya ingin file konfigurasi (JSON, TOML, YAML) ikut ter-index, agar saya bisa mencari pengaturan yang pernah saya tulis.
64. Sebagai developer, saya ingin memfilter hasil hanya kode atau hanya dokumentasi (`type`), agar kode tidak membanjiri pencarian catatan dan sebaliknya.
65. Sebagai developer, saya ingin potongan kode memakai tampilan monospace dengan indentasi terjaga, agar kode tetap terbaca.
66. Sebagai developer, saya ingin pencarian dalam kode tetap terasa seperti pencarian teks biasa, agar saya tidak perlu mempelajari sintaks khusus.

#### Tabel Pemetaan Ekstensi Kanonikal (Type & Language)

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
| *Diblok Default* | `.env*`, `*.pem`, `*.key`, `id_rsa`, `*.p12` | *(Ditolak demi keamanan rahasia/secret)* |

### J. Statistik dan pengaturan

67. Sebagai developer, saya ingin melihat statistik index (jumlah dokumen, ukuran, sebaran per tipe dan bahasa), agar saya memahami isi knowledge base saya.
68. Sebagai developer, saya ingin mengatur ukuran file maksimum yang di-index, agar saya mengontrol apa yang dianggap terlalu besar.
69. Sebagai developer, saya ingin mengatur daftar folder dan pola yang diabaikan, agar saya menyesuaikannya dengan gaya kerja saya.
70. Sebagai developer, saya ingin mengatur bobot field (judul, tag, isi) untuk peringkat, agar saya bisa bereksperimen dan melihat dampaknya pada urutan hasil.
71. Sebagai developer, saya ingin pengaturan tersimpan permanen dan berlaku setelah restart, agar saya tidak mengatur ulang.

### K. Pembelajar (pemilik proyek)

72. Sebagai pembelajar Rust, saya ingin logika bisnis terpisah dari kerangka HTTP dan dari klien Elasticsearch, agar saya benar-benar berlatih trait, modul, penanganan error, dan async di Rust.
73. Sebagai pembelajar Elasticsearch, saya ingin memetakan setiap milestone (match, multi_match, bool, fuzzy, prefix, highlight, aggregation, analyzer kustom, tuning BM25, autocomplete) ke fitur nyata, agar tiap konsep punya konteks.
74. Sebagai pembelajar information retrieval, saya ingin bisa melihat skor dan membandingkan hasil dengan bobot field berbeda, agar saya memahami cara peringkat bekerja.
75. Sebagai pembelajar, saya ingin Elasticsearch bisa dibangun ulang dari sumber data, agar saya bebas bereksperimen dengan mapping dan analyzer tanpa takut kehilangan data.
76. Sebagai pembelajar, saya ingin modul-modul inti bisa diuji tanpa Elasticsearch, agar siklus umpan balik cepat.
77. Sebagai pembelajar Tauri, saya ingin aplikasi desktop tipis yang hanya menangani UI dan komunikasi HTTP, agar batas tanggung jawab antar komponen jelas.
78. Sebagai pembelajar, saya ingin log terstruktur di backend, agar saya bisa menelusuri apa yang terjadi saat indexing dan pencarian.
79. Sebagai pembelajar, saya ingin dokumentasi singkat tiap keputusan arsitektur, agar saya bisa menjelaskan alasan desain saat menjadikannya portofolio.

---

## Implementation Decisions

### Arsitektur dan batas komponen

- Sistem terdiri dari **dua service terpisah** yang dijalankan sendiri-sendiri: **backend Rust** dan **aplikasi desktop Tauri**. Keduanya berkomunikasi lewat HTTP. Backend tidak ditanam di dalam Tauri dan tidak dijalankan sebagai sidecar.
- **Docker hanya dipakai untuk PostgreSQL dan Elasticsearch** (lewat satu berkas Docker Compose). Backend dan desktop dijalankan langsung di mesin pengembang.
- **Tauri tidak memiliki logika pencarian.** Tauri hanya: UI, state, interaksi pengguna, request HTTP, dan menampilkan hasil. Seluruh API, indexing, parsing, pencarian, validasi, dan logika bisnis ada di backend. Elasticsearch menangani index, pencarian, peringkat, aggregation, dan highlight.
- Backend memakai **arsitektur berlapis/hexagonal**: lapisan API (Axum) → lapisan aplikasi (use case) → domain (aturan dan tipe), dengan **repository sebagai batas** ke Elasticsearch, PostgreSQL, dan sistem file. Backend **bukan** sekadar pembungkus CRUD Elasticsearch; ia memiliki logika domain nyata (parsing query, perencanaan scan, ekstraksi dokumen, pembangunan query).
- Satu paket backend dengan pemisahan modul internal (api, domain, application, infrastructure) untuk tahap awal. Pemecahan menjadi banyak crate ditunda sampai ada kebutuhan nyata.
- Aplikasi adalah **single-user, lokal, tanpa autentikasi**. Backend mengikat ke alamat lokal secara default.

### Pembagian peran penyimpanan

- **PostgreSQL** menyimpan **metadata, state job indexing, dan pengaturan**. Ini sumber kebenaran untuk: folder terdaftar, registry dokumen (path, hash isi, waktu modifikasi, ukuran, status), riwayat dan status job, dan setting.
- **Elasticsearch** menyimpan **konten yang bisa dicari** beserta field turunan untuk peringkat, filter, dan facet. Elasticsearch dianggap **dapat dibuang dan dibangun ulang** dari file di disk dan metadata di PostgreSQL.
- Dokumen **tidak** disalin penuh ke PostgreSQL. File di disk adalah sumber isi.

### Modul yang dibangun

Modul dalam (logika banyak, antarmuka kecil dan stabil, dapat diuji terpisah):

1. **Query Parser** — mengubah teks query mentah menjadi query terstruktur: istilah bebas ditambah filter `key:value` (`language`, `extension`, `type`, `project`, `tag`). Menangani kutipan, karakter khusus, token filter tidak valid (mengembalikan peringatan terstruktur), dan query kosong. Murni, tanpa I/O.
2. **Document Extractor** — mengubah satu file (path + byte) menjadi dokumen terstandar: judul (heading H1 pertama untuk Markdown, nama file untuk lainnya), isi teks, ekstensi, bahasa (dari ekstensi), tipe (dokumentasi, kode, konfigurasi), proyek (nama subfolder tingkat-atas di bawah folder root), tag (dari front matter Markdown bila ada), ukuran, waktu modifikasi. Mendeteksi file biner dan file melebihi batas ukuran. Murni terhadap input yang diberikan.
3. **Scan Planner** — membandingkan hasil penelusuran folder dengan registry di PostgreSQL dan menghasilkan **rencana**: daftar file untuk ditambah, diperbarui (hash berubah), dihapus (file hilang), atau dilewati (tidak berubah, diabaikan, terlalu besar, biner). Memakai hash isi sebagai penentu perubahan, dengan waktu modifikasi sebagai optimasi. Murni terhadap dua input tersebut.
4. **Search Query Builder** — mengubah query terstruktur + parameter (halaman, ukuran, urutan, bobot field) menjadi permintaan pencarian Elasticsearch: multi-match dengan bobot field, komponen fuzzy dan prefix, klausa bool (filter terpisah dari penilaian), highlight, aggregation untuk facet, pengurutan, pagination, dan permintaan saran autocomplete. Murni; mengembalikan struktur permintaan tanpa mengirimnya.
5. **Index Orchestrator** — use case yang menjalankan rencana scan: membuat job, mengekstrak dokumen, melakukan bulk index dengan konkurensi terkendali, memperbarui registry dan progres job, menangani kegagalan per file tanpa menggagalkan job, serta memastikan satu job aktif per folder.
6. **Search Repository** — adaptor Elasticsearch di balik trait: membuat/memperbarui index, mapping dan analyzer, bulk index, hapus, pencarian, aggregation, saran, statistik, dan health.
7. **Metadata Repository** — adaptor PostgreSQL di balik trait: folder, registry dokumen, job, setting.
8. **HTTP API** — lapisan Axum: routing, validasi input, pemetaan error domain ke respons HTTP, serialisasi, CORS untuk klien desktop.
9. **Desktop UI** — Tauri + React + TypeScript + Vite + Tailwind: kotak pencarian, daftar hasil, sidebar facet, preview, manajemen folder, progres job, statistik, pengaturan.

### Desain index Elasticsearch (konseptual)

- Satu index untuk dokumen, dengan **versi/alias** agar pembangunan ulang aman.
- Field utama: id stabil, judul, isi, path absolut dan relatif, ekstensi, bahasa, tipe, proyek, tag, ukuran, waktu modifikasi, waktu indexing.
- Field judul, isi, dan tag dianalisis untuk teks penuh. Field ekstensi, bahasa, tipe, proyek bersifat keyword untuk filter dan aggregation.
- **Analyzer kustom untuk kode** (Fase 8): memecah identifier camelCase dan snake_case menjadi sub-kata sehingga `authenticateUser` dan `authenticate_user` cocok dengan `authenticate user`; mempertahankan token asli agar pencarian persis tetap berfungsi.
- **Subfield untuk prefix/autocomplete** (misalnya n-gram tepi atau tipe sejenis) pada judul dan istilah penting.
- Peringkat memakai **BM25 bawaan** dengan bobot field default: judul tertinggi, tag menengah, isi dasar. Bobot dapat diubah lewat setting.
- Highlight dikonfigurasi untuk menghasilkan beberapa fragmen terbaik per dokumen. **Nomor baris dihitung oleh backend** dengan menemukan lokasi fragmen pada isi dokumen; index tidak dipecah per baris.

### Skema metadata PostgreSQL (konseptual)

- **Folder terdaftar**: identitas, path root, waktu dibuat, waktu scan terakhir, status.
- **Registry dokumen**: identitas yang sama dengan id di Elasticsearch, folder induk, path relatif, hash isi, ukuran, waktu modifikasi, status (ter-index, dilewati beserta alasan, gagal beserta alasan, dikecualikan/EXCLUDED), waktu indexing terakhir.
- **Job indexing**: identitas, folder, jenis (import/re-scan/rebuild), status, penghitung (ditambah, diperbarui, dihapus, dilewati, gagal), waktu mulai dan selesai, ringkasan error.
- **Setting**: pasangan kunci-nilai untuk ukuran file maksimum, pola abaikan, bobot field.
- Migrasi skema dikelola secara berversi.

### Pipeline indexing

1. Pengguna memicu Import atau Re-scan untuk sebuah folder → API membuat job dan segera merespons dengan identitas job.
2. Penelusuran folder menerapkan aturan abaikan (folder umum seperti sistem versi dan dependensi, serta pola setting) dan membatasi tipe file ke Markdown, TXT, source code, dan konfigurasi. **PDF tidak didukung.**
3. Scan Planner menghasilkan rencana terhadap registry. File dengan status `EXCLUDED` otomatis dilewati (*skip*) agar tidak ter-index kembali.
4. Orchestrator menjalankan rencana di latar belakang: ekstraksi, bulk index ke Elasticsearch, pembaruan registry. Dokumen yang file-nya hilang dihapus dari Elasticsearch dan registry. Dokumen yang dihapus individual oleh pengguna via API ditandai sebagai `EXCLUDED` di registry sehingga tidak di-index ulang saat Re-scan.
5. Progres job dapat di-poll oleh klien sampai selesai.
6. Operasi bersifat **idempoten**: menjalankan ulang job yang sama tidak menghasilkan duplikasi. Id dokumen diturunkan secara stabil dari identitas folder dan path relatif via `UUIDv5(FolderId, RelativePath)`. Saat file rename/pindah terdeteksi via pencocokan hash (`ScanPlanner`), sistem menghapus ID lama dan membuat ID baru di registry/Elasticsearch secara atomik; file di `new_path` dibaca ulang dari disk untuk ekstraksi konten/metadata segar guna menjaga prinsip *Zero Content Redundancy* di PostgreSQL.

### Perilaku pencarian

- Query bebas dicari lewat multi-match pada judul, tag, dan isi dengan bobot field.
- Komponen fuzzy memberi toleransi salah ketik dengan bobot lebih rendah daripada kecocokan persis; komponen prefix menangani awalan kata.
- Filter dari token inline dan dari klik facet **disatukan** menjadi klausa filter pada query bool, tidak memengaruhi skor.
- Facet dihitung lewat aggregation pada ekstensi, bahasa, tipe, dan proyek, dan mencerminkan query aktif.
- Pengurutan: relevansi (default), tanggal modifikasi, nama.
- Pagination berbasis offset dan ukuran halaman dengan batas maksimum yang masuk akal.
- Autocomplete memakai endpoint terpisah yang ringan.
- Respons selalu menyertakan total hasil dan waktu proses.

### Kontrak API (tingkat tinggi)

| Metode | Endpoint | Tujuan |
|---|---|---|
| GET | `/api/health` | Status backend, Elasticsearch, PostgreSQL |
| GET | `/api/stats` | Statistik index (jumlah, ukuran, sebaran) |
| GET | `/api/folders` | Daftar folder terdaftar |
| POST | `/api/index/folder` | Daftarkan folder dan mulai Import, atau picu Re-scan; mengembalikan identitas job |
| POST | `/api/index` | Index atau un-exclude satu file/dokumen tunggal (backend-only, pulihkan status EXCLUDED ke INDEXED) |
| GET | `/api/index/jobs/:id` | Status dan progres job |
| POST | `/api/index/jobs/:id/cancel` | Batalkan background job yang sedang berjalan |
| POST | `/api/index/rebuild` | Bangun ulang index Elasticsearch dari seluruh folder (global lock: tolak jika ada job aktif, return 409 pada scan baru) |
| DELETE | `/api/folders/:id` | Hapus folder beserta dokumennya dari index |
| GET | `/api/search` | Pencarian: parameter query, halaman, ukuran, urutan; respons berisi query, total, waktu proses, hasil (id, judul, path, skor, fragmen highlight beserta nomor baris, metadata), dan facet |
| GET | `/api/suggest` | Saran autocomplete |
| GET | `/api/documents/:id` | Isi dan metadata dokumen untuk preview |
| DELETE | `/api/documents/:id` | Hapus satu dokumen dari index (status EXCLUDED di registry) |
| GET / PUT | `/api/settings` | Baca dan ubah pengaturan |

- Respons error memakai format seragam (kode, pesan, rincian) dan kode status HTTP yang sesuai.
- Peringatan non-fatal (misalnya token filter tidak dikenal) dikembalikan di dalam respons pencarian, bukan sebagai error.

### Antarmuka desktop

- Tata letak: kotak pencarian di atas, sidebar facet di kiri, daftar hasil di tengah, panel preview di sisi lain atau sebagai tampilan detail.
- Halaman terpisah untuk manajemen folder dan progres job, statistik, dan pengaturan.
- Pencarian dan autocomplete memakai debounce; status loading, kosong, dan error ditangani eksplisit.
- Alamat backend dapat dikonfigurasi.

### Operasional dan kualitas

- Logging terstruktur dengan tracing di backend; setiap request dan job memiliki konteks yang dapat ditelusuri.
- Error didefinisikan dengan tipe error yang jelas per lapisan dan dipetakan ke respons API di batas HTTP.
- Konfigurasi (alamat database, alamat Elasticsearch, alamat bind) lewat variabel lingkungan.
- Target kinerja (indikatif untuk skala ±5.000 file): pencarian tipikal selesai di bawah 200 ms di sisi backend; Import 5.000 file selesai dalam hitungan menit; Re-scan tanpa perubahan selesai dalam hitungan detik.
- Alat pengembangan: Docker Compose, Clippy, rustfmt, dan runner test (misalnya cargo-nextest); perintah umum dibungkus dalam task runner sederhana.

---

## Testing Decisions

### Apa itu test yang baik di proyek ini

- Test hanya memeriksa **perilaku luar** (input → output atau efek yang teramati lewat antarmuka publik), bukan detail implementasi internal. Refactor yang tidak mengubah perilaku tidak boleh mematahkan test.
- Satu test satu perilaku, dengan nama yang menjelaskan perilaku itu.
- Test harus deterministik: tidak bergantung pada waktu nyata, urutan eksekusi, atau data sisa dari test lain.
- Test integrasi memakai **Elasticsearch dan PostgreSQL asli** (instance terisolasi per suite), bukan mock, karena nilai utamanya ada pada interaksi nyata dengan mesin pencari dan database.

### Modul yang diuji

**Unit test (murni, cepat, tanpa I/O):**

- **Query Parser**: istilah bebas, satu dan banyak filter, kutipan, karakter khusus, filter tidak valid atau tidak dikenal (menghasilkan peringatan), query kosong, huruf besar-kecil, spasi berlebih.
- **Document Extractor**: ekstraksi judul (H1 vs nama file), front matter dan tag, deteksi bahasa dan tipe dari ekstensi, penentuan proyek, deteksi file biner, batas ukuran, encoding yang tidak valid.
- **Scan Planner**: file baru, berubah, tidak berubah, terhapus, berganti nama/pindah, diabaikan, terlalu besar; idempotensi (rencana kedua setelah rencana pertama selesai kosong).
- **Search Query Builder**: bobot field, penggabungan filter dari token dan facet, fuzzy dan prefix, highlight, aggregation, pengurutan, pagination, batas ukuran halaman.

**Test integrasi (Elasticsearch + PostgreSQL asli):**

- **Index Orchestrator**: Import end-to-end, Re-scan inkremental, penghapusan dokumen untuk file hilang, ketahanan terhadap satu file gagal, satu job aktif per folder, idempotensi.
- **Search Repository (Elasticsearch)**: pembuatan index dan mapping, analyzer kode (camelCase/snake_case), peringkat dasar (judul mengalahkan isi), fuzzy, prefix, highlight beserta fragmen, aggregation, hapus, rebuild lewat alias.
- **Metadata Repository (PostgreSQL)**: operasi folder, registry, job, setting, dan migrasi.
- **HTTP API**: kontrak setiap endpoint, validasi input, format error, alur lengkap dari Import hingga Search lewat HTTP.

### Prior art

Proyek ini greenfield sehingga belum ada test pendahulu di basis kode. Konvensi yang diikuti adalah praktik umum ekosistem Rust: unit test berdampingan dengan modul yang diuji, test integrasi terpisah yang memakai container Docker sementara untuk dependensi nyata. Test pertama yang ditulis menjadi acuan gaya bagi test berikutnya.

---

## Out of Scope

- **Pencarian semantik**: embedding, vector search, KNN, hybrid search, reranking (Fase 9).
- **Fase optimasi kinerja yang mendalam** (Fase 10): profiling, tuning skala besar, benchmark formal.
- **File PDF** dan format biner lain (Word, gambar, OCR).
- **File watcher real-time** dan re-scan terjadwal otomatis; pembaruan hanya manual.
- **Parsing AST / tree-sitter**, indeks simbol, go-to-definition, find-references, dan fitur "mini Sourcegraph" lainnya.
- **Dukungan OpenSearch**; hanya Elasticsearch.
- **Bundling installer satu klik** yang memasukkan backend, Elasticsearch, dan PostgreSQL ke dalam paket desktop.
- **Multi-user, autentikasi, otorisasi, dan akses jarak jauh**; sistem satu pengguna dan lokal.
- **Sinkronisasi cloud**, indexing folder jaringan, dan integrasi dengan layanan eksternal.
- **PostgreSQL sebagai penyimpan konten penuh** atau sumber kebenaran isi dokumen.
- **Test otomatis E2E UI desktop** (misal via Playwright); pengujian frontend difokuskan pada unit/integration test (Vitest, RTL, MSW) dengan target cakupan 80% pada stores & helpers sesuai ARCHITECTURE.
- Aplikasi seluler dan versi web.
- Penyuntingan dokumen dari dalam aplikasi; LynxSearch hanya mencari dan menampilkan.

---

## Further Notes

### Pemetaan fase ke cakupan PRD

| Fase | Fokus | Isi utama |
|---|---|---|
| 1 | Project setup | Struktur repo, Docker Compose (PostgreSQL + Elasticsearch), tooling |
| 2 | Rust API | Axum, konfigurasi, error, tracing, health, kerangka lapisan |
| 3 | Elasticsearch | Mapping, analyzer dasar, Search Repository, Metadata Repository, migrasi |
| 4 | Document indexing | Document Extractor, Scan Planner, Index Orchestrator, job dan progres |
| 5 | Basic search | Query Builder dasar, multi-match berbobot, highlight, pagination |
| 6 | Tauri UI | Kotak pencarian, hasil, preview, manajemen folder, progres |
| 7 | Advanced search | Query Parser, filter inline, facet, fuzzy, prefix, autocomplete, sorting, tuning BM25 |
| 8 | Code search | File kode dan konfigurasi, analyzer identifier, snippet dengan nomor baris, filter bahasa dan tipe |

Fase 1–6 menghasilkan aplikasi yang sudah bisa dipakai; Fase 7–8 menjadikannya proyek portofolio yang kuat.

### Pemetaan milestone belajar Elasticsearch

match (F5) → multi_match (F5) → bool must/should/filter (F7) → fuzzy (F7) → prefix (F7) → highlight (F5) → aggregations (F7) → analyzer kustom (F8) → tuning BM25 (F7) → autocomplete (F7).

### Asumsi yang perlu dikonfirmasi

Hal-hal berikut diputuskan sementara oleh penulis PRD karena belum dibahas eksplisit dan sebaiknya dikonfirmasi sebelum implementasi:

1. **Definisi `project` (Dikonfirmasi)**: nama subfolder tingkat-atas di bawah folder root yang didaftarkan. File yang berada tepat di root dipetakan menjadi `project = None` (null).
2. **Definisi `type` (Dikonfirmasi)**: kategori turunan dari ekstensi (`doc` untuk Markdown/TXT, `code` untuk source code, `config` untuk JSON/TOML/YAML).
3. **Definisi `language` vs `tag` (Dikonfirmasi)**: `language:` murni diturunkan dari ekstensi file. Pencarian berdasarkan tag menggunakan filter inline terpisah `tag:`.
4. **Sumber tag (Dikonfirmasi)**: front matter YAML pada file Markdown yang diekstrak secara otomatis.
5. **Batas ukuran file default** dan **daftar folder diabaikan default** (nilai awal diserahkan ke implementasi: default 2 MB dan crate `ignore`, dapat diubah lewat setting).
6. **Bobot field dapat diatur lewat setting** untuk mendukung eksperimen peringkat.
7. **Target kinerja** pada bagian Operasional bersifat indikatif, bukan komitmen kontrak.

### Pertanyaan terbuka

- Apakah folder yang dihapus dari daftar harus meminta konfirmasi di UI?
- Apakah preview perlu mendukung pencarian di dalam dokumen (find-in-page), atau cukup highlight?
- Jenis dan ukuran font/tema (terang/gelap) pada UI.

### Referensi

- Robertson, S. & Zaragoza, H. (2009). *The Probabilistic Relevance Framework: BM25 and Beyond*. Foundations and Trends in Information Retrieval, 3(4). — landasan teori BM25.
- Manning, C. D., Raghavan, P. & Schütze, H. (2008). *Introduction to Information Retrieval*. Cambridge University Press. — inverted index, tokenisasi, peringkat, evaluasi.
- Dokumentasi resmi Elasticsearch (Query DSL, analyzers, aggregations, highlighting, similarity/BM25).
- Dokumentasi resmi Tauri, Axum, dan Tokio.

### Catatan pengiriman

Skill PRD menyarankan PRD dikirim sebagai GitHub issue. Dokumen ini ditulis sebagai Markdown siap tempel ke issue atau ke berkas dokumentasi repo.
