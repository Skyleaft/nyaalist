export type ThemeMode = 'dark' | 'light' | 'auto';

class ThemeStore {
  mode = $state<ThemeMode>('auto');
  resolved = $state<'dark' | 'light'>('dark');

  init() {
    if (typeof window === 'undefined') return;

    try {
      const saved = localStorage.getItem('nyaalist-theme') as ThemeMode | null;
      if (saved && (saved === 'dark' || saved === 'light' || saved === 'auto')) {
        this.mode = saved;
      } else {
        this.mode = 'auto';
      }
    } catch (_) {
      this.mode = 'auto';
    }

    this.apply();

    // Listen for OS system theme changes
    try {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', () => {
        if (this.mode === 'auto') {
          this.apply();
        }
      });
    } catch (_) {}
  }

  setMode(newMode: ThemeMode) {
    this.mode = newMode;
    try {
      if (typeof window !== 'undefined') {
        localStorage.setItem('nyaalist-theme', newMode);
      }
    } catch (_) {}
    this.apply();
  }

  apply() {
    if (typeof window === 'undefined') return;

    let isDark = true;
    if (this.mode === 'auto') {
      isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    } else {
      isDark = this.mode === 'dark';
    }

    this.resolved = isDark ? 'dark' : 'light';
    const root = document.documentElement;

    if (isDark) {
      root.classList.add('dark');
      root.classList.remove('light');
      root.setAttribute('data-theme', 'dark');
      root.style.colorScheme = 'dark';
    } else {
      root.classList.remove('dark');
      root.classList.add('light');
      root.setAttribute('data-theme', 'light');
      root.style.colorScheme = 'light';
    }
  }

  cycle() {
    const next: Record<ThemeMode, ThemeMode> = {
      dark: 'light',
      light: 'auto',
      auto: 'dark'
    };
    this.setMode(next[this.mode]);
  }
}

export const theme = new ThemeStore();
