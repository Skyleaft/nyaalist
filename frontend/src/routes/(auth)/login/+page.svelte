<script lang="ts">
  import { auth } from '#lib/stores/auth.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';

  let username = $state('');
  let password = $state('');
  let loginError = $state<string | null>(null);

  async function handleLogin(e: Event) {
    e.preventDefault();
    loginError = null;

    try {
      await auth.login(username.trim(), password);
      const redirect = page.url.searchParams.get('redirect') || '/';
      goto(redirect);
    } catch (err: any) {
      loginError = err.message || 'Invalid username or password.';
    }
  }
</script>

<svelte:head>
  <title>Login - Nyaalist</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center p-4 bg-[#121212]">
  <div class="w-full max-w-md bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-8 shadow-2xl space-y-6">
    <div class="space-y-2 border-b border-[#2C2C2E] pb-4">
      <div class="flex items-center gap-2">
        <div class="w-3 h-3 rounded-full bg-[#32D74B] animate-pulse"></div>
        <h1 class="text-xl font-bold tracking-tight text-white font-mono">nyaalist::login</h1>
      </div>
      <p class="text-xs text-[#98989D]">
        Enter your credentials to access the anime torrent explorer & downloader dashboard.
      </p>
    </div>

    {#if loginError || auth.error}
      <div class="bg-[#3A1C1C] border-l-4 border-[#FF453A] p-4 rounded-r-lg text-sm text-[#FF453A] font-mono">
        {loginError || auth.error}
      </div>
    {/if}

    <form onsubmit={handleLogin} class="space-y-4 text-sm">
      <div class="space-y-1">
        <label for="username" class="block text-xs font-mono uppercase text-[#98989D]">Username</label>
        <input
          id="username"
          type="text"
          bind:value={username}
          required
          placeholder="admin"
          autocomplete="username"
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] font-mono transition-colors"
        />
      </div>

      <div class="space-y-1">
        <label for="password" class="block text-xs font-mono uppercase text-[#98989D]">Password</label>
        <input
          id="password"
          type="password"
          bind:value={password}
          required
          placeholder="••••••••••••"
          autocomplete="current-password"
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] font-mono transition-colors"
        />
      </div>

      <button
        type="submit"
        disabled={auth.loading}
        class="w-full bg-[#00E5FF] hover:bg-[#00c8e0] text-[#121212] font-semibold font-mono py-2.5 px-4 rounded-xl shadow-lg hover:shadow-[#00E5FF]/20 transition-all cursor-pointer disabled:opacity-50"
      >
        {auth.loading ? 'Authenticating...' : 'Sign In'}
      </button>
    </form>

    <div class="pt-4 border-t border-[#2C2C2E] flex items-center justify-between text-xs font-mono text-[#98989D]">
      <span>Guest visitor?</span>
      <a href="/explore" class="text-[#00E5FF] hover:text-cyan-300 transition-colors flex items-center gap-1">
        Browse releases publicly &rarr;
      </a>
    </div>
  </div>
</div>
