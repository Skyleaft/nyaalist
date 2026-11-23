# Nyaalist - Engineering & Operational Skills Guide (`skills.md`)

This document outlines the core engineering capabilities, architectural patterns, operational conventions, and domain skills required to develop, test, and maintain **Nyaalist**.

---

## 1. System Architecture Overview

Nyaalist is an all-in-one, self-hosted anime torrent manager, automated library organizer, and FFmpeg transcoding suite designed with a dark, telemetry-first technical dashboard.

```
                    ┌───────────────────────────────┐
                    │     Frontend (SvelteKit 2)    │
                    │   Svelte 5 Runes + Tailwind 4 │
                    │    WattVision Technical UI    │
                    └───────┬───────────────▲───────┘
                            │ REST          │ WebSocket
                            │ (JSON)        │ (Events)
                    ┌───────▼───────────────┴───────┐
                    │      Backend (Rust / Axum)    │
                    │    Tokio Async Microservices  │
                    └───────┬───────────────┬───────┘
                            │               │
        ┌───────────────────┼───────────────┼───────────────────┐
        │                   │               │                   │
┌───────▼───────┐   ┌───────▼───────┐   ┌───▼───────────┐   ┌───▼───────────┐
│ PostgreSQL 19 │   │ librqbit 9.1  │   │ FFmpeg Runner │   │ Scraper & API │
│ SeaORM + SQLx │   │ Torrent Engine│   │ Queue Worker  │   │ Nyaa + AniList│
└───────────────┘   └───────────────┘   └───────────────┘   └───────────────┘
```

---

## 2. Core Technical Skills & Domains

### 2.1 Backend Engineering (Rust & Axum)
- **Framework & Runtime:** Axum 0.7 running on Tokio 1.40 async multi-threaded runtime.
- **State Architecture:** Global `AppState` Arc containing:
  - `PgPool` (SQLx connection pool for high-throughput telemetry updates).
  - `DatabaseConnection` (SeaORM 2.0 ORM connection for entity workflows and automated migrations).
  - `TorrentEngine` (Arc wrapper around librqbit session).
  - `WsHub` (WebSocket broadcast hub for live telemetry).
  - `AniListClient` (GraphQL client with rate-limiting and local DB caching).
  - `AppConfig` (Type-safe configuration loaded from environment).
- **Dual Database Pattern:**
  - **SeaORM 2.0:** Handles relational entity mapping (`backend/src/entities/`) and automated database migrations (`backend/src/migration/`) on application startup.
  - **SQLx 0.8:** Handles high-frequency updates, raw SQL batching (`ANY($1)`), and stream processing for download and transcode telemetry.
- **WebSocket Streaming:** Broadcasts real-time events (`torrent_progress`, `encode_progress`, `anilist_metadata_synced`) to connected web clients with zero client polling.

### 2.2 BitTorrent Engine (librqbit)
- Embedded BitTorrent v1 client powered by `librqbit 9.1`.
- Capabilities:
  - Magnet URI parsing, metadata resolution, and DHT peer discovery.
  - Granular bandwidth telemetry: download/upload speeds, active seeders, connected peers, ETA calculation.
  - Pause, resume, and cancellation with optional disk cleanup.
  - Non-blocking telemetry monitor loop pushing 1-second snapshots via WebSockets.

### 2.3 Anime Metadata & Scraping Pipeline
- **Nyaa Scraper (`scraper` crate):**
  - Parses category listings, search queries, uploader catalogs, and pagination directly from HTML.
  - Extracts title, file size, seeders, leechers, magnet URI, and upload timestamp (`data-timestamp`).
  - Supports sorting by date (`s=id`), size, seeders, leechers, and completed count (`asc` / `desc`).
- **Title Parser (`title_parser.rs`):**
  - Regex and heuristic extractor for uploader tags (e.g. `[SubsPlease]`, `[Erai-raws]`), anime title, episode number, resolution (`1080p`, `720p`), video codec (`HEVC`, `x264`), audio, and CRC32 checksums.
- **Local Database Batch Optimization:**
  - Before making remote requests, resolves titles in batch:
    1. Query `title_mappings` cache table (`raw_title_cleaned = ANY($1)`).
    2. Query `anime_metadata` table directly (`LOWER(title_romaji) = ANY($1)` or `LOWER(title_english) = ANY($1)`).
    3. Fuzzy substring query against local database.
    4. Auto-saves local matches to `title_mappings` for future $0\text{ ms}$ lookups.
- **AniList GraphQL Client & Asynchronous Rate Limiting:**
  - Enqueues unmapped titles to an asynchronous background worker channel.
  - Implements sliding window rate limiter (90 requests/min).
  - Handles HTTP 429 Too Many Requests with exponential backoff and dynamic `Retry-After` header inspection.

### 2.4 Video Transcoding & FFmpeg Pipeline
- **FFmpeg Runner (`ffmpeg_runner.rs`):**
  - Asynchronously spawns `ffmpeg` processes with `-progress pipe:1`.
  - Parses real-time frame rate, current time in seconds, bitrate, speed multiplier, and percentage.
  - Broadcasts progress telemetry over WebSockets.
