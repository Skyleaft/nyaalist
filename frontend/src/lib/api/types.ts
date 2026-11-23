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
