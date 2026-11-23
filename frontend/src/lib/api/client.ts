import type {
  AuthStatusResponse,
  AuthSuccessResponse,
  LoginRequest,
  NyaaRelease,
  AnimeMetadata,
  SetupRequest,
  UserResponse
} from './types';

const API_BASE = '/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
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
  }
};
