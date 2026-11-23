import { api } from '#lib/api/client';
import type { UserResponse } from '#lib/api/types';

class AuthStore {
  setupRequired = $state<boolean>(false);
  isAuthenticated = $state<boolean>(false);
  user = $state<UserResponse | null>(null);
  loading = $state<boolean>(true);
  error = $state<string | null>(null);

  async init() {
    this.loading = true;
    this.error = null;
    try {
      const status = await api.auth.getStatus();
      this.setupRequired = status.setup_required;
      this.isAuthenticated = status.authenticated;
      this.user = status.user;
    } catch (err: any) {
      console.warn('Auth status check offline or failed:', err);
      // Defaults to offline state
    } finally {
      this.loading = false;
    }
  }

  async setup(username: string, password: string) {
    this.loading = true;
    this.error = null;
    try {
      const res = await api.auth.setup({ username, password });
      this.user = res.user;
      this.isAuthenticated = true;
      this.setupRequired = false;
      return res;
    } catch (err: any) {
      this.error = err.message || 'Setup failed';
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async login(username: string, password: string) {
    this.loading = true;
    this.error = null;
    try {
      const res = await api.auth.login({ username, password });
      this.user = res.user;
      this.isAuthenticated = true;
      return res;
    } catch (err: any) {
      this.error = err.message || 'Login failed';
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async logout() {
    try {
      await api.auth.logout();
    } catch (e) {
      // Ignored
    } finally {
      this.user = null;
      this.isAuthenticated = false;
    }
  }
}

export const auth = new AuthStore();
