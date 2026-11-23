export interface UserResponse {
  id: string;
  username: string;
  role: string;
}

export interface AuthStatusResponse {
  setup_required: boolean;
  authenticated: boolean;
  user: UserResponse | null;
}

export interface AuthSuccessResponse {
  message: string;
  user: UserResponse;
  token: string;
}

export interface SetupRequest {
  username: string;
  password: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface ParsedRelease {
  uploader?: string;
  anime_title: string;
  episode?: string;
  resolution?: string;
  codec?: string;
  audio?: string;
  source?: string;
  crc32?: string;
}

export interface AnimeMetadata {
  id: number;
  mal_id?: number;
  title_romaji: string;
  title_english?: string;
  title_native?: string;
  format?: string;
  status?: string;
  episodes?: number;
  season?: string;
  season_year?: number;
  genres?: string[];
  average_score?: number;
  description?: string;
  cover_image_url?: string;
  cover_image_local_path?: string;
  banner_image_url?: string;
  last_synced_at: string;
}

export interface NyaaRelease {
  id: number;
  title: string;
  uploader?: string;
  magnet_uri: string;
  torrent_file_url?: string;
  category: string;
  file_size_bytes: number;
  file_size_formatted: string;
  seeders: number;
  leechers: number;
  completed_count: number;
  nyaa_created_at: string;
  parsed: ParsedRelease;
  anilist_metadata?: AnimeMetadata | null;
}

export interface DownloadJob {
  id: string;
  nyaa_torrent_id: number | null;
  info_hash: string;
  name: string;
  magnet_uri: string;
  save_path: string;
  status: 'QUEUED' | 'DOWNLOADING' | 'PAUSED' | 'COMPLETED' | 'ERROR';
  total_bytes: number;
  downloaded_bytes: number;
  uploaded_bytes: number;
  download_speed: number;
  upload_speed: number;
  progress_pct: number;
  eta_seconds: number | null;
  seeders: number;
  peers: number;
  error_message: string | null;
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface TorrentStatsSnapshot {
  id: string;
  info_hash: string;
  name: string;
  status: string;
  total_bytes: number;
  downloaded_bytes: number;
  uploaded_bytes: number;
  download_speed: number;
  upload_speed: number;
  progress_pct: number;
  eta_seconds: number | null;
  seeders: number;
  peers: number;
  error_message?: string | null;
}

export interface AddTorrentRequest {
  magnet_uri: string;
  nyaa_id?: number | null;
}

export interface LibraryFile {
  id: string;
  download_job_id?: string | null;
  anilist_id?: number | null;
  original_filename: string;
  current_path: string;
  target_library_path?: string | null;
  episode?: string | null;
  season?: number | null;
  resolution?: string | null;
  file_size_bytes: number;
  is_moved: boolean;
  is_encoded: boolean;
  created_at: string;
  updated_at: string;
}

export interface LibraryFileItem extends LibraryFile {
  anime_title?: string | null;
  anime_cover_path?: string | null;
}

export interface MoveFileRequest {
  file_id?: string | null;
  source_path: string;
  target_path: string;
  new_filename?: string | null;
}

export interface AutoOrganizeRequest {
  file_id?: string | null;
  download_job_id?: string | null;
  template?: string | null;
}

export interface ScanDirectoryResult {
  scanned_count: number;
  new_files_count: number;
  files: LibraryFile[];
}

export interface EncodingPreset {
  id: string;
  name: string;
  description?: string | null;
  video_codec: string;
  preset: string;
  crf: number;
  target_resolution?: string | null;
  audio_codec: string;
  audio_bitrate?: string | null;
  container: string;
  custom_ffmpeg_args?: string | null;
  is_default: boolean;
  created_at: string;
}

export interface EncodingJob {
  id: string;
  source_file_path: string;
  target_file_path: string;
  preset_id: string;
  status: 'QUEUED' | 'ENCODING' | 'COMPLETED' | 'FAILED' | 'CANCELLED';
  progress_pct: number;
  current_frame: number;
  total_frames?: number | null;
  fps: number;
  current_time_sec: number;
  total_duration_sec: number;
  speed: number;
  source_size_bytes: number;
  encoded_size_bytes: number;
  eta_seconds?: number | null;
  error_message?: string | null;
  created_at: string;
  started_at?: string | null;
  completed_at?: string | null;
}

export interface EncodingJobItem extends EncodingJob {
  preset_name: string;
  source_filename: string;
}

export interface EncodeProgressTelemetry {
  job_id: string;
  file_name: string;
  preset_name: string;
  status: string;
  progress_pct: number;
  current_frame: number;
  fps: number;
  speed: number;
  current_time_sec: number;
  total_duration_sec: number;
  eta_seconds?: number | null;
  source_size_bytes: number;
  current_size_bytes: number;
}

export interface CreatePresetRequest {
  name: string;
  description?: string;
  video_codec?: string;
  preset?: string;
  crf?: number;
  target_resolution?: string;
  audio_codec?: string;
  audio_bitrate?: string;
  container?: string;
  custom_ffmpeg_args?: string;
  is_default?: boolean;
}

export interface QueueJobRequest {
  source_file_path: string;
  preset_id?: string;
  target_file_path?: string;
}
