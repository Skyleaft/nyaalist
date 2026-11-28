<script lang="ts">
  import '../app.css';
  import { auth } from '#lib/stores/auth.svelte';
  import { theme } from '#lib/stores/theme.svelte';
  import { torrents } from '#lib/stores/torrents.svelte';
  import { library } from '#lib/stores/library.svelte';
  import { encoder } from '#lib/stores/encoder.svelte';
  import { translator } from '#lib/stores/translator.svelte';
  import { onMount } from 'svelte';
  import { crossfade, fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import {
    Compass,
    LayoutDashboard,
    DownloadCloud,
    Library,
    Film,
    Languages,
    Settings,
    LogOut,
    LogIn,
    Sun,
    Moon,
    Monitor,
    ChevronDown,
    Search,
    X,
    Shield,
    ArrowRight
  } from 'lucide-svelte';

  let { children } = $props();

  let scrollY = $state(0);
  const isScrolled = $derived(scrollY > 16);

  let isUserMenuOpen = $state(false);
  let isQuickJumpOpen = $state(false);
  let quickJumpQuery = $state('');
  let userMenuRef = $state<HTMLElement | null>(null);
  let quickJumpInputEl = $state<HTMLInputElement | null>(null);

  const [send, receive] = crossfade({
    duration: 240,
    easing: cubicOut,
    fallback(node) {
      return fade(node, { duration: 140 });
    }
  });

  type NavItem = {
    href: string;
    label: string;
    description: string;
    icon: typeof Compass;
    adminOnly?: boolean;
    public?: boolean;
    match: (p: string) => boolean;
    liveCount?: () => number;
    liveActive?: () => boolean;
  };

  const navItems: NavItem[] = [
    {
      href: '/explore',
      label: 'Explore',
      description: 'Browse Nyaa releases & AniList metadata',
      icon: Compass,
      public: true,
      match: (p) => p.startsWith('/explore')
    },
    {
      href: '/',
      label: 'Dashboard',
      description: 'Overview, queue health & recent activity',
      icon: LayoutDashboard,
      match: (p) => p === '/'
    },
    {
      href: '/downloads',
      label: 'Downloads',
      description: 'Manage active torrent transfers',
      icon: DownloadCloud,
      match: (p) => p === '/downloads',
      liveCount: () => torrents.activeCount,
      liveActive: () => torrents.activeCount > 0
    },
    {
      href: '/library',
      label: 'Library',
      description: 'Organize, rename & inspect local media',
      icon: Library,
      match: (p) => p === '/library',
      liveCount: () => library.pendingCount
    },
    {
      href: '/encoder',
      label: 'Encoder',
      description: 'Transcode videos with FFmpeg presets',
      icon: Film,
      match: (p) => p === '/encoder',
      liveActive: () => Boolean(encoder.activeJob || encoder.activeTelemetry)
    },
    {
      href: '/translator',
      label: 'AI Sub',
      description: 'Extract, translate & embed subtitles',
      icon: Languages,
      match: (p) => p === '/translator',
      liveActive: () => Boolean(translator.activeJob || translator.activeTelemetry)
    },
    {
      href: '/settings',
      label: 'Settings',
      description: 'Paths, torrent limits, FFmpeg & AI config',
      icon: Settings,
      adminOnly: true,
      match: (p) => p === '/settings'
    }
  ];

  const visibleNavItems = $derived(
    navItems.filter((item) => {
      if (item.adminOnly && !(auth.isAuthenticated && auth.user?.role === 'admin')) {
        return false;
      }
      if (!auth.isAuthenticated && !item.public) {
        return false;
      }
      return true;
    })
  );

  const filteredQuickJumpItems = $derived.by(() => {
    const q = quickJumpQuery.trim().toLowerCase();
    if (!q) return visibleNavItems;
    return visibleNavItems.filter(
      (item) =>
        item.label.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q) ||
        item.href.toLowerCase().includes(q)
    );
  });

  let mobileNavEl = $state<HTMLElement | null>(null);

  $effect(() => {
    const _ = page.url.pathname;
    isUserMenuOpen = false;
    isQuickJumpOpen = false;
    if (mobileNavEl) {
      const activeLink = mobileNavEl.querySelector<HTMLElement>('[data-active="true"]');
      if (activeLink) {
        activeLink.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' });
      }
    }
  });

  $effect(() => {
    if (isQuickJumpOpen) {
      quickJumpQuery = '';
      queueMicrotask(() => quickJumpInputEl?.focus());
    }
  });

  onMount(async () => {
    theme.init();
    await auth.init();
  });

  $effect(() => {
    if (auth.loading) return;

    const pathname = page.url.pathname;

    if (auth.setupRequired) {
      if (pathname !== '/setup') {
        goto('/setup');
      }
      return;
    }

    const isPublic =
      pathname.startsWith('/explore') || pathname === '/login' || pathname === '/setup';

    if (!auth.isAuthenticated && !isPublic) {
      const redirectTarget = encodeURIComponent(pathname + (page.url.search || ''));
      goto(`/login?redirect=${redirectTarget}`);
      return;
    }

    if (auth.isAuthenticated && (pathname === '/login' || pathname === '/setup')) {
      goto('/');
      return;
    }
  });

  async function handleLogout() {
    isUserMenuOpen = false;
    await auth.logout();
    goto('/explore');
  }

  function openQuickJump() {
    isUserMenuOpen = false;
    isQuickJumpOpen = true;
  }

  function navigateQuickJump(href: string) {
    isQuickJumpOpen = false;
    goto(href);
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isQuickJumpOpen = !isQuickJumpOpen;
      isUserMenuOpen = false;
      return;
    }
    if (e.key === 'Escape') {
      isQuickJumpOpen = false;
      isUserMenuOpen = false;
    }
  }

  function handleWindowClick(e: MouseEvent) {
    if (isUserMenuOpen && userMenuRef && !userMenuRef.contains(e.target as Node)) {
      isUserMenuOpen = false;
    }
  }

  const isAuthPage = $derived(
    page.url.pathname === '/login' || page.url.pathname === '/setup'
  );
