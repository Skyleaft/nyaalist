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
