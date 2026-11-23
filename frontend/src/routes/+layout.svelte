<script lang="ts">
  import '../app.css';
  import { auth } from '#lib/stores/auth.svelte';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  let { children } = $props();

  onMount(async () => {
    await auth.init();

    // Check setup requirement
    if (auth.setupRequired && page.url.pathname !== '/setup') {
      goto('/setup');
    }
  });

  async function handleLogout() {
    await auth.logout();
    goto('/login');
  }
</script>

<div class="min-h-screen bg-[#121212] text-[#FFFFFF] flex flex-col font-sans">
  {#if !auth.loading}
    <header class="border-b border-[#2C2C2E] bg-[#1E1E1E]/80 backdrop-blur px-6 py-3 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="w-3 h-3 rounded-full bg-[#32D74B] animate-pulse"></div>
        <a href="/" class="text-lg font-bold font-mono tracking-tight text-white hover:text-[#00E5FF] transition-colors">nyaalist</a>
        <span class="text-[10px] font-mono uppercase bg-[#121212] text-[#00E5FF] px-2 py-0.5 rounded border border-[#2C2C2E]">v0.1.0</span>
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
            class="text-[#00E5FF] hover:text-cyan-300 transition-colors border border-[#2C2C2E] px-2.5 py-1 rounded"
          >
            Login
          </a>
        {/if}
      </div>
    </header>
  {/if}

  <main class="flex-1 flex flex-col">
    {@render children()}
  </main>
</div>
