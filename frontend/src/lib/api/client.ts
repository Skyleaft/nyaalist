import type {
  AuthStatusResponse,
  AuthSuccessResponse,
  LoginRequest,
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
  }
};
