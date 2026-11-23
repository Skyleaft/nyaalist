# Nyaalist - System Architecture & Implementation Plan (`plan.md`)

## 1. Executive Summary & Vision

**Nyaalist** is a self-hosted anime torrent explorer, downloader, library organizer, and video transcoding pipeline built for anime enthusiasts. It bridges raw torrent releases from [nyaa.si](https://nyaa.si) with high-fidelity anime metadata from [AniList](https://anilist.co), automates downloads via BitTorrent magnet links with live WebSocket telemetry, organizes downloaded media files into clean library directory structures, and provides an integrated FFmpeg-based transcoding pipeline for video compression.

### Technology Stack Overview
- **Backend:** **Rust** (Async Tokio runtime, Axum web framework, `librqbit` BitTorrent engine, `sqlx`, and Tokio async subprocess for FFmpeg).
- **Architecture:** **Vertical Slice Architecture (VSA)** — feature-driven module partitioning instead of traditional horizontal layered folders.
- **Database:** **PostgreSQL** (ACID compliance, JSONB configuration storage, full-text search indexing).
- **Frontend Panel:** **SvelteKit** (latest Svelte 5 with Runes: `$state`, `$derived`, `$effect`, and Tailwind CSS).
- **Design System:** Adapted from the **WattVision Technical Design System** (`design-md-sistema-de-monitoreo-el-ctrico-wattvision-DESIGN.md`), featuring high-contrast dark surfaces (`#121212`, `#1E1E1E`), Neon Cyan (`#00E5FF`) telemetry accents, Lime Green (`#32D74B`) live indicators, and JetBrains Mono monospace typography.
- **Deployment:** Multi-container **Docker Compose** orchestration with hardware-accelerated/CPU FFmpeg support.

---

## 2. System Architecture Overview

```mermaid
graph TD
    subgraph Frontend ["SvelteKit 5 Web Panel (WattVision Dark Theme)"]
        UI_Dash["Dashboard / KPIs"]
        UI_Explore["Nyaa Explorer & AniList Sync"]
        UI_Downloader["Torrent Monitor (WS)"]
        UI_Library["Library File Manager"]
        UI_Encoder["Video Transcoder (WS)"]
        UI_Config["Configurator & Auth"]
    end

    subgraph Backend ["Rust Backend (Vertical Slice Architecture)"]
        Router["Axum HTTP & WebSocket Router"]
        
        subgraph VSA ["Vertical Slices"]
            Slice_Auth["features/auth"]
            Slice_Explore["features/explore"]
            Slice_Downloader["features/downloader"]
            Slice_Library["features/library"]
            Slice_Encoder["features/encoder"]
            Slice_Config["features/config"]
        end

        subgraph Core ["Core Shared Infrastructure"]
            State["AppState"]
            DB_Pool["PgPool (sqlx)"]
            WSHub["WebSocket Event Hub"]
        end
    end

    subgraph External ["External Services & Tools"]
        Nyaa["nyaa.si (Scraper / RSS)"]
        AniList["AniList GraphQL API"]
        BT_Swarm["BitTorrent Swarm"]
        FFmpeg_Bin["FFmpeg Engine"]
    end

    subgraph Storage ["Persistent Storage"]
        Postgres[(PostgreSQL 16)]
        CoversDir[("Local Covers Cache")]
        DownloadsDir[("Downloads Volume")]
        LibraryDir[("Library Volume")]
    end

    UI_Explore <--> Router
    UI_Downloader <-->|WebSocket Stream| Router
    UI_Encoder <-->|WebSocket Stream| Router
    UI_Library <--> Router

    Router --> VSA
    VSA --> Core
    Core --> Postgres

    Slice_Explore --> Nyaa
    Slice_Explore --> AniList
    Slice_Explore --> CoversDir

    Slice_Downloader --> BT_Swarm
    Slice_Downloader --> DownloadsDir

    Slice_Library --> DownloadsDir
    Slice_Library --> LibraryDir

    Slice_Encoder --> FFmpeg_Bin
    FFmpeg_Bin --> DownloadsDir
    FFmpeg_Bin --> LibraryDir
```

---

## 3. Design System & UI Specifications

The UI design is adapted from `design-md-sistema-de-monitoreo-el-ctrico-wattvision-DESIGN.md`, prioritizing **technical reliability, mission-control clarity, and high data density**.

### 3.1 Color Palette Tokens

| Token Name | Hex Code | Role & Usage |
| :--- | :--- | :--- |
| **Background Primary** | `#121212` | Main page background (avoids OLED pure black eye fatigue). |
| **Surface Card** | `#1E1E1E` | Card containers, modal sheets, dropdown menus. |
| **Surface Elevated / Hover** | `#252525` | Table row hover, active list item selection, pill toggle backgrounds. |
| **Border Subtle** | `#2C2C2E` | Card borders (`1px solid`), table row dividers, input field borders. |
| **Accent Primary (Data/Telemetry)** | `#00E5FF` | Neon Cyan: download speeds, active progress bars, primary buttons, KPIs. |
| **Accent Secondary (Live/Normal)** | `#32D74B` | Lime Green: live WebSocket indicator, completed downloads, healthy seeds. |
| **Accent Alert (Warning/Error)** | `#FF453A` | Vivid Red: torrent failures, low disk space, encoding errors, delete buttons. |
| **Text Primary** | `#FFFFFF` | Headers, primary titles, critical metrics (high contrast). |
| **Text Secondary** | `#98989D` | Subtitles, release metadata, labels, table headers. |
| **Alert Container** | `#3A1C1C` | Background for error banners with a `4px solid #FF453A` left border. |

### 3.2 Typography Rules

- **Header / UI Text:** `Inter`, Semi-Bold (600) / Bold (700). Clean, modern, international character support.
- **Data & Telemetry Metrics:** `JetBrains Mono` or `Fira Code` (monospace) for:
  - Bandwidth speeds: `14.8 MB/s`
  - Progress percentages: `84.2%`
  - Release hashes & CRC32: `[4C1D82D9]`
  - Torrent info-hashes: `e2a4...7b91`
  - Video resolutions & frame counters: `1920x1080 @ 23.976 fps`
  - Time elapsed / ETA: `00:14:22`
- **Body Text:** `Inter Regular`, 14px, line-height 1.5.

### 3.3 Component Styling & Layout System

- **Layout Grid:** 12-column responsive grid with `8px` modular spacing steps (`8px`, `16px`, `24px`, `32px`).
- **Cards:**
  ```css
  background-color: #1E1E1E;
  border-radius: 16px;
  border: 1px solid #2C2C2E;
  padding: 20px;
  ```
- **Tables:**
  - Table header: uppercase, 12px, tracking-wider, color `#98989D`.
  - Rows: `border-bottom: 1px solid #2C2C2E`.
  - Row Hover: transition background to `#252525`.
- **Progress Bars:**
  - Track: `#2C2C2E`, height `8px`, `border-radius: 4px`.
  - Fill: Gradient from `#00E5FF` to `#32D74B` with animated glow when active.
- **Badges:**
  - Release Group: `#2C2C2E` background, `#00E5FF` text, monospace font.
  - Quality tag (`1080p`, `HEVC`): `#1E1E1E` background, border `1px solid #2C2C2E`, `#FFFFFF` text.
  - Episode tag: `#00E5FF` outline, bold monospace.

---

## 4. Database Schema (PostgreSQL DDL)

The database handles user credentials, application configurations, scraped torrent items, cached AniList anime entries, title resolution mappings, download jobs, library index, and FFmpeg encoding jobs.

```sql
-- Extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. Users Table (Authentication)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'admin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Dynamic Application Configuration Store
CREATE TABLE app_config (
    key VARCHAR(64) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed Initial Default Configuration
INSERT INTO app_config (key, value) VALUES 
('paths', '{"download_dir": "/data/downloads", "library_dir": "/data/library", "covers_dir": "/data/covers"}'),
('downloader', '{"max_active_downloads": 5, "max_download_speed_kb": 0, "max_upload_speed_kb": 0, "auto_move_on_complete": false}'),
('nyaa', '{"base_url": "https://nyaa.si", "default_category": "1_2", "auto_sync_interval_mins": 30}'),
('naming', '{"template": "{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}]"}');

-- 3. AniList Anime Metadata Cache
CREATE TABLE anime_metadata (
    id INT PRIMARY KEY, -- AniList ID
    mal_id INT,
    title_romaji TEXT NOT NULL,
    title_english TEXT,
    title_native TEXT,
    synonyms TEXT[] DEFAULT '{}',
    format VARCHAR(32), -- TV, MOVIE, OVA, ONA, SPECIAL
    status VARCHAR(32), -- FINISHED, RELEASING, NOT_YET_RELEASED
    episodes INT,
    season VARCHAR(16), -- WINTER, SPRING, SUMMER, FALL
    season_year INT,
    genres TEXT[] DEFAULT '{}',
    average_score INT,
    description TEXT,
    cover_image_url TEXT,
    cover_image_local_path TEXT,
    banner_image_url TEXT,
    last_synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_anime_metadata_title_romaji ON anime_metadata (LOWER(title_romaji));
CREATE INDEX idx_anime_metadata_title_english ON anime_metadata (LOWER(title_english));

-- 4. Cleaned Title Mapping Cache (Minimizes AniList API requests)
CREATE TABLE title_mappings (
    raw_title_cleaned TEXT PRIMARY KEY,
    anilist_id INT REFERENCES anime_metadata(id) ON DELETE SET NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Nyaa Torrents (Scraped Releases)
CREATE TABLE nyaa_torrents (
    id BIGINT PRIMARY KEY, -- Nyaa View ID (e.g., 2189488)
    title TEXT NOT NULL,
    uploader VARCHAR(128),
    parsed_anime_title TEXT,
    parsed_episode VARCHAR(32),
    parsed_resolution VARCHAR(32),
    parsed_codec VARCHAR(32),
    parsed_audio VARCHAR(32),
    parsed_hash VARCHAR(32), -- CRC32 Checksum
    magnet_uri TEXT NOT NULL,
    torrent_file_url TEXT,
    category VARCHAR(64) NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    seeders INT NOT NULL DEFAULT 0,
    leechers INT NOT NULL DEFAULT 0,
    completed_count INT NOT NULL DEFAULT 0,
    nyaa_created_at TIMESTAMPTZ NOT NULL,
    anilist_id INT REFERENCES anime_metadata(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_nyaa_torrents_uploader ON nyaa_torrents (uploader);
CREATE INDEX idx_nyaa_torrents_anilist_id ON nyaa_torrents (anilist_id);
CREATE INDEX idx_nyaa_torrents_created ON nyaa_torrents (nyaa_created_at DESC);

-- 6. Active and Completed Download Jobs
CREATE TABLE download_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    nyaa_torrent_id BIGINT REFERENCES nyaa_torrents(id) ON DELETE SET NULL,
    info_hash VARCHAR(64) UNIQUE NOT NULL,
    name TEXT NOT NULL,
    magnet_uri TEXT NOT NULL,
    save_path TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'QUEUED', -- QUEUED, DOWNLOADING, PAUSED, COMPLETED, ERROR
    total_bytes BIGINT NOT NULL DEFAULT 0,
    downloaded_bytes BIGINT NOT NULL DEFAULT 0,
    uploaded_bytes BIGINT NOT NULL DEFAULT 0,
    download_speed BIGINT NOT NULL DEFAULT 0, -- Bytes/sec
    upload_speed BIGINT NOT NULL DEFAULT 0,   -- Bytes/sec
    progress_pct REAL NOT NULL DEFAULT 0.0,
    eta_seconds BIGINT,
    seeders INT NOT NULL DEFAULT 0,
    peers INT NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 7. Managed Library Files
CREATE TABLE library_files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    download_job_id UUID REFERENCES download_jobs(id) ON DELETE SET NULL,
    anilist_id INT REFERENCES anime_metadata(id) ON DELETE SET NULL,
    original_filename TEXT NOT NULL,
    current_path TEXT NOT NULL,
    target_library_path TEXT,
    episode VARCHAR(32),
    season INT DEFAULT 1,
    resolution VARCHAR(32),
    file_size_bytes BIGINT NOT NULL,
    is_moved BOOLEAN NOT NULL DEFAULT FALSE,
    is_encoded BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 8. FFmpeg Video Encoding Presets
CREATE TABLE encoding_presets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) UNIQUE NOT NULL,
    description TEXT,
    video_codec VARCHAR(32) NOT NULL DEFAULT 'libx265',
    preset VARCHAR(32) NOT NULL DEFAULT 'medium',
    crf INT NOT NULL DEFAULT 24,
    target_resolution VARCHAR(32), -- e.g. '1280x720', NULL for source resolution
    audio_codec VARCHAR(32) NOT NULL DEFAULT 'copy',
    audio_bitrate VARCHAR(32) DEFAULT '160k',
    container VARCHAR(16) NOT NULL DEFAULT 'mkv',
    custom_ffmpeg_args TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO encoding_presets (name, description, video_codec, preset, crf, target_resolution, audio_codec, container, is_default) VALUES
('720p x265 Balanced', 'Compress to 720p with HEVC CRF 24 medium preset', 'libx265', 'medium', 24, '1280x720', 'copy', 'mkv', TRUE),
('1080p x265 High Quality', 'Re-encode to 1080p HEVC CRF 21 for storage efficiency', 'libx265', 'medium', 21, '1920x1080', 'copy', 'mkv', FALSE);

-- 9. FFmpeg Encoding Queue
CREATE TABLE encoding_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_file_path TEXT NOT NULL,
    target_file_path TEXT NOT NULL,
    preset_id UUID NOT NULL REFERENCES encoding_presets(id),
    status VARCHAR(32) NOT NULL DEFAULT 'QUEUED', -- QUEUED, ENCODING, COMPLETED, FAILED, CANCELLED
    progress_pct REAL NOT NULL DEFAULT 0.0,
    current_frame BIGINT NOT NULL DEFAULT 0,
    total_frames BIGINT,
    fps REAL NOT NULL DEFAULT 0.0,
    current_time_sec REAL NOT NULL DEFAULT 0.0,
    total_duration_sec REAL NOT NULL DEFAULT 0.0,
    speed REAL NOT NULL DEFAULT 0.0,
    source_size_bytes BIGINT NOT NULL DEFAULT 0,
    encoded_size_bytes BIGINT NOT NULL DEFAULT 0,
    eta_seconds BIGINT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);
```

---

## 5. Backend Vertical Slice Architecture (Rust)

In standard layered architectures, a single feature modification spans controllers, services, repositories, and models. In **Vertical Slice Architecture**, each feature is encapsulated within its own isolated slice containing its endpoints, business rules, queries, and background tasks.

### 5.1 Project Directory Structure

```
backend/
├── Cargo.toml
├── .env.example                     # Environment configuration template
├── migrations/
│   └── 20261123_001_init.sql
├── src/
│   ├── main.rs                      # Axum router setup, DB connection, server bind
│   ├── core/                        # Cross-cutting foundational modules
│   │   ├── mod.rs
│   │   ├── config.rs                # Typed config loader using dotenvy & env vars
│   │   ├── db.rs                    # PgPool builder and migration runner
│   │   ├── error.rs                 # Domain & Axum HTTP AppError
│   │   ├── state.rs                 # AppState holding DbPool, WsBroadcaster, Engines
│   │   └── ws.rs                    # Centralized WebSocket broadcast bus
│   └── features/                    # Vertical Slices
│       ├── auth/                    # Slice: Authentication & First-Run Setup
│       │   ├── mod.rs
│       │   ├── routes.rs
│       │   ├── setup_admin.rs       # Check if users exist; if not, permit admin creation
│       │   ├── login.rs             # Verify password via Argon2id, issue JWT
│       │   └── middleware.rs        # Axum request extractor for AuthUser
│       ├── explore/                 # Slice: Scraper, Title Parser & AniList Sync
│       │   ├── mod.rs
│       │   ├── routes.rs            # GET /api/explore/latest, GET /api/explore/user/:name
│       │   ├── nyaa_scraper.rs      # HTML table scraping & RSS feed reader
│       │   ├── title_parser.rs      # Anitomy-style heuristics for anime title/ep/tags
│       │   ├── anilist_client.rs    # GraphQL client with token-bucket rate limiter
│       │   └── sync_service.rs      # Coordination: Nyaa item -> Parse -> AniList -> DB
│       ├── downloader/              # Slice: Torrent Client Engine & Telemetry
│       │   ├── mod.rs
│       │   ├── routes.rs            # POST /add, POST /pause, DELETE /cancel
│       │   ├── torrent_engine.rs    # librqbit session manager
│       │   ├── torrent_stats.rs     # Speed, ETA, piece calculation
│       │   └── ws_handler.rs        # WebSocket stream broadcasting torrent telemetry
│       ├── library/                 # Slice: File Management & Organizing
│       │   ├── mod.rs
│       │   ├── routes.rs            # GET /files, POST /move, POST /organize
│       │   ├── file_scanner.rs      # Recursive disk scanner
│       │   ├── mover.rs             # Atomic cross-device file move / copy
│       │   └── template_renamer.rs  # Expression formatter for anime media files
│       ├── encoder/                 # Slice: Video Transcoder & Queue
│       │   ├── mod.rs
│       │   ├── routes.rs            # GET /presets, POST /jobs, DELETE /jobs/:id
│       │   ├── ffmpeg_runner.rs     # Tokio Command spawn with `-progress pipe:1`
│       │   ├── queue_worker.rs      # Sequential/throttled background worker
│       │   └── ws_handler.rs        # WebSocket stream broadcasting encoding stats
│       └── configurator/            # Slice: System Paths & Defaults
│           ├── mod.rs
│           ├── routes.rs            # GET /settings, PUT /settings
│           └── path_validator.rs    # Validates disk permissions for downloads/library
```

---

### 5.2 Deep-Dive: Slice 1 - Authentication & First-Run Setup (`features/auth`)

#### Business Logic:
1. When the backend boots, it checks `SELECT COUNT(*) FROM users`.
2. If `count == 0`, the backend enters `SETUP_REQUIRED` mode.
3. The frontend displays the setup wizard, prompting for initial administrator credentials.
4. Once the first user is created via `POST /api/auth/setup`, the setup route is locked permanently.
5. All sensitive endpoints enforce the `AuthUser` Axum extractor verifying the JWT token stored in an `HttpOnly` secure cookie.

#### API Endpoints:
- `GET /api/auth/status` -> `{ "setup_required": boolean, "authenticated": boolean, "username": string | null }`
- `POST /api/auth/setup` -> Body: `{ "username": "admin", "password": "..." }`
- `POST /api/auth/login` -> Body: `{ "username": "admin", "password": "..." }`
- `POST /api/auth/logout` -> Clears auth cookie.

---

### 5.3 Deep-Dive: Slice 2 - Explore & AniList Sync (`features/explore`)

#### 1. Nyaa Scraping Engine (`nyaa_scraper.rs`)
Supports two operational modes:
- **Category Scraping:** Scrapes `https://nyaa.si/?c=1_2` (Anime - English-translated) or other anime subcategories, extracting the torrent title, view ID, magnet URI, size, seeders, leechers, and date.
- **User Scraping:** Scrapes user-specific uploads, e.g., `https://nyaa.si/user/Erai-raws` or `https://nyaa.si/user/SubsPlease`.
- **Parsing Mechanism:** Uses `scraper` (HTML5 selector parsing via CSS selectors like `tr.default, tr.success, tr.danger`) and fallback to Nyaa RSS feeds (`?page=rss`) for ultra-low latency updates.

#### 2. Filename Parsing Heuristic Engine (`title_parser.rs`)
Extracts structured anime tokens from filenames:
- Target Example 1: `[Erai-raws] Dragon Ball Super: Beerus - 07 [1080p CR WEBRip HEVC AAC][MultiSub][4C1D82D9]`
- Target Example 2: `[Kozame] Odekake Kozame (Little Shark's Outings) - 93 [1080p] [CC2F9599]`

```rust
pub struct ParsedRelease {
    pub uploader: Option<String>,      // "Erai-raws" / "Kozame"
    pub anime_title: String,           // "Dragon Ball Super: Beerus" / "Odekake Kozame (Little Shark's Outings)"
    pub episode: Option<String>,       // "07" / "93"
    pub resolution: Option<String>,    // "1080p"
    pub codec: Option<String>,         // "HEVC"
    pub audio: Option<String>,         // "AAC"
    pub source: Option<String>,        // "CR WEBRip"
    pub crc32: Option<String>,         // "4C1D82D9" / "CC2F9599"
}
```

**Heuristic Steps:**
1. **Uploader Extraction:** Match leading square brackets `^\[([^\]]+)\]`.
2. **Tag & Hash Stripping:** Extract trailing bracket groups containing known patterns:
   - Resolution: `480p|720p|1080p|2160p|4K`
   - Codecs: `x264|x265|HEVC|AVC|AV1|10bit`
   - Audio: `AAC|FLAC|AC3|EAC3|Opus|Dual-Audio`
   - CRC32 Checksums: `^[0-9A-Fa-f]{8}$`
3. **Episode Boundary Detection:** Look for episode delimiters: ` - (\d+(\.\d+)?)` or ` EP?(\d+)` or ` S\d+E\d+`.
4. **Clean Anime Title:** Normalize the remaining string by trimming extra whitespace and stripping stray symbols.

#### 3. AniList GraphQL Sync Engine (`anilist_client.rs`)
- Query AniList GraphQL API (`https://graphql.anilist.co`):
```graphql
query ($search: String) {
  Media(search: $search, type: ANIME, sort: SEARCH_MATCH) {
    id
    idMal
    title {
      romaji
      english
      native
    }
    format
    status
    episodes
    season
    seasonYear
    genres
    averageScore
    description
    coverImage {
      extraLarge
      large
    }
    bannerImage
  }
}
```
- **Local Database Caching:**
  - Saves the cleaned search string in `title_mappings` to prevent redundant AniList API queries.
  - Upserts complete record in `anime_metadata`.
  - Asynchronously downloads the cover image to `/data/covers/{anilist_id}.jpg` and serves it locally at `/api/covers/{id}.jpg`.
  - Implements a 90 requests/minute rate-limiting token bucket to respect AniList API limits.

---

### 5.4 Deep-Dive: Slice 3 - Downloader Engine (`features/downloader`)

#### Engine Integration:
- Uses `librqbit`, a high-performance asynchronous BitTorrent client written entirely in pure Rust.
- Manages magnet URIs, DHT peer discovery, tracker announces, piece verification, and file writing to the configured download folder (`/data/downloads`).
- Background Tokio task queries active torrent stats every 1000ms:
  - Down/Up speed (bytes/sec)
  - Downloaded / Total bytes
  - Completion percentage (`progress_pct`)
  - Connected seeders & peers
  - Dynamic ETA calculation: `(total_bytes - downloaded_bytes) / current_down_speed`
- Telemetry is stored in `download_jobs` and broadcast over the Downloader WebSocket channel.

#### API Endpoints:
- `GET /api/downloader/jobs` -> Returns active and finished torrents.
- `POST /api/downloader/add` -> Body: `{ "magnet_uri": "...", "save_path": "...", "auto_move": false }`
- `POST /api/downloader/:id/pause` -> Pause active torrent.
- `POST /api/downloader/:id/resume` -> Resume paused torrent.
- `DELETE /api/downloader/:id?delete_data=true` -> Removes torrent and optionally deletes downloaded files.

---

### 5.5 Deep-Dive: Slice 4 - Library Management (`features/library`)

#### Functionality:
- Scans completed download paths and organized library directories.
- Allows users to organize raw downloads into their structured media library (e.g., for Plex/Jellyfin/Kodi).
- **Template Renaming Engine:**
  - Evaluates patterns like `{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}`.
  - Automatically fetches metadata from `anime_metadata` using the linked `anilist_id`.
- Performs atomic moves or cross-filesystem file copies with progress reporting.

#### API Endpoints:
- `GET /api/library/files` -> Lists media files in download and destination paths.
- `POST /api/library/move` -> Body: `{ "source_path": "...", "target_path": "...", "new_filename": "..." }`
- `POST /api/library/auto-organize` -> Body: `{ "download_job_id": "...", "template": "..." }`

---

### 5.6 Deep-Dive: Slice 5 - Anime Video Encoder (`features/encoder`)

#### FFmpeg Pipeline & Progress Streaming:
- Uses `tokio::process::Command` to invoke the `ffmpeg` binary.
- Configures FFmpeg to output machine-readable telemetry using `-progress pipe:1` and `-nostats`.

```bash
ffmpeg -i "/data/downloads/input.mkv" \
  -c:v libx265 -crf 24 -preset medium \
  -vf "scale=1280:720" \
  -c:a copy -c:s copy \
  -progress pipe:1 -nostats \
  "/data/library/output_720p.mkv"
```

#### FFmpeg Output Parser:
The runner reads stdout line-by-line and extracts:
- `frame=1420`
- `fps=48.2`
- `out_time_us=59200000` (converts to seconds: 59.2s)
- `speed=2.01x`
- Progress calculation: `current_time / total_video_duration * 100`

#### Telemetry Streaming:
Updates the `encoding_jobs` database row and immediately pushes an `encode_progress` packet to all connected clients on the Encoder WebSocket channel.

---

### 5.7 Deep-Dive: Slice 6 - Configurator (`features/config`)

- Stores and retrieves global settings from the `app_config` PostgreSQL table.
- **Path Verification:** Validates that `/data/downloads` and `/data/library` exist, are directories, and have write permissions before saving.
- Configures default torrent limits (max active torrents, max bandwidth limits) and default encoding presets.

---

### 5.8 Backend Environment Configuration Specification (`.env`)

The backend configuration relies on 12-factor application standards, parsing environment variables loaded from a `.env` file via `dotenvy` at startup, with strongly typed defaults and fallback values handled in `core/config.rs`.

#### 1. Complete `.env` / `.env.example` Specification

```bash
# ==============================================================================
# Nyaalist Backend Configuration (.env)
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. Server & Network Configuration
# ------------------------------------------------------------------------------
# Host interface to bind (0.0.0.0 for LAN/Docker access, 127.0.0.1 for local dev)
HOST=0.0.0.0
# Port for the Axum HTTP & WebSocket server
PORT=8080
# Environment mode: 'development' | 'production'
APP_ENV=development
# Logging filter: trace | debug | info | warn | error
RUST_LOG=info,nyaalist=debug,tower_http=info
# Allowed CORS origins for frontend client (comma-separated)
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000

# ------------------------------------------------------------------------------
# 2. Database Configuration (PostgreSQL)
# ------------------------------------------------------------------------------
# PostgreSQL connection URI: postgres://<user>:<password>@<host>:<port>/<dbname>
DATABASE_URL=postgres://nyaa_user:nyaa_secure_password@localhost:5432/nyaalist
# Connection pool limits & timeouts
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
DATABASE_TIMEOUT_SECONDS=30

# ------------------------------------------------------------------------------
# 3. Security & Authentication
# ------------------------------------------------------------------------------
# Secret key used to sign and verify JWT tokens (min 32 characters)
JWT_SECRET=nyaalist_super_secret_jwt_random_key_change_me_in_production_min_32_bytes
# JWT token validity period in hours
JWT_EXPIRATION_HOURS=72
# Secure cookie flag (Set to 'true' in production behind HTTPS reverse proxy)
COOKIE_SECURE=false
# Session cookie identifier
COOKIE_NAME=nyaalist_session

# ------------------------------------------------------------------------------
# 4. Storage & Media Directory Paths
# ------------------------------------------------------------------------------
# Base data root directory
DATA_DIR=./data
# Target directory where active and finished torrent downloads land
DOWNLOADS_DIR=./data/downloads
# Destination directory where organized anime library files are placed
LIBRARY_DIR=./data/library
# Local cache path for AniList cover poster images
COVERS_DIR=./data/covers
# Temporary scratch folder used during FFmpeg video re-encoding
TEMP_ENCODE_DIR=./data/temp_encode

# ------------------------------------------------------------------------------
# 5. Nyaa.si Scraping & Feed Configuration
# ------------------------------------------------------------------------------
# Nyaa base URL (allows proxying or alternative mirror instances)
NYAA_BASE_URL=https://nyaa.si
# Default category to scrape (1_2 = Anime - English-translated, 1_0 = All Anime)
NYAA_DEFAULT_CATEGORY=1_2
# Background polling frequency in minutes for RSS updates
NYAA_AUTO_SYNC_INTERVAL_MINS=30
# Custom HTTP User-Agent header sent during scraping requests
NYAA_USER_AGENT=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Nyaalist/1.0
# Timeout for scraping requests in seconds
NYAA_REQUEST_TIMEOUT_SECONDS=15

# ------------------------------------------------------------------------------
# 6. AniList GraphQL API Integration
# ------------------------------------------------------------------------------
# AniList GraphQL endpoint URL
ANILIST_GRAPHQL_URL=https://graphql.anilist.co
# Token-bucket rate limit (AniList limit is 90 requests/minute)
ANILIST_RATE_LIMIT_PER_MINUTE=90
# Request timeout in seconds
ANILIST_REQUEST_TIMEOUT_SECONDS=10

# ------------------------------------------------------------------------------
# 7. BitTorrent Downloader Engine (librqbit)
# ------------------------------------------------------------------------------
# Incoming TCP/UDP port for BitTorrent peer swarm connections
TORRENT_LISTEN_PORT=4240
# Enable Distributed Hash Table (DHT) for trackerless magnet resolution
TORRENT_ENABLE_DHT=true
# Maximum number of concurrently active torrent downloads
TORRENT_MAX_ACTIVE_DOWNLOADS=5
# Maximum peers connected per individual torrent
TORRENT_MAX_PEERS_PER_TORRENT=50
# Bandwidth rate limits in KB/s (0 = unlimited)
TORRENT_GLOBAL_DOWNLOAD_LIMIT_KBPS=0
TORRENT_GLOBAL_UPLOAD_LIMIT_KBPS=0
# Peer inactivity timeout in seconds
TORRENT_PEER_TIMEOUT_SECONDS=120

# ------------------------------------------------------------------------------
# 8. Anime Video Transcoder (FFmpeg)
# ------------------------------------------------------------------------------
# Binary path or command for FFmpeg executable
FFMPEG_BINARY_PATH=ffmpeg
# Binary path or command for FFprobe executable
FFPROBE_BINARY_PATH=ffprobe
# Maximum number of concurrent FFmpeg encoding jobs (default: 1 for CPU safety)
ENCODER_MAX_CONCURRENT_JOBS=1
# Default preset name applied when none is specified
ENCODER_DEFAULT_PRESET=720p x265 Balanced
```

#### 2. Environment Variables Reference Table

| Variable | Type | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `HOST` | String | `0.0.0.0` | IP interface for Axum web server to bind to. |
| `PORT` | u16 | `8080` | HTTP and WebSocket listening port. |
| `APP_ENV` | String | `development` | Deployment environment mode (`development`, `production`). |
| `RUST_LOG` | String | `info,nyaalist=debug` | Tracing log level filter for system components. |
| `CORS_ALLOWED_ORIGINS` | String | `http://localhost:3000` | Comma-delimited list of accepted web origins. |
| `DATABASE_URL` | String | *Required* | PostgreSQL connection string. |
| `DATABASE_MAX_CONNECTIONS` | u32 | `20` | Maximum SQL connections in pool. |
| `DATABASE_MIN_CONNECTIONS` | u32 | `5` | Idle connections maintained in pool. |
| `JWT_SECRET` | String | *Required* | 32+ character key for JWT token hashing and validation. |
| `JWT_EXPIRATION_HOURS` | i64 | `72` | Hours before an authentication token expires. |
| `COOKIE_SECURE` | bool | `false` | Enables `Secure` flag on cookies (must be `true` on HTTPS). |
| `DOWNLOADS_DIR` | Path | `./data/downloads` | Folder where active torrent data is stored. |
| `LIBRARY_DIR` | Path | `./data/library` | Destination folder for organized anime files. |
| `COVERS_DIR` | Path | `./data/covers` | Storage folder for cached anime cover images. |
| `TEMP_ENCODE_DIR` | Path | `./data/temp_encode` | Working folder for ongoing FFmpeg transcode jobs. |
| `NYAA_BASE_URL` | String | `https://nyaa.si` | Nyaa.si source URL. |
| `NYAA_DEFAULT_CATEGORY` | String | `1_2` | Scrape category (`1_2` = English-translated anime). |
| `NYAA_AUTO_SYNC_INTERVAL_MINS` | u64 | `30` | Interval between automatic Nyaa background syncs. |
| `ANILIST_GRAPHQL_URL` | String | `https://graphql.anilist.co`| AniList GraphQL API endpoint. |
| `ANILIST_RATE_LIMIT_PER_MINUTE`| u32 | `90` | Token-bucket requests per minute limit. |
| `TORRENT_LISTEN_PORT` | u16 | `4240` | Port for incoming BitTorrent swarm peers. |
| `TORRENT_ENABLE_DHT` | bool | `true` | Enables mainline DHT for magnet link resolving. |
| `TORRENT_MAX_ACTIVE_DOWNLOADS` | usize | `5` | Maximum active downloads running in parallel. |
| `FFMPEG_BINARY_PATH` | String | `ffmpeg` | Path or command name to invoke FFmpeg binary. |
| `ENCODER_MAX_CONCURRENT_JOBS`| usize | `1` | Max concurrent video transcode processes. |

#### 3. Rust Configuration Loader Implementation (`src/core/config.rs`)

```rust
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub app_env: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub cookie_secure: bool,
    pub downloads_dir: PathBuf,
    pub library_dir: PathBuf,
    pub covers_dir: PathBuf,
    pub temp_encode_dir: PathBuf,
    pub nyaa_base_url: String,
    pub nyaa_default_category: String,
    pub anilist_graphql_url: String,
    pub anilist_rate_limit: u32,
    pub torrent_listen_port: u16,
    pub torrent_enable_dht: bool,
    pub ffmpeg_binary_path: String,
    pub encoder_max_concurrent_jobs: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Loads .env file if present, silently ignores if missing (e.g. in Docker)
        let _ = dotenvy::dotenv();

        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse()?,
            app_env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL environment variable must be set"),
            database_max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string()).parse()?,
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET environment variable must be set"),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "72".to_string()).parse()?,
            cookie_secure: std::env::var("COOKIE_SECURE")
                .unwrap_or_else(|_| "false".to_string()).parse()?,
            downloads_dir: PathBuf::from(std::env::var("DOWNLOADS_DIR")
                .unwrap_or_else(|_| "./data/downloads".to_string())),
            library_dir: PathBuf::from(std::env::var("LIBRARY_DIR")
                .unwrap_or_else(|_| "./data/library".to_string())),
            covers_dir: PathBuf::from(std::env::var("COVERS_DIR")
                .unwrap_or_else(|_| "./data/covers".to_string())),
            temp_encode_dir: PathBuf::from(std::env::var("TEMP_ENCODE_DIR")
                .unwrap_or_else(|_| "./data/temp_encode".to_string())),
            nyaa_base_url: std::env::var("NYAA_BASE_URL")
                .unwrap_or_else(|_| "https://nyaa.si".to_string()),
            nyaa_default_category: std::env::var("NYAA_DEFAULT_CATEGORY")
                .unwrap_or_else(|_| "1_2".to_string()),
            anilist_graphql_url: std::env::var("ANILIST_GRAPHQL_URL")
                .unwrap_or_else(|_| "https://graphql.anilist.co".to_string()),
            anilist_rate_limit: std::env::var("ANILIST_RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "90".to_string()).parse()?,
            torrent_listen_port: std::env::var("TORRENT_LISTEN_PORT")
                .unwrap_or_else(|_| "4240".to_string()).parse()?,
            torrent_enable_dht: std::env::var("TORRENT_ENABLE_DHT")
                .unwrap_or_else(|_| "true".to_string()).parse()?,
            ffmpeg_binary_path: std::env::var("FFMPEG_BINARY_PATH")
                .unwrap_or_else(|_| "ffmpeg".to_string()),
            encoder_max_concurrent_jobs: std::env::var("ENCODER_MAX_CONCURRENT_JOBS")
                .unwrap_or_else(|_| "1".to_string()).parse()?,
        })
    }
}
```

---

## 6. Real-Time WebSocket Communication Protocol

Nyaalist uses Axum WebSockets (`/api/ws`) with a unified JSON event stream.

### 6.1 Downloader Telemetry Frame (`torrent_progress`)
Sent every 1000ms while active torrents exist:

```json
{
  "event": "torrent_progress",
  "data": [
    {
      "id": "c1f7a0b3-9840-4281-b541-1144de9432bb",
      "info_hash": "4c1d82d9a6b5e4f3c2b1a09876543210fedcba98",
      "name": "[Erai-raws] Dragon Ball Super: Beerus - 07 [1080p CR WEBRip HEVC AAC]",
      "status": "DOWNLOADING",
      "progress_pct": 68.4,
      "downloaded_bytes": 943718400,
      "total_bytes": 1379991552,
      "download_speed": 12582912,
      "upload_speed": 524288,
      "eta_seconds": 34,
      "seeders": 42,
      "peers": 15
    }
  ]
}
```

### 6.2 Encoder Telemetry Frame (`encode_progress`)
Sent every 500ms during an active transcoding session:

```json
{
  "event": "encode_progress",
  "data": {
    "job_id": "8f3e2b10-6c9a-41d5-a332-9011f4e190aa",
    "file_name": "Dragon Ball Super: Beerus - 07.mkv",
    "preset_name": "720p x265 Balanced",
    "status": "ENCODING",
    "progress_pct": 43.7,
    "current_frame": 14208,
    "fps": 58.4,
    "speed": 2.43,
    "current_time_sec": 592.5,
    "total_duration_sec": 1420.0,
    "eta_seconds": 340,
    "source_size_bytes": 1379991552,
    "current_size_bytes": 320485760
  }
}
```

---

## 7. Frontend Architecture (SvelteKit 5 Panel)

The frontend is built using **SvelteKit** and **Svelte 5** leveraging modern Runes (`$state`, `$derived`, `$effect`) for state management and reactive rendering.

### 7.1 Directory Layout

```
frontend/
├── src/
│   ├── app.html
│   ├── app.css                   # Tailwind CSS + WattVision color variables
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts         # Type-safe fetch client
│   │   │   ├── ws.ts             # WebSocket manager with auto-reconnect
│   │   │   └── types.ts          # TypeScript interfaces for API & WS payloads
│   │   ├── components/           # WattVision Styled UI Components
│   │   │   ├── ui/
│   │   │   │   ├── Card.svelte       # #1E1E1E background, 16px radius, #2C2C2E border
│   │   │   │   ├── KpiCard.svelte    # 32px JetBrains Mono bold numbers, Neon Cyan accent
│   │   │   │   ├── Badge.svelte      # Release tag pills (1080p, HEVC, CRC)
│   │   │   │   ├── ProgressBar.svelte# Neon Cyan / Lime Green gradient progress
│   │   │   │   ├── Button.svelte     # Neon Cyan action button with glowing state
│   │   │   │   ├── DataTable.svelte  # Technical table with #252525 hover state
│   │   │   │   └── AlertBox.svelte   # #3A1C1C alert banner with #FF453A left border
│   │   │   ├── layout/
│   │   │   │   ├── Sidebar.svelte    # Navigation bar with active glow indicators
│   │   │   │   ├── Header.svelte     # Search bar, user profile, live WS status dot
│   │   │   │   └── LiveBadge.svelte  # #32D74B pulsing dot for active connection
│   │   │   ├── explore/
│   │   │   │   ├── ReleaseCard.svelte
│   │   │   │   ├── AniListModal.svelte
│   │   │   │   └── UserFilterBar.svelte
│   │   │   ├── downloader/
│   │   │   │   ├── TorrentRow.svelte
│   │   │   │   └── AddMagnetModal.svelte
│   │   │   ├── encoder/
│   │   │   │   ├── EncodeJobCard.svelte
│   │   │   │   └── PresetEditorModal.svelte
│   │   │   └── library/
│   │   │       ├── FileTree.svelte
│   │   │       └── RenameModal.svelte
│   │   └── stores/               # Svelte 5 Rune State Singletons
│   │       ├── auth.svelte.ts    # User session and setup state
│   │       ├── torrents.svelte.ts# Live torrent telemetry store
│   │       ├── encoder.svelte.ts # Live transcode telemetry store
│   │       └── config.svelte.ts  # System settings store
│   └── routes/
│       ├── +layout.svelte        # Shell layout & persistent WebSocket connection
│       ├── (auth)/
│       │   ├── login/+page.svelte# User login form
│       │   └── setup/+page.svelte# First-run administrator wizard
│       └── (app)/
│           ├── +layout.svelte    # Authenticated app shell with Sidebar & Header
│           ├── +page.svelte      # Dashboard: KPIs, active downloads, recent encodes
│           ├── explore/+page.svelte  # Nyaa feed, search, uploader filtering
│           ├── downloads/+page.svelte# Full torrent download manager
│           ├── library/+page.svelte  # Library file manager & destination mover
│           ├── encoder/+page.svelte  # FFmpeg transcode queue & preset config
│           └── settings/+page.svelte # Configurator for folders, defaults, auth
```

### 7.2 UI Wireframes & Layout Mockups

#### 1. Technical Dashboard (`/`)
```
+----------------------------------------------------------------------------------------------------+
|  [nyaalist]     Search anime or magnet...                  [WS: LIVE (Green Dot)] [Admin (Profile)]|
+----------------------------------------------------------------------------------------------------+
| [Sidebar]   |  [ KPI 1: Active Speed ]   [ KPI 2: Active Torrents ]   [ KPI 3: Transcoding Jobs ]  |
|             |  18.4 MB/s [Cyan Mono]     3 Downloading / 12 Seed      1 Active (720p x265)         |
| > Dashboard |--------------------------------------------------------------------------------------|
| > Explore   |  Active Downloads (8 cols)                              | System Alerts (4 cols)     |
| > Downloads |  [Erai-raws] Dragon Ball Super: Beerus - 07             | [!] Low disk space warning |
| > Library   |  ========================>-------------- [68.4%]        |     /data/downloads (88%)  |
| > Encoder   |  Speed: 12.5 MB/s | Seeds: 42 | ETA: 34s [Cyan/Green]   |                            |
| > Settings  |---------------------------------------------------------| [✓] Encode completed       |
|             |  Active Transcoding                                     |     Kozame Ep 93 (720p)    |
|             |  Odekake Kozame - 93.mkv -> 720p x265                   |     Saved: 68% size        |
|             |  ==============>------------------------ [43.7%]        |                            |
|             |  FPS: 58.4 | Speed: 2.43x | ETA: 05m 40s                |                            |
+----------------------------------------------------------------------------------------------------+
```

#### 2. Nyaa Explorer with AniList Metadata Sync (`/explore`)
```
+----------------------------------------------------------------------------------------------------+
| Filters: [Category: Anime (1_2) v]  [Uploader: Erai-raws [x]]  [Search: Dragon Ball             ]  |
+----------------------------------------------------------------------------------------------------+
| Cover   | Release Title & Tags                       | AniList Match         | Size   | S / L | Act|
+---------+--------------------------------------------+-----------------------+--------+-------+----+
| [Cover] | [Erai-raws] Dragon Ball Super: Beerus - 07 | Dragon Ball Super     | 1.3 GB | 42/15 |[DL]|
| (Local  | Tags: [1080p] [HEVC] [AAC] [MultiSub]      | Score: 78% | TV (131) |        | [Cyan]|    |
|  Cache) | Hash: [4C1D82D9]                           | Airing: Finished      |        | [Lime]|    |
+---------+--------------------------------------------+-----------------------+--------+-------+----+
| [Cover] | [Kozame] Odekake Kozame - 93               | Odekake Kozame        | 85 MB  | 18/2  |[DL]|
|         | Tags: [1080p] | Hash: [CC2F9599]           | Score: 72% | ONA (100)|        |       |    |
+----------------------------------------------------------------------------------------------------+
```

---

## 8. Deployment & DevOps (Docker Compose)

The application is deployed as a coordinated stack using Docker Compose.

### 8.1 Backend Dockerfile (`backend/Dockerfile`)
Uses a multi-stage Rust build with `ffmpeg` installed in the final Debian bookworm-slim runner:

```dockerfile
# Stage 1: Build binary
FROM rust:1.80-bullseye AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

# Stage 2: Minimal runtime with FFmpeg
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/nyaalist-backend /app/nyaalist-backend
COPY --from=builder /app/migrations /app/migrations

ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/app/nyaalist-backend"]
```

### 8.2 Frontend Dockerfile (`frontend/Dockerfile`)
Builds SvelteKit with Node adapter:

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:20-alpine AS runner
WORKDIR /app
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules
ENV PORT=3000
EXPOSE 3000
CMD ["node", "build"]
```

### 8.3 Docker Compose Specification (`docker-compose.yml`)

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: nyaalist-db
    restart: unless-stopped
    environment:
      POSTGRES_DB: nyaalist
      POSTGRES_USER: nyaa_user
      POSTGRES_PASSWORD: nyaa_secure_password
    volumes:
      - nyaalist-pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U nyaa_user -d nyaalist"]
      interval: 5s
      timeout: 5s
      retries: 5

  backend:
    build:
      context: ./backend
      dockerfile: Dockerfile
    container_name: nyaalist-backend
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    env_file:
      - .env
    volumes:
      - ./data/downloads:/data/downloads
      - ./data/library:/data/library
      - ./data/covers:/data/covers
    ports:
      - "8080:8080"

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile
    container_name: nyaalist-frontend
    restart: unless-stopped
    depends_on:
      - backend
    environment:
      ORIGIN: http://localhost:3000
      PUBLIC_API_URL: http://localhost:8080
      PUBLIC_WS_URL: ws://localhost:8080/api/ws
    ports:
      - "3000:3000"

volumes:
  nyaalist-pgdata:
```

---

## 9. Implementation Roadmap & Milestones

| Phase | Objective | Deliverables |
| :--- | :--- | :--- |
| **Phase 1** | Project Initialization & Database | - Scaffolding `backend/` (Cargo), `.env.example`, and `frontend/` (SvelteKit 5 + Tailwind).<br>- Typed environment configuration loader (`core/config.rs`).<br>- Write PostgreSQL DDL migrations (`001_init.sql`).<br>- Implement Axum server with `AppState`, `PgPool`, and `AppError`. |
| **Phase 2** | Auth & Setup Slice (`features/auth`) | - First-run admin detection logic (`/api/auth/status`).<br>- Setup wizard endpoint (`/api/auth/setup`) with Argon2id password hashing.<br>- JWT issue/verification middleware. |
| **Phase 3** | Explore & Sync Slice (`features/explore`) | - Nyaa HTML & RSS scrapers (`nyaa_scraper.rs`).<br>- Anitomy-style token extraction (`title_parser.rs`).<br>- AniList GraphQL client with rate limiter & cover image cacher.<br>- Endpoints: `GET /api/explore/latest`, `GET /api/explore/user/:username`. |
| **Phase 4** | Downloader Slice (`features/downloader`) | - Integrate `librqbit` BitTorrent engine.<br>- Magnet link resolver & file download tasks.<br>- WebSocket broadcast bus streaming live speeds, progress, seeds, and ETA. |
| **Phase 5** | Library Management (`features/library`) | - Disk file scanner.<br>- File move / copy engine with pattern renaming templates (`{title} - S{season}E{episode}`). |
| **Phase 6** | Video Transcoder (`features/encoder`) | - Tokio Command subprocess runner invoking `ffmpeg` with `-progress pipe:1`.<br>- Transcoding presets (`720p x265 Balanced`, `1080p x265 HQ`).<br>- Real-time WebSocket streaming of frame, fps, speed, and percentage. |
| **Phase 7** | Frontend UI Implementation | - Implement WattVision dark theme components (`Card`, `KpiCard`, `ProgressBar`, `DataTable`, `Badge`).<br>- Svelte 5 Rune-based reactive stores (`torrents.svelte.ts`, `encoder.svelte.ts`).<br>- Pages: `/login`, `/setup`, `/`, `/explore`, `/downloads`, `/library`, `/encoder`, `/settings`. |
| **Phase 8** | Docker Deployment & E2E Verification | - Multi-stage `Dockerfile.backend` with FFmpeg.<br>- Multi-stage `Dockerfile.frontend`.<br>- `docker-compose.yml` validation with volumes for downloads, library, and PostgreSQL data. |
