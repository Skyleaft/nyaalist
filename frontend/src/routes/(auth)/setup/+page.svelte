<script lang="ts">
  import { auth } from '#lib/stores/auth.svelte';
  import { goto } from '$app/navigation';

  let username = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let validationError = $state<string | null>(null);

  async function handleSetup(e: Event) {
    e.preventDefault();
    validationError = null;

    if (username.trim().length < 3) {
      validationError = 'Username must be at least 3 characters long.';
      return;
    }

    if (password.length < 8) {
      validationError = 'Password must be at least 8 characters long.';
      return;
    }

    if (password !== confirmPassword) {
      validationError = 'Passwords do not match.';
      return;
    }

    try {
      await auth.setup(username.trim(), password);
      goto('/');
    } catch (err: any) {
      validationError = err.message || 'Failed to complete initial setup.';
    }
  }
</script>

<svelte:head>
  <title>Initial Setup - Nyaalist</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center p-4 bg-[#121212]">
  <div class="w-full max-w-md bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-8 shadow-2xl space-y-6">
    <div class="space-y-2 border-b border-[#2C2C2E] pb-4">
      <div class="flex items-center gap-2">
        <div class="w-3 h-3 rounded-full bg-[#00E5FF] animate-pulse"></div>
        <h1 class="text-xl font-bold tracking-tight text-white font-mono">nyaalist::setup</h1>
      </div>
      <p class="text-xs text-[#98989D]">
        First-time startup detected. Create the primary administrator account to secure your instance.
      </p>
    </div>

    {#if validationError || auth.error}
      <div class="bg-[#3A1C1C] border-l-4 border-[#FF453A] p-4 rounded-r-lg text-sm text-[#FF453A] font-mono">
        {validationError || auth.error}
      </div>
    {/if}

    <form onsubmit={handleSetup} class="space-y-4 text-sm">
      <div class="space-y-1">
        <label for="username" class="block text-xs font-mono uppercase text-[#98989D]">Admin Username</label>
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
        <label for="password" class="block text-xs font-mono uppercase text-[#98989D]">Master Password</label>
        <input
          id="password"
          type="password"
          bind:value={password}
          required
          placeholder="••••••••••••"
          autocomplete="new-password"
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] font-mono transition-colors"
        />
        <span class="text-[10px] text-[#98989D]">Minimum 8 characters (Argon2id protected)</span>
      </div>

      <div class="space-y-1">
        <label for="confirmPassword" class="block text-xs font-mono uppercase text-[#98989D]">Confirm Password</label>
        <input
          id="confirmPassword"
          type="password"
          bind:value={confirmPassword}
          required
          placeholder="••••••••••••"
          autocomplete="new-password"
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-4 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] font-mono transition-colors"
        />
      </div>

      <button
        type="submit"
        disabled={auth.loading}
        class="w-full bg-[#00E5FF] hover:bg-[#00c8e0] text-[#121212] font-semibold font-mono py-2.5 px-4 rounded-xl shadow-lg hover:shadow-[#00E5FF]/20 transition-all cursor-pointer disabled:opacity-50"
      >
        {auth.loading ? 'Initializing Database...' : 'Initialize Administrator Account'}
      </button>
    </form>
  </div>
</div>
