<script lang="ts">
  import '../app.css';
  import { auth } from '#lib/stores/auth.svelte';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  let { children } = $props();

  onMount(async () => {
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
</script>

<div class="min-h-screen bg-[#121212] text-[#FFFFFF] flex flex-col font-sans">
  {#if !auth.loading}
    <header class="border-b border-[#2C2C2E] bg-[#1E1E1E]/80 backdrop-blur px-6 py-3 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="w-3 h-3 rounded-full bg-[#32D74B] animate-pulse"></div>
        <a href="/" class="text-lg font-bold font-mono tracking-tight text-white hover:text-[#00E5FF] transition-colors">nyaalist</a>
        <span class="text-[10px] font-mono uppercase bg-[#121212] text-[#00E5FF] px-2 py-0.5 rounded border border-[#2C2C2E]">v0.1.0</span>

        <nav class="hidden md:flex items-center gap-1 ml-6 text-xs font-mono">
          <a
            href="/explore"
            class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors flex items-center gap-1.5 {page.url.pathname.startsWith('/explore') ? 'text-[#00E5FF] bg-[#252525]' : ''}"
          >
            <span>Explore</span>
            <span class="text-[9px] px-1 py-0.2 rounded bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40 uppercase font-sans font-bold">Public</span>
          </a>
          <a
            href="/"
            class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors {page.url.pathname === '/' ? 'text-[#00E5FF] bg-[#252525]' : ''}"
          >
            Dashboard
          </a>
          <a
            href="/downloads"
            class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors flex items-center gap-1.5 {page.url.pathname === '/downloads' ? 'text-[#00E5FF] bg-[#252525]' : ''}"
          >
            <span>Downloads</span>
          </a>
          <a
            href="/library"
            class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors flex items-center gap-1.5 {page.url.pathname === '/library' ? 'text-[#00E5FF] bg-[#252525]' : ''}"
          >
            <span>Library</span>
          </a>
          <a
            href="/encoder"
            class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors flex items-center gap-1.5 {page.url.pathname === '/encoder' ? 'text-[#00E5FF] bg-[#252525]' : ''}"
          >
            <span>Encoder</span>
          </a>
          {#if auth.isAuthenticated && auth.user?.role === 'admin'}
            <a
              href="/settings"
              class="px-3 py-1.5 rounded-lg text-gray-300 hover:text-white hover:bg-[#252525] transition-colors flex items-center gap-1.5 {page.url.pathname === '/settings' ? 'text-[#00E5FF] bg-[#252525]' : ''}"
            >
              <span>Settings</span>
              <span class="text-[9px] px-1 py-0.2 rounded bg-amber-500/20 text-amber-400 border border-amber-500/40 uppercase font-sans font-bold">Admin</span>
            </a>
          {/if}
        </nav>
      </div>

      <div class="flex items-center gap-4 text-xs font-mono">
        {#if auth.isAuthenticated && auth.user}
          <div class="flex items-center gap-2 text-gray-300">
            <span class="text-[#98989D]">User:</span>
            <span class="text-[#00E5FF] font-bold">{auth.user.username}</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded bg-[#121212] text-gray-400 border border-[#2C2C2E] uppercase">{auth.user.role}</span>
          </div>
          <button
            onclick={handleLogout}
            class="text-[#FF453A] hover:text-red-400 transition-colors border border-[#2C2C2E] hover:border-[#FF453A] px-2.5 py-1 rounded cursor-pointer"
          >
            Logout
          </button>
        {:else if !auth.setupRequired}
          <a
            href="/login"
            class="text-[#00E5FF] hover:bg-[#00E5FF]/10 transition-colors border border-[#00E5FF]/60 px-3 py-1 rounded flex items-center gap-1"
          >
            Sign In
          </a>
        {/if}
      </div>
    </header>
  {/if}

  <main class="flex-1 flex flex-col">
    {@render children()}
  </main>
</div>