- **Queue Worker (`queue_worker.rs`):**
  - Continuously polls `encoding_jobs` table for `QUEUED` tasks.
  - Automatically loads assigned `encoding_presets` (e.g. `libx265`, `CRF 24`, `medium`, `copy` audio).
  - Upon completion, marks `library_files.is_encoded = true`.
- **Media Library Integration:**
  - Directly queue encode jobs from the Library UI for organized or raw media files with customizable output paths.

### 2.5 Automated Media Library Organizer
- **Directory Scanning (`walkdir`):**
  - Scans download destination folders for completed video containers (`.mkv`, `.mp4`, `.avi`).
  - Automatically parses filename heuristics and links metadata.
- **Template Renaming Engine:**
  - Organizes files into standard Jellyfin/Plex structures:
    ```
    {anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}
    ```
  - Supports custom tokens, formatting, and manual directory moves.

### 2.6 Frontend Architecture (SvelteKit 2 & Svelte 5)
- **Svelte 5 Runes:** Universal reactivity via `$state`, `$derived`, `$props`, and `$effect`.
- **Tailwind CSS v4:** Theme configuration using the "WattVision" design system:
  - Primary Background: `#121212`
  - Card Surface: `#1E1E1E`
  - Accent / Primary Cyan: `#00E5FF`
  - Success / Lime: `#32D74B`
  - Warning / Alert: `#FF453A`
  - Border Lines: `#2C2C2E`
  - Monospace Fonts: JetBrains Mono / Fira Code for telemetry figures.
- **Store Architecture:** Reactive singleton stores in `#lib/stores/`:
  - `auth.svelte.ts`: Session state, login, and setup checks.
  - `torrents.svelte.ts`: Active download jobs and WebSocket speed telemetry.
  - `library.svelte.ts`: Tracked files, organize actions, scan triggers, encode actions.
  - `encoder.svelte.ts`: Encoding presets, job queues, live transcode telemetry.

---

## 3. Database & Migration Conventions

Nyaalist uses **SeaORM 2.0** migrations executed automatically on startup:

```
backend/src/migration/
├── mod.rs                                   # Migrator registration
├── m20261123_000001_initial_schema.rs       # Base tables, extensions, indexes
└── m20261123_000002_drop_download_jobs_fk.rs# Constraint adjustments
```

### Key Schema Tables:
1. `users`: Credentials, Argon2 password hashes, roles.
2. `app_config`: Key-value application runtime configurations.
3. `anime_metadata`: AniList enriched records (titles, scores, episodes, cover image paths).
4. `title_mappings`: Cache linking raw release title heuristics to AniList IDs.
5. `nyaa_torrents`: Scraped release records (optional cache).
6. `download_jobs`: Active and completed downloads with librqbit info hashes.
7. `library_files`: Local video files, Plex paths, moved status, encoded status.
8. `encoding_presets`: FFmpeg configurations (codec, CRF, speed preset, scale, audio).
9. `encoding_jobs`: Background transcode queue tasks and telemetry.

### Adding New Migrations:
1. Create `backend/src/migration/mYYYYMMDD_xxxxxx_name.rs` implementing `MigrationTrait`.
2. Register the struct in `Migrator::migrations()` in `backend/src/migration/mod.rs`.
3. Migrations will run automatically when the backend boots.

---

## 4. Development & Operational Commands

### Backend (Rust)
```bash
# Verify backend compilation
cargo check --manifest-path backend/Cargo.toml

# Run unit tests
cargo test --manifest-path backend/Cargo.toml

# Run backend development server (default port: 8080 or 5041)
cargo run --manifest-path backend/Cargo.toml
```

### Frontend (SvelteKit)
```bash
# Install dependencies
pnpm --dir frontend install

# Start Vite development server (default port: 3000)
pnpm --dir frontend run dev

# Run TypeScript and Svelte check
pnpm --dir frontend run check

# Build production bundle
pnpm --dir frontend run build
```

### Docker Compose
```bash
# Start full stack (PostgreSQL 19, Backend, Frontend)
docker compose up --build -d

# View live logs
docker compose logs -f backend
```

---

## 5. Engineering Best Practices & Rules

1. **Windows Path Quoting:** When running shell commands touching paths with parentheses (e.g. `frontend/src/routes/(app)/library/+page.svelte`), always quote the path so PowerShell does not interpret `(app)` as a subexpression.
2. **Git Sandbox Guidelines on Windows:** Run Git operations (`git add`, `git commit`, `git push`, `gh pr create`) with elevated permissions (`BypassSandbox: true`) to avoid NT directory locking conflicts (`0xC0000022`).
3. **N+1 Prevention:** Always batch database lookups using array parameters (`= ANY($1)`) rather than running single-item queries in iteration loops.
4. **Resilient Downloader Inserts:** Ensure torrent additions never crash the whole request when external foreign keys are missing; always provide defensive database fallbacks.
5. **No Polling Over HTTP:** High-frequency metrics (speeds, frames, sync status) must always be delivered through WebSockets.
