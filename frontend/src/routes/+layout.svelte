<script lang="ts">
  import '../app.css';
  import { auth } from '#lib/stores/auth.svelte';
  import { theme } from '#lib/stores/theme.svelte';
  import { onMount } from 'svelte';
  import { crossfade, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import {
    Compass,
    LayoutDashboard,
    DownloadCloud,
    Library,
    Film,
    Settings,
    LogOut,
    LogIn,
    User,
    Sun,
    Moon,
    Monitor
  } from 'lucide-svelte';

  let { children } = $props();

  let scrollY = $state(0);
  const isScrolled = $derived(scrollY > 20);

  const [send, receive] = crossfade({
    duration: 250,
    easing: cubicOut,
    fallback(node) {
      return fade(node, { duration: 150 });
    }
  });

  const navItems = [
    { href: '/explore', label: 'Explore', icon: Compass, badge: 'Public', match: (p: string) => p.startsWith('/explore') },
    { href: '/', label: 'Dashboard', icon: LayoutDashboard, match: (p: string) => p === '/' },
    { href: '/downloads', label: 'Downloads', icon: DownloadCloud, match: (p: string) => p === '/downloads' },
    { href: '/library', label: 'Library', icon: Library, match: (p: string) => p === '/library' },
    { href: '/encoder', label: 'Encoder', icon: Film, match: (p: string) => p === '/encoder' },
    { href: '/settings', label: 'Settings', icon: Settings, badge: 'Admin', adminOnly: true, match: (p: string) => p === '/settings' }
  ];

  const visibleNavItems = $derived(
    navItems.filter(item => !item.adminOnly || (auth.isAuthenticated && auth.user?.role === 'admin'))
  );

  let mobileNavEl = $state<HTMLElement | null>(null);

  $effect(() => {
    // Scroll active link into view on mobile navigation
    const _ = page.url.pathname;
    if (mobileNavEl) {
      const activeLink = mobileNavEl.querySelector<HTMLElement>('[data-active="true"]');
      if (activeLink) {
        activeLink.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' });
      }
    }
  });

  onMount(async () => {
    theme.init();
    await auth.init();
  });

  $effect(() => {
    if (auth.loading) return;

    const pathname = page.url.pathname;

    // Check setup requirement
    if (auth.setupRequired) {
      if (pathname !== '/setup') {
        goto('/setup');
      }
      return;
    }

    // Publicly accessible paths that guest users can consume
    const isPublic = pathname.startsWith('/explore') || pathname === '/login' || pathname === '/setup';

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
    await auth.logout();
    goto('/explore');
  }

  const isAuthPage = $derived(
    page.url.pathname === '/login' || page.url.pathname === '/setup'
  );
</script>

<svelte:window bind:scrollY />

<div class="min-h-screen bg-[#F8FAFC] dark:bg-[#0A0E17] text-slate-800 dark:text-[#EDF2F7] flex flex-col font-sans relative overflow-x-clip selection:bg-[#00E5FF]/20 selection:text-[#00E5FF] transition-colors duration-300">
  <!-- Ambient Glassmorphism Aurora Orbs -->
  <div class="pointer-events-none fixed -top-40 -left-40 w-96 h-96 rounded-full bg-cyan-400/20 dark:bg-cyan-500/15 blur-[120px] animate-aurora-1"></div>
  <div class="pointer-events-none fixed top-12 -right-40 w-[32rem] h-[32rem] rounded-full bg-purple-400/18 dark:bg-purple-600/12 blur-[140px] animate-aurora-2"></div>
  <div class="pointer-events-none fixed -bottom-40 left-1/3 w-[30rem] h-[30rem] rounded-full bg-emerald-400/18 dark:bg-emerald-500/10 blur-[130px] animate-aurora-1"></div>

  {#if !auth.loading && !isAuthPage}
    <header class="sticky top-0 z-40 w-full transition-all duration-300 {isScrolled ? 'pt-2 pb-1' : 'pt-3 sm:pt-4 pb-2'}">
      <div class="mx-auto w-[calc(100%-1.5rem)] max-w-7xl transition-all duration-300">
        <div class="glass-panel-elevated rounded-2xl md:rounded-full px-3.5 sm:px-4 md:px-6 flex flex-col md:flex-row md:items-center justify-between gap-2.5 md:gap-4 transition-all duration-300 border {isScrolled ? 'py-2 border-slate-300/90 dark:border-cyan-400/30 shadow-[0_16px_40px_rgba(0,0,0,0.12)] dark:shadow-[0_16px_50px_rgba(0,0,0,0.6)] backdrop-blur-2xl bg-white/95 dark:bg-[#101726]/90' : 'py-2.5 sm:py-3 border-slate-200/80 dark:border-white/10 shadow-[0_8px_30px_rgba(0,0,0,0.06)] dark:shadow-[0_12px_40px_rgba(0,0,0,0.35)]'}">
          <!-- Top Row on Mobile / Full Row on Desktop -->
          <div class="flex items-center justify-between w-full md:w-auto gap-2 sm:gap-3">
            <!-- Brand & Version -->
            <div class="flex items-center gap-2.5 sm:gap-3 shrink-0">
              <div class="relative flex items-center justify-center">
                <span class="w-2.5 h-2.5 rounded-full bg-cyan-500 dark:bg-[#00E5FF] animate-pulse"></span>
                <span class="absolute w-5 h-5 rounded-full bg-cyan-500/25 dark:bg-[#00E5FF]/25 animate-ping"></span>
              </div>

              <a href="/" class="flex items-center gap-2 group">
                <span class="text-base sm:text-lg font-extrabold tracking-tight bg-gradient-to-r from-cyan-600 via-sky-500 to-indigo-600 dark:from-cyan-400 dark:via-sky-300 dark:to-indigo-300 bg-clip-text text-transparent group-hover:opacity-90 transition-opacity">
                  nyaalist
                </span>
              </a>

              <span class="hidden sm:inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-500/20 dark:border-cyan-400/20">
                v0.1.0
              </span>
            </div>

            <!-- Mobile Controls (Theme Switcher & User Status) -->
            <div class="flex md:hidden items-center gap-1.5 sm:gap-2 shrink-0">
              <!-- Compact Theme Switcher -->
              <div class="flex items-center p-0.5 rounded-full bg-slate-200/70 dark:bg-slate-950/60 border border-slate-300/80 dark:border-white/10 backdrop-blur-md shadow-inner">
                <button
                  type="button"
                  onclick={() => theme.setMode('light')}
                  title="Light theme"
                  class="p-1 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'light'
                    ? 'bg-amber-400 text-slate-950 shadow-sm font-bold scale-105'
                    : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
                >
                  <Sun class="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  onclick={() => theme.setMode('dark')}
                  title="Dark theme"
                  class="p-1 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'dark'
                    ? 'bg-cyan-400 text-slate-950 shadow-sm font-bold scale-105'
                    : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
                >
                  <Moon class="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  onclick={() => theme.setMode('auto')}
                  title="Auto theme"
                  class="p-1 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'auto'
                    ? 'bg-purple-400 text-slate-950 shadow-sm font-bold scale-105'
                    : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
                >
                  <Monitor class="w-3.5 h-3.5" />
                </button>
              </div>

              {#if auth.isAuthenticated && auth.user}
                <div class="flex items-center gap-1.5 pl-1.5 pr-1 py-0.5 rounded-full bg-slate-200/60 dark:bg-slate-950/40 border border-slate-300 dark:border-white/10 backdrop-blur-md">
                  <div class="w-5 h-5 rounded-full bg-gradient-to-tr from-cyan-500 to-indigo-500 flex items-center justify-center text-[9px] font-bold text-black uppercase shadow-sm">
                    {auth.user.username.slice(0, 1)}
                  </div>
                  <button
                    onclick={handleLogout}
                    title="Sign out"
                    class="w-6 h-6 rounded-full bg-rose-500/10 hover:bg-rose-500/20 text-rose-500 dark:text-rose-400 border border-rose-500/20 hover:border-rose-500/40 flex items-center justify-center transition-all cursor-pointer"
                  >
                    <LogOut class="w-3 h-3" />
                  </button>
                </div>
              {:else if !auth.setupRequired}
                <a
                  href="/login"
                  class="glass-pill px-2.5 py-1 rounded-full text-xs font-semibold text-cyan-600 dark:text-cyan-300 hover:text-cyan-800 dark:hover:text-white hover:bg-cyan-500/15 border-cyan-400/30 flex items-center gap-1 transition-all shadow-[0_0_12px_rgba(0,229,255,0.15)] btn-expressive"
                >
                  <LogIn class="w-3.5 h-3.5" />
                  <span>Sign In</span>
                </a>
              {/if}
            </div>
          </div>

          <!-- M3 Expressive Navigation Pills with Svelte Transitions -->
          <nav class="hidden md:flex items-center gap-1 p-1 rounded-full bg-slate-100/70 dark:bg-slate-950/40 border border-slate-200 dark:border-white/5 backdrop-blur-md text-xs font-medium relative">
            {#each visibleNavItems as item (item.href)}
              {@const active = item.match(page.url.pathname)}
              <a
                href={item.href}
                class="relative px-3.5 py-1.5 rounded-full transition-colors duration-200 flex items-center gap-2 z-10 active:scale-95 cursor-pointer {active
                  ? 'text-cyan-700 dark:text-cyan-300 font-semibold'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5'}"
              >
                {#if active}
                  <div
                    in:receive={{ key: 'desktop-nav-active-pill' }}
                    out:send={{ key: 'desktop-nav-active-pill' }}
                    class="absolute inset-0 rounded-full bg-cyan-500/15 dark:bg-cyan-500/20 border border-cyan-400/40 shadow-[0_0_12px_rgba(0,229,255,0.2)] -z-10"
                  ></div>
                {/if}
                <item.icon class="w-3.5 h-3.5 transition-transform duration-200 {active ? 'scale-110 text-cyan-600 dark:text-cyan-300' : ''}" />
                <span>{item.label}</span>
                {#if item.badge}
                  <span class="text-[9px] px-1.5 py-0.2 rounded-full font-bold uppercase tracking-wider {item.badge === 'Admin' ? 'bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-amber-500/40' : 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-400/30'}">
                    {item.badge}
                  </span>
                {/if}
              </a>
            {/each}
          </nav>

          <!-- Desktop Right Side: Theme Switcher & User Status -->
          <div class="hidden md:flex items-center gap-2 sm:gap-3">
            <!-- 3-Way Theme Switcher (Dark / Light / Auto) -->
            <div class="flex items-center p-0.5 rounded-full bg-slate-200/70 dark:bg-slate-950/60 border border-slate-300/80 dark:border-white/10 backdrop-blur-md shadow-inner">
              <button
                type="button"
                onclick={() => theme.setMode('light')}
                title="Light theme"
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'light'
                  ? 'bg-amber-400 text-slate-950 shadow-sm font-bold scale-105'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Sun class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onclick={() => theme.setMode('dark')}
                title="Dark theme"
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center justify-center {theme.mode === 'dark'
                  ? 'bg-cyan-400 text-slate-950 shadow-sm font-bold scale-105'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Moon class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onclick={() => theme.setMode('auto')}
                title="Auto theme (matches OS preference)"
                class="p-1.5 rounded-full transition-all cursor-pointer flex items-center gap-1 {theme.mode === 'auto'
                  ? 'bg-purple-400 text-slate-950 shadow-sm font-bold scale-105 px-2'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white'}"
              >
                <Monitor class="w-3.5 h-3.5" />
                {#if theme.mode === 'auto'}
                  <span class="text-[9px] font-mono uppercase font-bold tracking-tight">Auto</span>
                {/if}
              </button>
            </div>

            {#if auth.isAuthenticated && auth.user}
              <div class="flex items-center gap-2 pl-2 pr-1 py-1 rounded-full bg-slate-200/60 dark:bg-slate-950/40 border border-slate-300 dark:border-white/10 backdrop-blur-md">
                <div class="w-6 h-6 rounded-full bg-gradient-to-tr from-cyan-500 to-indigo-500 flex items-center justify-center text-[10px] font-bold text-black uppercase shadow-sm">
                  {auth.user.username.slice(0, 1)}
                </div>
                <span class="text-xs font-semibold text-slate-700 dark:text-slate-200 hidden sm:inline">{auth.user.username}</span>
                <span class="text-[9px] px-1.5 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-slate-600 dark:text-slate-300 font-mono uppercase">
                  {auth.user.role}
                </span>
                <button
                  onclick={handleLogout}
                  title="Sign out"
                  class="w-7 h-7 rounded-full bg-rose-500/10 hover:bg-rose-500/20 text-rose-500 dark:text-rose-400 border border-rose-500/20 hover:border-rose-500/40 flex items-center justify-center transition-all cursor-pointer btn-expressive ml-0.5"
                >
                  <LogOut class="w-3.5 h-3.5" />
                </button>
              </div>
            {:else if !auth.setupRequired}
              <a
                href="/login"
                class="glass-pill px-4 py-1.5 rounded-full text-xs font-semibold text-cyan-600 dark:text-cyan-300 hover:text-cyan-800 dark:hover:text-white hover:bg-cyan-500/15 border-cyan-400/30 flex items-center gap-1.5 transition-all shadow-[0_0_15px_rgba(0,229,255,0.15)] btn-expressive"
              >
                <LogIn class="w-3.5 h-3.5" />
                <span>Sign In</span>
              </a>
            {/if}
          </div>

          <!-- Mobile Navigation Sub-bar: integrated with Svelte Transitions -->
          <div
            bind:this={mobileNavEl}
            class="flex md:hidden overflow-x-auto gap-1.5 pt-2 pb-0.5 scrollbar-none text-xs font-medium border-t border-slate-200/60 dark:border-white/5 w-full relative"
          >
            {#each visibleNavItems as item (item.href)}
              {@const active = item.match(page.url.pathname)}
              <a
                href={item.href}
                data-active={active}
                class="relative px-3 py-1.5 rounded-full whitespace-nowrap flex items-center gap-1.5 transition-colors duration-200 z-10 active:scale-95 cursor-pointer {active
                  ? 'text-cyan-700 dark:text-cyan-300 font-semibold'
                  : 'text-slate-600 dark:text-slate-400 bg-slate-200/70 dark:bg-slate-950/40 border border-black/5 dark:border-white/5'}"
              >
                {#if active}
                  <div
                    in:receive={{ key: 'mobile-nav-active-pill' }}
                    out:send={{ key: 'mobile-nav-active-pill' }}
                    class="absolute inset-0 rounded-full bg-cyan-500/20 border border-cyan-400/40 shadow-[0_0_12px_rgba(0,229,255,0.15)] -z-10"
                  ></div>
                {/if}
                <item.icon class="w-3.5 h-3.5 transition-transform duration-200 {active ? 'scale-110 text-cyan-600 dark:text-cyan-300' : ''}" />
                <span>{item.label}</span>
              </a>
            {/each}
          </div>
        </div>
      </div>
    </header>
  {/if}

  <main class="flex-1 flex flex-col relative">
    {#key page.url.pathname}
      <div in:fade={{ duration: 180, delay: 40, easing: cubicOut }} class="flex-1 flex flex-col">
        {@render children()}
      </div>
    {/key}
  </main>
</div>
