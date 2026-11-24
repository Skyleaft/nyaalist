<script lang="ts">
  import { auth } from '#lib/stores/auth.svelte';
  import { goto } from '$app/navigation';
  import { Shield, Lock, User, Sparkles, AlertCircle } from 'lucide-svelte';

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

<div class="min-h-screen flex items-center justify-center p-4 relative overflow-hidden bg-[var(--bg-primary)]">
  <!-- Ambient Glassmorphism Orbs -->
  <div class="pointer-events-none absolute -top-32 -left-32 w-80 h-80 rounded-full bg-cyan-500/20 blur-[100px] animate-aurora-1"></div>
  <div class="pointer-events-none absolute -bottom-32 -right-32 w-80 h-80 rounded-full bg-emerald-500/20 blur-[100px] animate-aurora-2"></div>

  <div class="w-full max-w-md glass-panel-elevated rounded-3xl p-8 sm:p-9 shadow-2xl space-y-6 relative border border-slate-200/80 dark:border-white/15 animate-scale-in z-10">
    <div class="space-y-2 border-b border-slate-200/80 dark:border-white/5 pb-4">
      <div class="flex items-center gap-2.5">
        <div class="relative flex items-center justify-center">
          <div class="w-3 h-3 rounded-full bg-cyan-500 dark:bg-cyan-400 animate-pulse"></div>
          <div class="absolute w-5 h-5 rounded-full bg-cyan-500/30 dark:bg-cyan-400/30 animate-ping"></div>
        </div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white font-mono">nyaalist::setup</h1>
      </div>
      <p class="text-xs text-slate-600 dark:text-slate-400">
        First-time startup detected. Create the primary administrator account to secure your instance.
      </p>
    </div>

    {#if validationError || auth.error}
      <div class="p-3.5 bg-rose-500/10 border border-rose-500/40 rounded-2xl text-xs font-mono text-rose-600 dark:text-rose-300 flex items-center gap-2">
        <AlertCircle class="w-4 h-4 text-rose-500 dark:text-rose-400 shrink-0" />
        <span>{validationError || auth.error}</span>
      </div>
    {/if}

    <form onsubmit={handleSetup} class="space-y-4 text-sm font-mono">
      <div class="space-y-1.5">
        <label for="username" class="block text-xs uppercase text-slate-600 dark:text-slate-400 font-bold">Admin Username</label>
        <div class="relative">
          <User class="w-4 h-4 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            id="username"
            type="text"
            bind:value={username}
            required
            placeholder="admin"
            autocomplete="username"
            class="glass-input w-full rounded-2xl pl-10 pr-4 py-2.5"
          />
        </div>
      </div>

      <div class="space-y-1.5">
        <label for="password" class="block text-xs uppercase text-slate-600 dark:text-slate-400 font-bold">Master Password</label>
        <div class="relative">
          <Lock class="w-4 h-4 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            id="password"
            type="password"
            bind:value={password}
            required
            placeholder="••••••••••••"
            autocomplete="new-password"
            class="glass-input w-full rounded-2xl pl-10 pr-4 py-2.5"
          />
        </div>
        <span class="text-[10px] text-slate-500 dark:text-slate-400 block pl-1">Minimum 8 characters (Argon2id protected)</span>
      </div>

      <div class="space-y-1.5">
        <label for="confirmPassword" class="block text-xs uppercase text-slate-600 dark:text-slate-400 font-bold">Confirm Password</label>
        <div class="relative">
          <Lock class="w-4 h-4 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            id="confirmPassword"
            type="password"
            bind:value={confirmPassword}
            required
            placeholder="••••••••••••"
            autocomplete="new-password"
            class="glass-input w-full rounded-2xl pl-10 pr-4 py-2.5"
          />
        </div>
      </div>

      <button
        type="submit"
        disabled={auth.loading}
        class="btn-expressive w-full bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white font-bold font-mono py-3 px-4 rounded-2xl shadow-[0_0_20px_rgba(0,229,255,0.25)] transition-all cursor-pointer disabled:opacity-40 flex items-center justify-center gap-2 mt-2"
      >
        {#if auth.loading}
          <div class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
          <span>Initializing Database...</span>
        {:else}
          <Sparkles class="w-4 h-4 stroke-[2.5]" />
          <span>Initialize Administrator</span>
        {/if}
      </button>
    </form>
  </div>
</div>
