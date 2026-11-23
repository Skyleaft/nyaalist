import type {
  AuthStatusResponse,
  AuthSuccessResponse,
  LoginRequest,
  NyaaRelease,
  AnimeMetadata,
  SetupRequest,
  UserResponse,
  DownloadJob,
  AddTorrentRequest,
  LibraryFileItem,
  MoveFileRequest,
  AutoOrganizeRequest,
  ScanDirectoryResult
} from './types';
export function getApiBase(): string {
  const customUrl = import.meta.env.PUBLIC_API_URL;
  if (customUrl && typeof customUrl === 'string' && customUrl.trim().length > 0) {
    return `${customUrl.trim().replace(/\/+$/, '')}/api`;
  }
  return '/api';
}

export function getWebSocketUrl(endpoint = '/api/ws'): string {
  const customWs = import.meta.env.PUBLIC_WS_URL;
  if (customWs && typeof customWs === 'string' && customWs.trim().length > 0) {
    return customWs.trim();
  }
  if (typeof window !== 'undefined') {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${proto}//${window.location.host}${endpoint}`;
  }
  return `ws://localhost:8080${endpoint}`;
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const baseUrl = getApiBase();
  const res = await fetch(`${baseUrl}${path}`, {
    ...options,
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers
    }
  });

  const data = await res.json().catch(() => ({}));

  if (!res.ok) {
    throw new Error(data.error || `Request failed with status ${res.status}`);
  }

  return data as T;
}

export const api = {
  auth: {
    getStatus: () => request<AuthStatusResponse>('/auth/status'),
    setup: (payload: SetupRequest) =>
      request<AuthSuccessResponse>('/auth/setup', {
        method: 'POST',
        body: JSON.stringify(payload)
      }),
    login: (payload: LoginRequest) =>
      request<AuthSuccessResponse>('/auth/login', {
        method: 'POST',
        body: JSON.stringify(payload)
      }),
    logout: () =>
      request<{ message: string }>('/auth/logout', {
        method: 'POST'
      }),
    getMe: () => request<UserResponse>('/auth/me')
  },
  explore: {
    getLatest: (params?: { category?: string; page?: number; query?: string }) => {
      const sp = new URLSearchParams();
      if (params?.category) sp.set('category', params.category);
      if (params?.page) sp.set('page', params.page.toString());
      if (params?.query) sp.set('query', params.query);
      return request<NyaaRelease[]>(`/explore/latest?${sp.toString()}`);
    },
    getUserReleases: (username: string, page?: number) => {
      const sp = new URLSearchParams();
      if (page) sp.set('page', page.toString());
      return request<NyaaRelease[]>(`/explore/user/${encodeURIComponent(username)}?${sp.toString()}`);
    },
    syncMetadata: (title: string, nyaa_id?: number) =>
      request<AnimeMetadata | null>('/explore/sync', {
        method: 'POST',
        body: JSON.stringify({ title, nyaa_id })
      })
  },
  downloader: {
    getJobs: () => request<DownloadJob[]>('/downloader/jobs'),
    add: (payload: AddTorrentRequest) =>
      request<DownloadJob>('/downloader/add', {
        method: 'POST',
        body: JSON.stringify(payload)
      }),
    pause: (id: string) =>
      request<{ status: string; id: string }>(`/downloader/${id}/pause`, {
        method: 'POST'
      }),
    resume: (id: string) =>
      request<{ status: string; id: string }>(`/downloader/${id}/resume`, {
        method: 'POST'
      }),
    delete: (id: string, deleteFiles = false) =>
      request<{ status: string; id: string }>(`/downloader/${id}?delete_files=${deleteFiles}`, {
        method: 'DELETE'
      })
  },
  library: {
    getFiles: (params?: { is_moved?: boolean; search?: string }) => {
      const sp = new URLSearchParams();
      if (params?.is_moved !== undefined) sp.set('is_moved', params.is_moved.toString());
      if (params?.search) sp.set('search', params.search);
      return request<LibraryFileItem[]>(`/library/files?${sp.toString()}`);
    },
    scan: () =>
      request<ScanDirectoryResult>('/library/scan', {
        method: 'POST'
      }),
    move: (payload: MoveFileRequest) =>
      request<{ status: string; source: string; target: string }>('/library/move', {
        method: 'POST',
        body: JSON.stringify(payload)
      }),
    autoOrganize: (payload: AutoOrganizeRequest) =>
      request<{ status: string; organized_count: number; files: any[] }>('/library/auto-organize', {
        method: 'POST',
        body: JSON.stringify(payload)
      }),
    delete: (id: string) =>
      request<{ status: string; id: string }>(`/library/files/${id}`, {
        method: 'DELETE'
      })
  }
};
