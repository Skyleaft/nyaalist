# Nyaalist

<div align="center">

**Self-Hosted Anime Torrent Downloader, Automated Library Organizer & Video Transcoder**

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.7-blue.svg)](https://github.com/tokio-rs/axum)
[![Svelte](https://img.shields.io/badge/Svelte-5.2-ff3e00.svg?logo=svelte)](https://svelte.dev/)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-2.0-ff3e00.svg)](https://kit.svelte.dev/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind-4.0-38bdf8.svg?logo=tailwind-css)](https://tailwindcss.com/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-19-336791.svg?logo=postgresql)](https://www.postgresql.org/)
[![SeaORM](https://img.shields.io/badge/SeaORM-2.0-00E5FF.svg)](https://www.sea-ql.org/SeaORM/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

## Overview

**Nyaalist** is a modern, self-hosted media platform built for anime collectors and homelab enthusiasts. It consolidates your entire anime consumption pipeline into a single, high-performance web dashboard:

1. **Browse & Search:** Live scraping of Nyaa.si releases with intelligent title parsing and AniList metadata synchronization.
2. **Download:** Built-in BitTorrent engine powered by `librqbit` — no third-party clients (qBittorrent/Transmission) required.
3. **Organize:** Automated file detection, customizable directory naming templates for Jellyfin and Plex, and manual file management.
4. **Transcode:** Background FFmpeg queue worker with customizable encoding presets (HEVC x265, CRF, resolution scaling) and live progress telemetry.
5. **Monitor:** Real-time WebSockets delivering instantaneous download speeds, peer stats, and transcode telemetry with zero page refreshing.

---

## Key Features

### 🌐 Live Nyaa Explorer & Filter Pipeline
- Live scraped feeds from **Nyaa.si** with automatic parsing of release titles, episode numbers, resolution tags (`1080p`, `720p`), video codecs (`HEVC`, `x264`), and CRC32 hashes.
- Combined **Uploader Filter + Search Query** (e.g. search for `Frieren` within `SubsPlease`).
- Dynamic **Date Sorting** (ascending/descending) with localized timestamp tooltips and relative upload times (`2h ago`, `3d ago`).
- Interactive uploader badges for instant 1-click catalog filtering.

### 🌸 AniList Metadata Enrichment & Local DB Caching
- Seamlessly matches releases to **AniList GraphQL** entries (Romaji/English titles, cover posters, episode counts, formats, and community scores).
- **Sub-10ms Batch Resolution:** Releases are first cross-referenced against local PostgreSQL tables (`title_mappings` and `anime_metadata`) before touching external APIs.
- **Asynchronous Rate Limiting:** Non-blocking background sync queue with strict 90 req/min compliance and exponential backoff on HTTP 429 `Retry-After` responses.

### ⚡ Built-in BitTorrent Engine (`librqbit`)
- Fully embedded Rust torrent client with magnet link resolution, DHT support, and configurable peer limits.
- Real-time download/upload speed meters, progress percentages, ETA calculations, and active peer counts.
- Pause, resume, and cancellation with optional on-disk data purging.

### 📁 Automated Media Library Organizer
- Completed download watcher and manual directory scanner for `.mkv`, `.mp4`, and `.avi` containers.
- **Template Renaming Engine:** Organizes media into standard Plex and Jellyfin directory patterns:
  ```
  {anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}
  ```
- File status tracking with visual indicators (`✓ ORGANIZED`, `PENDING MOVE`, `✓ ENCODED`, `RAW SOURCE`).

### 🎬 Integrated FFmpeg Video Transcoder
- Background transcoding queue worker (`EncodingQueueWorker`) with automatic preset management.
- Default and custom encoding presets (HEVC/x265, CRF quality targets, speed presets, audio copy, and container packaging).
- **1-Click Library Encode:** Trigger encoding directly from the Media Library with an interactive preset selector and destination preview.
- Live telemetry streaming over WebSockets: current frame, FPS, speed multiplier (e.g. `2.4x`), current time, and real-time output size.

### 🎨 WattVision High-Contrast Technical UI
- Designed with high-contrast technical dark mode aesthetics (`#121212`, `#1E1E1E`, `#00E5FF`, `#32D74B`, `#FF453A`).
- Built with **Svelte 5 Runes** (`$state`, `$derived`) and **Tailwind CSS v4** for lightning-fast responsiveness.
- Real-time WebSocket connection state monitoring.

### 🔒 Security & Multi-User Authentication
- First-run setup wizard for initializing administrator credentials.
- Password hashing with **Argon2id**.
- Stateless session management with signed **JWT cookies** (`HttpOnly`, `SameSite=Lax`).

---

## Architecture

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

## Tech Stack

| Component | Technology | Version | Description |
| :--- | :--- | :--- | :--- |
| **Backend Language** | Rust | 1.80+ | Memory safety, zero-cost abstractions, multi-threaded performance |
| **Web Framework** | Axum | 0.7 | High-performance asynchronous HTTP & WebSocket server |
| **Async Runtime** | Tokio | 1.40 | Non-blocking async event-driven runtime |
| **ORM & Migrations** | SeaORM | 2.0 | Automated schema migrations and type-safe relational models |
| **Database Pool** | SQLx | 0.8 | Raw async PostgreSQL driver for bulk operations and telemetry |
| **Database** | PostgreSQL | 19-alpine | Relational persistence with UUID and TIMESTAMPTZ support |
| **Torrent Engine** | librqbit | 9.1 | Embedded high-speed BitTorrent engine |
| **Video Processing** | FFmpeg / ffprobe | System | Video transcoding, compression, and stream analysis |
| **Frontend Framework** | Svelte / SvelteKit | 5.2 / 2.0 | Next-gen rune-based reactivity and server-side rendering |
| **CSS Styling** | Tailwind CSS | 4.0 | Modern utility-first stylesheet engine |
| **Build Tool** | Vite | 8.3 | Instant HMR development server and production bundler |

---

## Quick Start

### Option 1: Docker Compose (Recommended)

1. **Clone the repository:**
   ```bash
   git clone https://github.com/Skyleaft/nyaalist.git
   cd nyaalist
   ```

2. **Configure environment variables:**
   ```bash
   cp .env.example .env
   # Modify JWT_SECRET, ports, or storage paths if desired
   ```

3. **Start the containers:**
   ```bash
   docker compose up --build -d
   ```

4. **Access the application:**
   - **Frontend UI:** [http://localhost:3000](http://localhost:3000)
   - **Backend API:** [http://localhost:5041](http://localhost:5041)

   On first launch, you will be greeted by the **Setup Wizard** to create your admin credentials.

---

### Option 2: Local Development

#### Prerequisites
- **Rust:** `1.80+` with Cargo (`rustup update`)
- **Node.js:** `v20+` and `pnpm` (`npm install -g pnpm`)
- **PostgreSQL:** Running instance (`localhost:5432`)
- **FFmpeg & FFprobe:** Installed and available in your `PATH`

#### 1. Setup Database
```sql
CREATE DATABASE nyaalist;
CREATE USER nyaa_user WITH ENCRYPTED PASSWORD 'nyaa_secure_password';
GRANT ALL PRIVILEGES ON DATABASE nyaalist TO nyaa_user;
```

#### 2. Run Backend
```bash
cd backend
cp ../.env.example .env
# Ensure DATABASE_URL matches your PostgreSQL connection

# Run check and start server
cargo check
cargo run
```
*SeaORM migrations will automatically verify and execute on startup.*

#### 3. Run Frontend
```bash
cd frontend
pnpm install
pnpm run dev
```
Open [http://localhost:3000](http://localhost:3000) in your browser.

---

## Configuration Reference (`.env`)

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `HOST` | `0.0.0.0` | Bind host address for the backend server |
| `PORT` | `8080` (or `5041`) | HTTP & WebSocket server port |
| `APP_ENV` | `development` | Environment mode (`development` or `production`) |
| `DATABASE_URL` | `postgres://...` | PostgreSQL connection string |
| `JWT_SECRET` | `change_me_...` | Secret key for signing session tokens (min 32 bytes) |
| `JWT_EXPIRATION_HOURS` | `72` | Session lifetime in hours |
| `DOWNLOADS_DIR` | `./data/downloads` | Directory where active torrents download |
| `LIBRARY_DIR` | `./data/library` | Destination directory for organized media |
| `COVERS_DIR` | `./data/covers` | Storage directory for cached anime posters |
| `TEMP_ENCODE_DIR` | `./data/temp_encode` | Working folder for active FFmpeg transcoding |
| `NYAA_BASE_URL` | `https://nyaa.si` | Nyaa source domain |
| `NYAA_DEFAULT_CATEGORY`| `1_2` | Default category (English-translated Anime) |
| `ANILIST_GRAPHQL_URL` | `https://graphql.anilist.co` | AniList API endpoint |
| `ANILIST_RATE_LIMIT_PER_MINUTE` | `90` | Max external API requests per minute |
| `TORRENT_LISTEN_PORT` | `4240` | BitTorrent listen port for inbound connections |
| `TORRENT_MAX_ACTIVE_DOWNLOADS` | `5` | Maximum simultaneous downloading torrents |
| `FFMPEG_BINARY_PATH` | `ffmpeg` | Path to `ffmpeg` executable |
| `FFPROBE_BINARY_PATH` | `ffprobe` | Path to `ffprobe` executable |

---

## API Overview

### Authentication (`/api/auth`)
- `GET /api/auth/status` - Check if setup is required or user is authenticated
- `POST /api/auth/setup` - Initial admin registration
- `POST /api/auth/login` - User login & JWT cookie issuance
- `POST /api/auth/logout` - Invalidate session
- `GET /api/auth/me` - Current user identity

### Explore & AniList (`/api/explore`)
- `GET /api/explore/latest?category=1_2&page=1&query=...&sort=id&order=desc` - Scrape releases
- `GET /api/explore/user/:username?page=1&query=...&sort=id&order=desc` - Scrape uploader releases
- `POST /api/explore/sync` - Manual title AniList synchronization
- `GET /api/covers/:filename` - Serve cached anime poster images

### Downloader (`/api/downloader`)
- `GET /api/downloader/jobs` - List all torrent jobs
- `POST /api/downloader/add` - Queue magnet link (`{ magnet_uri, nyaa_id }`)
- `POST /api/downloader/:id/pause` - Pause download
- `POST /api/downloader/:id/resume` - Resume download
- `DELETE /api/downloader/:id?delete_files=false` - Remove torrent job

### Library (`/api/library`)
- `GET /api/library/files?is_moved=true&search=...` - List tracked library files
- `POST /api/library/scan` - Scan download folders for video files
- `POST /api/library/move` - Manual file relocation / renaming
- `POST /api/library/auto-organize` - Execute template auto-organization
- `POST /api/library/files/:id/encode` - Queue file for background video encoding
- `DELETE /api/library/files/:id` - Untrack file

### Encoder (`/api/encoder`)
- `GET /api/encoder/presets` - List encoding presets
- `POST /api/encoder/presets` - Create new preset
- `GET /api/encoder/jobs?status=ENCODING` - List transcoding jobs
- `POST /api/encoder/jobs` - Enqueue encoding task
- `POST /api/encoder/jobs/:id/cancel` - Terminate encoding job
- `DELETE /api/encoder/jobs/:id` - Delete job record

### WebSocket Telemetry (`/api/ws`)
- `torrent_progress` - Broadcasts download/upload speeds, percentages, ETAs, and peers.
- `encode_progress` - Broadcasts encoding frames, FPS, speed multiplier, elapsed time, and sizes.
- `anilist_metadata_synced` - Notifies frontend when background AniList metadata resolves.

---

## Testing & Quality Assurance

```bash
# Backend unit tests
cargo test --manifest-path backend/Cargo.toml

# Backend compilation check
cargo check --manifest-path backend/Cargo.toml

# Frontend typechecking
pnpm --dir frontend run check

# Frontend production build
pnpm --dir frontend run build
```

---

## License

This project is licensed under the [MIT License](LICENSE).