</script>

<svelte:window bind:scrollY onkeydown={handleWindowKeydown} onclick={handleWindowClick} />

{#snippet navIndicator(item: NavItem)}
  {@const count = item.liveCount ? item.liveCount() : 0}
  {@const activePulse = item.liveActive ? item.liveActive() : false}
  {#if activePulse}
    <span class="relative flex h-2 w-2 shrink-0" title="Active job running">
      <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75"></span>
      <span class="relative inline-flex rounded-full h-2 w-2 bg-cyan-500 dark:bg-cyan-400"></span>
    </span>
  {/if}
  {#if count > 0}
    <span class="px-1.5 py-0.5 rounded-full text-[9px] font-mono font-bold leading-none bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">
      {count}
    </span>
  {/if}
{/snippet}

<div class="min-h-screen bg-[#F8FAFC] dark:bg-[#0A0E17] text-slate-800 dark:text-[#EDF2F7] flex flex-col font-sans relative overflow-x-clip selection:bg-[#00E5FF]/20 selection:text-[#00E5FF] transition-colors duration-300">
  <!-- Ambient Glassmorphism Aurora Orbs -->
  <div class="pointer-events-none fixed -top-40 -left-40 w-96 h-96 rounded-full bg-cyan-400/20 dark:bg-cyan-500/15 blur-[120px] animate-aurora-1"></div>
  <div class="pointer-events-none fixed top-12 -right-40 w-[32rem] h-[32rem] rounded-full bg-purple-400/18 dark:bg-purple-600/12 blur-[140px] animate-aurora-2"></div>
  <div class="pointer-events-none fixed -bottom-40 left-1/3 w-[30rem] h-[30rem] rounded-full bg-emerald-400/18 dark:bg-emerald-500/10 blur-[130px] animate-aurora-1"></div>

  {#if !auth.loading && !isAuthPage}
    <header class="sticky top-0 z-40 w-full transition-all duration-300 {isScrolled ? 'pt-2 pb-1' : 'pt-3 sm:pt-4 pb-2'}">
      <div class="mx-auto w-[calc(100%-1.5rem)] max-w-7xl transition-all duration-300">
        <div
          class="glass-panel-elevated rounded-2xl md:rounded-full px-3.5 sm:px-4 md:px-5 flex flex-col md:flex-row md:items-center justify-between gap-2 md:gap-3 transition-all duration-300 border {isScrolled
            ? 'py-2 border-slate-300/90 dark:border-cyan-400/30 shadow-[0_16px_40px_rgba(0,0,0,0.12)] dark:shadow-[0_16px_50px_rgba(0,0,0,0.65)] backdrop-blur-2xl bg-white/95 dark:bg-[#0E1628]/92'
            : 'py-2.5 border-slate-200/80 dark:border-white/10 shadow-[0_8px_30px_rgba(0,0,0,0.06)] dark:shadow-[0_12px_40px_rgba(0,0,0,0.35)]'}"
        >
          <!-- Top Row on Mobile / Brand on Desktop -->
          <div class="flex items-center justify-between w-full md:w-auto gap-2">
            <!-- Brand -->
            <div class="flex items-center gap-2.5 shrink-0">
              <a href={auth.isAuthenticated ? '/' : '/explore'} class="flex items-center gap-2.5 group">
                <div class="relative flex items-center justify-center w-7 h-7 rounded-xl bg-gradient-to-tr from-cyan-500/20 to-indigo-500/20 border border-cyan-500/30 dark:border-cyan-400/30 shadow-inner">
                  <span class="w-2 h-2 rounded-full bg-cyan-500 dark:bg-[#00E5FF]"></span>
                  <span class="absolute w-4 h-4 rounded-full bg-cyan-500/30 dark:bg-[#00E5FF]/30 animate-ping"></span>
                </div>
                <span class="text-base sm:text-lg font-extrabold tracking-tight bg-gradient-to-r from-cyan-600 via-sky-500 to-indigo-600 dark:from-cyan-400 dark:via-sky-300 dark:to-indigo-300 bg-clip-text text-transparent group-hover:opacity-90 transition-opacity">
                  nyaalist
                </span>
              </a>

              {#if !auth.isAuthenticated}
                <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold bg-cyan-500/10 text-cyan-700 dark:text-cyan-300 border border-cyan-500/25">
                  Guest
                </span>
              {/if}
            </div>

            <!-- Mobile Right Controls (Quick Jump, Theme Cycle, User / Sign In) -->
            <div class="flex md:hidden items-center gap-1.5 shrink-0">
              <button
                type="button"
                onclick={openQuickJump}
                aria-label="Quick navigation"
                title="Quick jump (Ctrl+K)"
                class="p-1.5 rounded-full glass-pill text-slate-600 dark:text-slate-300 cursor-pointer"
              >
                <Search class="w-3.5 h-3.5" />
              </button>

              <button
                type="button"
                onclick={() => theme.cycle()}
                aria-label="Switch theme ({theme.mode})"
                title="Theme: {theme.mode} (tap to cycle)"
                class="p-1.5 rounded-full glass-pill text-slate-700 dark:text-slate-200 cursor-pointer"
              >
                {#if theme.mode === 'light'}
                  <Sun class="w-3.5 h-3.5 text-amber-500" />
                {:else if theme.mode === 'dark'}
                  <Moon class="w-3.5 h-3.5 text-cyan-400" />
                {:else}
                  <Monitor class="w-3.5 h-3.5 text-purple-400" />
                {/if}
              </button>

              {#if auth.isAuthenticated && auth.user}
                <button
                  type="button"
                  onclick={() => (isUserMenuOpen = !isUserMenuOpen)}
                  aria-label="Account menu"
                  aria-expanded={isUserMenuOpen}
                  class="flex items-center gap-1.5 pl-1 pr-2 py-0.5 rounded-full bg-slate-200/70 dark:bg-slate-950/60 border border-slate-300/80 dark:border-white/10 cursor-pointer"
                >
                  <div class="w-6 h-6 rounded-full bg-gradient-to-tr from-cyan-500 to-indigo-500 flex items-center justify-center text-[10px] font-bold text-white uppercase shadow-sm">
                    {auth.user.username.slice(0, 1)}
                  </div>
                  <ChevronDown class="w-3 h-3 text-slate-500 dark:text-slate-400 transition-transform {isUserMenuOpen ? 'rotate-180' : ''}" />
                </button>
              {:else if !auth.setupRequired}
                <a
                  href="/login"
                  class="glass-pill px-2.5 py-1 rounded-full text-xs font-semibold text-cyan-700 dark:text-cyan-300 border-cyan-500/30 flex items-center gap-1 btn-expressive"
                >
                  <LogIn class="w-3.5 h-3.5" />
                  <span>Sign In</span>
                </a>
              {/if}
            </div>
          </div>

          <!-- Desktop Center Navigation Rail -->
          <nav
            aria-label="Main navigation"
            class="hidden md:flex items-center gap-0.5 lg:gap-1 p-1 rounded-full bg-slate-100/80 dark:bg-slate-950/50 border border-slate-200/90 dark:border-white/8 backdrop-blur-md text-xs font-medium relative overflow-x-auto scrollbar-none"
          >
            {#each visibleNavItems as item (item.href)}
              {@const active = item.match(page.url.pathname)}
              <a
                href={item.href}
                aria-current={active ? 'page' : undefined}
                title={item.description}
                class="relative px-2.5 lg:px-3.5 py-1.5 rounded-full transition-colors duration-200 flex items-center gap-1.5 z-10 active:scale-95 whitespace-nowrap cursor-pointer {active
                  ? 'text-cyan-700 dark:text-cyan-300 font-semibold'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5'}"
              >
                {#if active}
                  <div
                    in:receive={{ key: 'desktop-nav-active-pill' }}
                    out:send={{ key: 'desktop-nav-active-pill' }}
                    class="absolute inset-0 rounded-full bg-cyan-500/15 dark:bg-cyan-500/20 border border-cyan-500/40 dark:border-cyan-400/40 shadow-[0_0_12px_rgba(0,229,255,0.2)] -z-10"
                  ></div>
                {/if}
                <item.icon class="w-3.5 h-3.5 shrink-0 transition-transform duration-200 {active ? 'scale-110 text-cyan-600 dark:text-cyan-300' : ''}" />
                <span>{item.label}</span>
                {@render navIndicator(item)}
              </a>
            {/each}
          </nav>

          <!-- Desktop Right Utility Cluster -->
          <div class="hidden md:flex items-center gap-2 shrink-0">
            <!-- Quick Jump Trigger -->
            <button
              type="button"
              onclick={openQuickJump}
              title="Quick Jump (Ctrl+K)"
              class="btn-expressive px-2.5 py-1.5 rounded-full glass-pill text-slate-600 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white text-[11px] font-mono flex items-center gap-1.5 cursor-pointer"
            >
              <Search class="w-3.5 h-3.5" />
              <kbd class="hidden xl:inline-block px-1.5 py-0.5 rounded bg-black/5 dark:bg-white/10 text-[9px] font-mono text-slate-500 dark:text-slate-400">
                Ctrl K
              </kbd>
            </button>

            <!-- Compact 3-Way Theme Switcher -->
            <div
              role="group"
              aria-label="Theme switcher"
              class="flex items-center p-0.5 rounded-full bg-slate-200/75 dark:bg-slate-950/60 border border-slate-300/80 dark:border-white/10 backdrop-blur-md shadow-inner"
            >
              <button
                type="button"
                onclick={() => theme.setMode('light')}
                title="Light theme"
                aria-label="Light theme"
                aria-pressed={theme.mode === 'light'}
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'light'
                  ? 'bg-amber-400 text-slate-950 shadow-sm scale-105'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Sun class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onclick={() => theme.setMode('dark')}
                title="Dark theme"
                aria-label="Dark theme"
                aria-pressed={theme.mode === 'dark'}
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'dark'
                  ? 'bg-cyan-400 text-slate-950 shadow-sm scale-105'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Moon class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onclick={() => theme.setMode('auto')}
                title="Auto theme (OS preference)"
                aria-label="Auto theme"
                aria-pressed={theme.mode === 'auto'}
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'auto'
                  ? 'bg-purple-400 text-slate-950 shadow-sm scale-105'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Monitor class="w-3.5 h-3.5" />
              </button>
            </div>

            <!-- User Menu Popover / Sign In -->
            {#if auth.isAuthenticated && auth.user}
              <div bind:this={userMenuRef} class="relative">
                <button
                  type="button"
                  onclick={() => (isUserMenuOpen = !isUserMenuOpen)}
                  aria-expanded={isUserMenuOpen}
                  aria-haspopup="menu"
                  class="flex items-center gap-2 pl-1.5 pr-2.5 py-1 rounded-full bg-slate-200/70 dark:bg-slate-950/55 border border-slate-300/80 dark:border-white/10 hover:border-cyan-500/40 transition-all cursor-pointer"
                >
                  <div class="w-6 h-6 rounded-full bg-gradient-to-tr from-cyan-500 to-indigo-500 flex items-center justify-center text-[10px] font-bold text-white uppercase shadow-sm">
                    {auth.user.username.slice(0, 1)}
                  </div>
                  <span class="text-xs font-semibold text-slate-800 dark:text-slate-100 hidden lg:inline max-w-[7rem] truncate">
                    {auth.user.username}
                  </span>
                  <ChevronDown class="w-3.5 h-3.5 text-slate-500 dark:text-slate-400 transition-transform duration-200 {isUserMenuOpen ? 'rotate-180' : ''}" />
                </button>

                {#if isUserMenuOpen}
                  <div
                    transition:scale={{ duration: 160, start: 0.95, opacity: 0, easing: cubicOut }}
                    role="menu"
                    class="absolute right-0 mt-2.5 w-60 glass-panel-elevated rounded-3xl p-2 border border-slate-200/90 dark:border-white/15 shadow-2xl z-50 origin-top-right space-y-1"
                  >
                    <div class="px-3 py-2.5 rounded-2xl bg-slate-100/80 dark:bg-slate-950/60 border border-slate-200/70 dark:border-white/8">
                      <div class="flex items-center justify-between gap-2">
                        <span class="text-xs font-bold text-slate-900 dark:text-white truncate">
                          {auth.user.username}
                        </span>
                        <span class="px-2 py-0.5 rounded-full text-[9px] font-mono font-bold uppercase bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">
                          {auth.user.role}
                        </span>
                      </div>
                      <div class="text-[10px] font-mono text-slate-500 dark:text-slate-400 mt-1">
                        nyaalist v0.1.0
                      </div>
                    </div>

                    {#if auth.user.role === 'admin'}
                      <a
                        href="/settings"
                        role="menuitem"
                        class="flex items-center justify-between px-3 py-2 rounded-2xl text-xs font-medium text-slate-700 dark:text-slate-200 hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
                      >
                        <span class="flex items-center gap-2">
                          <Shield class="w-3.5 h-3.5 text-amber-500" />
                          <span>Admin Settings</span>
                        </span>
                        <ArrowRight class="w-3 h-3 text-slate-400" />
                      </a>
                    {/if}

                    <button
                      type="button"
                      role="menuitem"
                      onclick={handleLogout}
                      class="w-full flex items-center gap-2 px-3 py-2 rounded-2xl text-xs font-semibold text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                    >
                      <LogOut class="w-3.5 h-3.5" />
                      <span>Sign Out</span>
                    </button>
                  </div>
                {/if}
              </div>
            {:else if !auth.setupRequired}
              <a
                href="/login"
                class="glass-pill px-4 py-1.5 rounded-full text-xs font-semibold text-cyan-700 dark:text-cyan-300 hover:text-cyan-900 dark:hover:text-white hover:bg-cyan-500/15 border-cyan-500/35 flex items-center gap-1.5 transition-all shadow-[0_0_15px_rgba(0,229,255,0.15)] btn-expressive"
              >
                <LogIn class="w-3.5 h-3.5" />
                <span>Sign In</span>
              </a>
            {/if}
          </div>

          <!-- Mobile Account Quick Drawer (when tapped on mobile avatar) -->
          {#if isUserMenuOpen && auth.isAuthenticated && auth.user}
            <div class="md:hidden pt-2 border-t border-slate-200/70 dark:border-white/10 flex items-center justify-between gap-2 text-xs">
              <div class="flex items-center gap-2 min-w-0">
                <span class="font-bold text-slate-900 dark:text-white truncate">{auth.user.username}</span>
                <span class="px-2 py-0.5 rounded-full text-[9px] font-mono uppercase bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">
                  {auth.user.role}
                </span>
              </div>
              <button
                type="button"
                onclick={handleLogout}
                class="px-3 py-1 rounded-full bg-rose-500/15 text-rose-600 dark:text-rose-300 border border-rose-500/30 font-semibold flex items-center gap-1.5 cursor-pointer"
              >
                <LogOut class="w-3 h-3" />
                <span>Sign Out</span>
              </button>
            </div>
          {/if}

          <!-- Mobile Navigation Sub-bar -->
          <nav
            bind:this={mobileNavEl}
            aria-label="Mobile navigation"
            class="flex md:hidden overflow-x-auto gap-1.5 pt-2 pb-0.5 scrollbar-none text-xs font-medium border-t border-slate-200/60 dark:border-white/8 w-full relative"
          >
            {#each visibleNavItems as item (item.href)}
              {@const active = item.match(page.url.pathname)}
              <a
                href={item.href}
                data-active={active}
                aria-current={active ? 'page' : undefined}
                class="relative px-3 py-1.5 rounded-full whitespace-nowrap flex items-center gap-1.5 transition-colors duration-200 z-10 active:scale-95 cursor-pointer {active
                  ? 'text-cyan-700 dark:text-cyan-300 font-semibold'
                  : 'text-slate-600 dark:text-slate-400 bg-slate-200/60 dark:bg-slate-950/45 border border-black/5 dark:border-white/5'}"
              >
                {#if active}
                  <div
                    in:receive={{ key: 'mobile-nav-active-pill' }}
                    out:send={{ key: 'mobile-nav-active-pill' }}
                    class="absolute inset-0 rounded-full bg-cyan-500/15 dark:bg-cyan-500/20 border border-cyan-500/40 shadow-[0_0_12px_rgba(0,229,255,0.15)] -z-10"
                  ></div>
                {/if}
                <item.icon class="w-3.5 h-3.5 shrink-0 transition-transform duration-200 {active ? 'scale-110 text-cyan-600 dark:text-cyan-300' : ''}" />
                <span>{item.label}</span>
                {@render navIndicator(item)}
              </a>
            {/each}
          </nav>
        </div>
      </div>
    </header>
  {/if}

  <!-- Quick Jump Command Palette (Ctrl+K) -->
  {#if isQuickJumpOpen}
    <div
      transition:fade={{ duration: 150, easing: cubicOut }}
      onclick={(e) => { if (e.target === e.currentTarget) isQuickJumpOpen = false; }}
      onkeydown={(e) => { if (e.key === 'Escape') isQuickJumpOpen = false; }}
      role="dialog"
      aria-modal="true"
      aria-label="Quick navigation palette"
      tabindex="-1"
      class="fixed inset-0 z-[70] flex items-start justify-center pt-[14vh] px-4 bg-slate-950/75 backdrop-blur-md"
    >
      <div
        transition:scale={{ duration: 190, start: 0.96, opacity: 0, easing: cubicOut }}
        class="glass-panel-elevated rounded-3xl max-w-lg w-full overflow-hidden border border-slate-200/80 dark:border-white/15 shadow-2xl"
      >
        <div class="flex items-center gap-2.5 px-4 py-3 border-b border-slate-200/70 dark:border-white/10">
          <Search class="w-4 h-4 text-cyan-600 dark:text-cyan-400 shrink-0" />
          <input
            bind:this={quickJumpInputEl}
            type="text"
            bind:value={quickJumpQuery}
            placeholder="Jump to page (Explore, Library, Encoder...)"
            onkeydown={(e) => {
              if (e.key === 'Enter' && filteredQuickJumpItems.length > 0) {
                e.preventDefault();
                navigateQuickJump(filteredQuickJumpItems[0].href);
              }
            }}
            class="w-full bg-transparent text-sm text-slate-900 dark:text-white placeholder:text-slate-400 focus:outline-none font-mono"
          />
          <button
            type="button"
            onclick={() => (isQuickJumpOpen = false)}
            class="p-1 rounded-full hover:bg-black/5 dark:hover:bg-white/10 text-slate-400 cursor-pointer"
            aria-label="Close quick jump"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <div class="p-2 max-h-80 overflow-y-auto space-y-1">
          {#if filteredQuickJumpItems.length === 0}
            <div class="py-8 text-center text-xs font-mono text-slate-500 dark:text-slate-400">
              No matching pages found.
            </div>
          {:else}
            {#each filteredQuickJumpItems as item (item.href)}
              {@const active = item.match(page.url.pathname)}
              <button
                type="button"
                onclick={() => navigateQuickJump(item.href)}
                class="w-full flex items-center justify-between gap-3 px-3.5 py-2.5 rounded-2xl text-left transition-colors cursor-pointer {active
                  ? 'bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30'
                  : 'hover:bg-black/5 dark:hover:bg-white/8 text-slate-700 dark:text-slate-200 border border-transparent'}"
              >
                <div class="flex items-center gap-3 min-w-0">
                  <div class="p-2 rounded-xl bg-slate-200/70 dark:bg-white/5 shrink-0">
                    <item.icon class="w-4 h-4" />
                  </div>
                  <div class="min-w-0">
                    <div class="text-xs font-bold flex items-center gap-2">
                      <span>{item.label}</span>
                      {@render navIndicator(item)}
                    </div>
                    <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate">
                      {item.description}
                    </div>
                  </div>
                </div>
                <ArrowRight class="w-3.5 h-3.5 text-slate-400 shrink-0" />
              </button>
            {/each}
          {/if}
        </div>

        <div class="px-4 py-2.5 border-t border-slate-200/70 dark:border-white/10 flex items-center justify-between text-[10px] font-mono text-slate-500 dark:text-slate-400 bg-slate-100/60 dark:bg-slate-950/40">
          <span>Enter to open · Esc to close</span>
          <button
            type="button"
            onclick={() => theme.cycle()}
            class="text-cyan-600 dark:text-cyan-300 hover:underline cursor-pointer"
          >
            Theme: {theme.mode}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <main class="flex-1 flex flex-col relative">
    {#key page.url.pathname}
      <div in:fade={{ duration: 180, delay: 40, easing: cubicOut }} class="flex-1 flex flex-col">
        {@render children()}
      </div>
    {/key}
  </main>
</div>
