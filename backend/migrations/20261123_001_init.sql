-- ==============================================================================
-- Nyaalist Initial Migration: 20261123_001_init.sql
-- Target: PostgreSQL 19
-- ==============================================================================

-- Extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. Users Table (Authentication)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'admin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Dynamic Application Configuration Store
CREATE TABLE IF NOT EXISTS app_config (
    key VARCHAR(64) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed Initial Default Configuration
INSERT INTO app_config (key, value) VALUES 
('paths', '{"download_dir": "/data/downloads", "library_dir": "/data/library", "covers_dir": "/data/covers"}'),
('downloader', '{"max_active_downloads": 5, "max_download_speed_kb": 0, "max_upload_speed_kb": 0, "auto_move_on_complete": false}'),
('nyaa', '{"base_url": "https://nyaa.si", "default_category": "1_2", "auto_sync_interval_mins": 30}'),
('naming', '{"template": "{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}]"}')
ON CONFLICT (key) DO NOTHING;

-- 3. AniList Anime Metadata Cache
CREATE TABLE IF NOT EXISTS anime_metadata (
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

CREATE INDEX IF NOT EXISTS idx_anime_metadata_title_romaji ON anime_metadata (LOWER(title_romaji));
CREATE INDEX IF NOT EXISTS idx_anime_metadata_title_english ON anime_metadata (LOWER(title_english));

-- 4. Cleaned Title Mapping Cache (Minimizes AniList API requests)
CREATE TABLE IF NOT EXISTS title_mappings (
    raw_title_cleaned TEXT PRIMARY KEY,
    anilist_id INT REFERENCES anime_metadata(id) ON DELETE SET NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Nyaa Torrents (Scraped Releases)
CREATE TABLE IF NOT EXISTS nyaa_torrents (
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

CREATE INDEX IF NOT EXISTS idx_nyaa_torrents_uploader ON nyaa_torrents (uploader);
CREATE INDEX IF NOT EXISTS idx_nyaa_torrents_anilist_id ON nyaa_torrents (anilist_id);
CREATE INDEX IF NOT EXISTS idx_nyaa_torrents_created ON nyaa_torrents (nyaa_created_at DESC);

-- 6. Active and Completed Download Jobs
CREATE TABLE IF NOT EXISTS download_jobs (
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
CREATE TABLE IF NOT EXISTS library_files (
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
CREATE TABLE IF NOT EXISTS encoding_presets (
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
('1080p x265 High Quality', 'Re-encode to 1080p HEVC CRF 21 for storage efficiency', 'libx265', 'medium', 21, '1920x1080', 'copy', 'mkv', FALSE)
ON CONFLICT (name) DO NOTHING;

-- 9. FFmpeg Encoding Queue
CREATE TABLE IF NOT EXISTS encoding_jobs (
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
