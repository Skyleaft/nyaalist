<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '#lib/api/client';
  import { auth } from '#lib/stores/auth.svelte';
  import { theme } from '#lib/stores/theme.svelte';
  import type { AppConfigDto, PathValidationResponse } from '#lib/api/types';
  import {
    Settings,
    HardDrive,
    DownloadCloud,
    Compass,
    Film,
    Check,
    AlertCircle,
    Save,
    CheckCircle2,
    ShieldAlert,
    Sun,
    Moon,
    Monitor,
    Palette,
    Languages,
    KeyRound
  } from 'lucide-svelte';

  let config = $state<AppConfigDto | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let saveSuccess = $state(false);
  let errorMessage = $state<string | null>(null);

  // Google AI Studio Translator key input
  let geminiApiKeyInput = $state('');
  let clearGeminiKey = $state(false);
  let testingGeminiKey = $state(false);
  let geminiTestResult = $state<{ ok: boolean; text: string } | null>(null);

  // Path validation states
  let validating = $state<Record<string, boolean>>({});
  let validationResults = $state<Record<string, PathValidationResponse | null>>({});

  onMount(async () => {
    if (auth.user?.role === 'admin') {
      await fetchConfig();
    } else {
      loading = false;
    }
  });

  async function fetchConfig() {
    loading = true;
    errorMessage = null;
    try {
      config = await api.config.get();
    } catch (err: any) {
      errorMessage = err.message || 'Failed to load configuration';
    } finally {
      loading = false;
    }
  }

  async function validateDirectory(key: string, path: string) {
    if (!path || !path.trim()) return;
    validating[key] = true;
    try {
      const res = await api.config.validatePath(path.trim());
      validationResults[key] = res;
    } catch (err: any) {
      validationResults[key] = {
        path,
        exists: false,
        is_dir: false,
        writable: false
      };
    } finally {
      validating[key] = false;
    }
  }

  async function testGeminiConnection() {
    if (!config) return;
    testingGeminiKey = true;
    geminiTestResult = null;
    try {
      const res = await api.translator.testKey({
        api_key: geminiApiKeyInput.trim() || undefined,
        model: config.gemini_model
      });
      geminiTestResult = { ok: true, text: res.message };
    } catch (err: any) {
      geminiTestResult = { ok: false, text: err.message || 'Failed to verify Google AI Studio API key' };
    } finally {
      testingGeminiKey = false;
    }
  }

  async function saveConfig() {
    if (!config) return;
    saving = true;
    saveSuccess = false;
    errorMessage = null;

    try {
      await api.config.update({
        downloads_dir: config.downloads_dir,
        library_dir: config.library_dir,
        nyaa_default_category: config.nyaa_default_category,
        torrent_global_download_limit_kbps: Number(config.torrent_global_download_limit_kbps),
        torrent_global_upload_limit_kbps: Number(config.torrent_global_upload_limit_kbps),
        encoder_default_preset: config.encoder_default_preset,
        gemini_api_key: geminiApiKeyInput.trim() || undefined,
        clear_gemini_api_key: clearGeminiKey || undefined,
        gemini_model: config.gemini_model
      });
      geminiApiKeyInput = '';
      clearGeminiKey = false;
      await fetchConfig();
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
      }, 4000);
    } catch (err: any) {
      errorMessage = err.message || 'Failed to save configuration';
    } finally {
      saving = false;
    }
  }
</script>

<svelte:head>
  <title>Settings - Nyaalist</title>
</svelte:head>

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-6xl mx-auto w-full space-y-6">
  {#if auth.user && auth.user.role !== 'admin'}
    <div class="glass-panel border border-rose-500/40 rounded-3xl p-10 text-center space-y-4 max-w-2xl mx-auto mt-12 bg-rose-500/10 dark:bg-rose-950/20 shadow-2xl">
      <div class="w-14 h-14 mx-auto rounded-full bg-rose-500/20 flex items-center justify-center text-rose-500 dark:text-rose-400">
        <ShieldAlert class="w-7 h-7" />
      </div>
      <h2 class="text-xl font-bold font-mono text-slate-900 dark:text-white">Administrator Privileges Required</h2>
      <p class="text-sm text-slate-600 dark:text-slate-400 max-w-md mx-auto">
        Modifying server storage directories, BitTorrent limits, and execution paths requires an administrator account.
      </p>
      <div class="pt-4">
        <a href="/" class="btn-expressive inline-flex items-center gap-2 px-5 py-2.5 glass-pill text-slate-700 dark:text-white rounded-2xl font-mono text-xs">
          &larr; Return to Dashboard
        </a>
      </div>
    </div>
  {:else}
    <!-- Top Bar -->
    <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-slate-200/80 dark:border-white/10">
      <div class="flex items-center gap-4">
        <div class="p-3.5 rounded-2xl bg-amber-500/10 text-amber-500 dark:text-amber-300 border border-amber-500/25 shadow-[0_0_20px_rgba(245,158,11,0.2)]">
          <Settings class="w-7 h-7" />
        </div>
        <div>
          <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white">
            System Settings
          </h1>
          <p class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-0.5">
            Configure storage paths, BitTorrent bandwidth caps, scraping parameters, and video presets
          </p>
        </div>
      </div>

      <div class="flex items-center gap-3">
        {#if saveSuccess}
          <div class="text-xs font-mono text-emerald-600 dark:text-emerald-300 bg-emerald-500/10 border border-emerald-500/30 px-3.5 py-2 rounded-2xl flex items-center gap-2">
            <Check class="w-4 h-4 stroke-[2.5]" />
            <span>Settings saved successfully</span>
          </div>
        {/if}

        <button
          onclick={saveConfig}
          disabled={saving || loading || !config}
          class="btn-expressive bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 disabled:opacity-40 text-white text-xs font-mono font-bold px-5 py-2.5 rounded-2xl transition-all flex items-center gap-2 cursor-pointer shadow-[0_0_20px_rgba(0,229,255,0.25)]"
        >
          {#if saving}
            <div class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
            <span>SAVING...</span>
          {:else}
            <Save class="w-4 h-4 stroke-[2.5]" />
            <span>SAVE CHANGES</span>
          {/if}
        </button>
      </div>
    </div>

    {#if errorMessage}
      <div class="glass-panel border-l-4 border-rose-500 p-4 rounded-2xl text-xs font-mono text-rose-300 bg-rose-950/20 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <AlertCircle class="w-4 h-4 text-rose-400" />
          <span>{errorMessage}</span>
        </div>
        <button onclick={() => (errorMessage = null)} class="text-rose-400 hover:text-white cursor-pointer font-bold">✕</button>
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center p-20 glass-panel rounded-3xl border border-white/10">
        <div class="w-8 h-8 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
      </div>
    {:else if config}
      <div class="space-y-6">
        <!-- Section 0: Theme & Visual Appearance -->
        <div class="glass-panel rounded-3xl p-6 space-y-4 border border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-800 dark:text-white flex items-center gap-2">
                <Palette class="w-4 h-4 text-cyan-500 dark:text-cyan-400" />
                <span>Interface Theme & Appearance</span>
              </h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Switch between dark, light, or automatic system theme preferences</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-400/25 uppercase font-bold">
              Current: {theme.mode}
            </span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3.5 pt-1">
            <button
              type="button"
              onclick={() => theme.setMode('dark')}
              class="btn-expressive p-4 rounded-2xl glass-card-interactive border text-left cursor-pointer transition-all flex items-start gap-3.5 {theme.mode === 'dark'
                ? 'border-cyan-400/50 bg-cyan-500/15 shadow-[0_0_20px_rgba(0,229,255,0.2)]'
                : 'border-slate-200/60 dark:border-white/10'}"
            >
              <div class="p-2.5 rounded-xl bg-slate-900 text-cyan-400 border border-cyan-400/30">
                <Moon class="w-5 h-5" />
              </div>
              <div class="space-y-1">
                <div class="text-sm font-bold text-slate-800 dark:text-white flex items-center gap-2">
                  <span>Dark Mode</span>
                  {#if theme.mode === 'dark'}
                    <span class="w-2 h-2 rounded-full bg-cyan-400"></span>
                  {/if}
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400">Deep cosmic dark canvas with glowing neon glassmorphism</p>
              </div>
            </button>

            <button
              type="button"
              onclick={() => theme.setMode('light')}
              class="btn-expressive p-4 rounded-2xl glass-card-interactive border text-left cursor-pointer transition-all flex items-start gap-3.5 {theme.mode === 'light'
                ? 'border-amber-400/50 bg-amber-500/15 shadow-[0_0_20px_rgba(245,158,11,0.2)]'
                : 'border-slate-200/60 dark:border-white/10'}"
            >
              <div class="p-2.5 rounded-xl bg-amber-100 text-amber-600 border border-amber-300/40">
                <Sun class="w-5 h-5" />
              </div>
              <div class="space-y-1">
                <div class="text-sm font-bold text-slate-800 dark:text-white flex items-center gap-2">
                  <span>Light Mode</span>
                  {#if theme.mode === 'light'}
                    <span class="w-2 h-2 rounded-full bg-amber-500"></span>
                  {/if}
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400">Clean, crisp frosted glass with soft radiant pastel accents</p>
              </div>
            </button>

            <button
              type="button"
              onclick={() => theme.setMode('auto')}
              class="btn-expressive p-4 rounded-2xl glass-card-interactive border text-left cursor-pointer transition-all flex items-start gap-3.5 {theme.mode === 'auto'
                ? 'border-purple-400/50 bg-purple-500/15 shadow-[0_0_20px_rgba(168,85,247,0.2)]'
                : 'border-slate-200/60 dark:border-white/10'}"
            >
              <div class="p-2.5 rounded-xl bg-purple-100 dark:bg-purple-950/60 text-purple-600 dark:text-purple-300 border border-purple-400/30">
                <Monitor class="w-5 h-5" />
              </div>
              <div class="space-y-1">
                <div class="text-sm font-bold text-slate-800 dark:text-white flex items-center gap-2">
                  <span>Auto (System)</span>
                  {#if theme.mode === 'auto'}
                    <span class="w-2 h-2 rounded-full bg-purple-400"></span>
                  {/if}
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400">Dynamically synchronizes with your device's OS preference ({theme.resolved})</p>
              </div>
            </button>
          </div>
        </div>
        <!-- Section 1: Storage Paths -->
        <div class="glass-panel rounded-3xl p-6 space-y-5 border border-slate-200/80 dark:border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-slate-200/80 dark:border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">Storage Directories</h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Filesystem target directories for torrent staging and library storage</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-400/25">PATH ENGINES</span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
            <!-- Downloads Dir -->
            <div class="space-y-2">
              <label for="input-dl-dir" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Downloads Directory</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">Staging area</span>
              </label>
              <div class="flex gap-2">
                <input
                  id="input-dl-dir"
                  type="text"
                  bind:value={config.downloads_dir}
                  class="glass-input flex-1 rounded-2xl px-3.5 py-2 text-xs font-mono"
                  placeholder="/data/downloads"
                />
                <button
                  type="button"
                  onclick={() => validateDirectory('downloads', config?.downloads_dir || '')}
                  disabled={validating['downloads']}
                  class="btn-expressive glass-pill text-xs font-mono px-3.5 py-2 rounded-2xl text-slate-700 dark:text-slate-200 hover:text-slate-900 dark:hover:text-white cursor-pointer"
                >
                  {validating['downloads'] ? '...' : 'Validate'}
                </button>
              </div>
              {#if validationResults['downloads']}
                <div class="flex items-center gap-2 text-[11px] font-mono mt-1">
                  {#if validationResults['downloads'].writable}
                    <span class="text-emerald-500 dark:text-emerald-400">✓ Writable directory verified</span>
                  {:else if validationResults['downloads'].exists}
                    <span class="text-amber-500 dark:text-amber-400">⚠ Exists but not writable</span>
                  {:else}
                    <span class="text-rose-500 dark:text-rose-400">✗ Directory does not exist</span>
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Library Dir -->
            <div class="space-y-2">
              <label for="input-lib-dir" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Organized Library Directory</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">Media root</span>
              </label>
              <div class="flex gap-2">
                <input
                  id="input-lib-dir"
                  type="text"
                  bind:value={config.library_dir}
                  class="glass-input flex-1 rounded-2xl px-3.5 py-2 text-xs font-mono"
                  placeholder="/data/library"
                />
                <button
                  type="button"
                  onclick={() => validateDirectory('library', config?.library_dir || '')}
                  disabled={validating['library']}
                  class="btn-expressive glass-pill text-xs font-mono px-3.5 py-2 rounded-2xl text-slate-700 dark:text-slate-200 hover:text-slate-900 dark:hover:text-white cursor-pointer"
                >
                  {validating['library'] ? '...' : 'Validate'}
                </button>
              </div>
              {#if validationResults['library']}
                <div class="flex items-center gap-2 text-[11px] font-mono mt-1">
                  {#if validationResults['library'].writable}
                    <span class="text-emerald-500 dark:text-emerald-400">✓ Writable directory verified</span>
                  {:else if validationResults['library'].exists}
                    <span class="text-amber-500 dark:text-amber-400">⚠ Exists but not writable</span>
                  {:else}
                    <span class="text-rose-500 dark:text-rose-400">✗ Directory does not exist</span>
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Temp Encode Dir -->
            <div class="space-y-2">
              <label for="input-temp-dir" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Transcode Temp Directory</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">FFmpeg scratch</span>
              </label>
              <input
                id="input-temp-dir"
                type="text"
                bind:value={config.temp_encode_dir}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>

            <!-- Covers Dir -->
            <div class="space-y-2">
              <label for="input-covers-dir" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>AniList Cover Cache</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">Image assets</span>
              </label>
              <input
                id="input-covers-dir"
                type="text"
                bind:value={config.covers_dir}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>
          </div>
        </div>

        <!-- Section 2: BitTorrent Engine Limits -->
        <div class="glass-panel rounded-3xl p-6 space-y-5 border border-slate-200/80 dark:border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-slate-200/80 dark:border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">BitTorrent Engine</h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">librqbit peer limits, ports, and global traffic shaping</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-300 border border-emerald-500/25">LIBRQBIT 9.1</span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-5">
            <div class="space-y-2">
              <label for="input-port" class="text-xs font-mono text-slate-700 dark:text-slate-300">Listen Port (TCP/UDP)</label>
              <input
                id="input-port"
                type="number"
                value={config.torrent_listen_port}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>

            <div class="space-y-2">
              <span class="text-xs font-mono text-slate-700 dark:text-slate-300 block">DHT Network</span>
              <div class="h-[38px] flex items-center">
                <span class="text-xs font-mono px-3 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-600 dark:text-emerald-300 font-bold">
                  {config.torrent_enable_dht ? 'ENABLED' : 'DISABLED'}
                </span>
              </div>
            </div>

            <div class="space-y-2">
              <label for="input-dl-limit" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Max Download Speed</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">KB/s (0 = unlim)</span>
              </label>
              <input
                id="input-dl-limit"
                type="number"
                bind:value={config.torrent_global_download_limit_kbps}
                min="0"
                step="128"
                class="glass-input w-full rounded-2xl px-3.5 py-2 text-xs font-mono"
              />
            </div>

            <div class="space-y-2">
              <label for="input-ul-limit" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Max Upload Speed</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">KB/s (0 = unlim)</span>
              </label>
              <input
                id="input-ul-limit"
                type="number"
                bind:value={config.torrent_global_upload_limit_kbps}
                min="0"
                step="64"
                class="glass-input w-full rounded-2xl px-3.5 py-2 text-xs font-mono"
              />
            </div>
          </div>
        </div>

        <!-- Section 3: Scraper & Metadata Integration -->
        <div class="glass-panel rounded-3xl p-6 space-y-5 border border-slate-200/80 dark:border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-slate-200/80 dark:border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">Scraper & Metadata Sync</h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Nyaa.si scrape settings and AniList GraphQL connection</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-400/25">ANILIST V2</span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
            <div class="space-y-2">
              <label for="input-nyaa-url" class="text-xs font-mono text-slate-700 dark:text-slate-300">Nyaa Base URL</label>
              <input
                id="input-nyaa-url"
                type="text"
                value={config.nyaa_base_url}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>

            <div class="space-y-2">
              <label for="select-category" class="text-xs font-mono text-slate-700 dark:text-slate-300">Default Nyaa Category</label>
              <select
                id="select-category"
                bind:value={config.nyaa_default_category}
                class="glass-input w-full rounded-2xl px-3.5 py-2 text-xs font-mono"
              >
                <option value="1_2" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - English-translated (1_2)</option>
                <option value="1_0" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - All Categories (1_0)</option>
                <option value="1_1" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - Raw (1_1)</option>
                <option value="1_3" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - Non-English-translated (1_3)</option>
                <option value="1_4" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - Music Video (1_4)</option>
              </select>
            </div>

            <div class="space-y-2">
              <label for="input-anilist-url" class="text-xs font-mono text-slate-700 dark:text-slate-300">AniList GraphQL Endpoint</label>
              <input
                id="input-anilist-url"
                type="text"
                value={config.anilist_graphql_url}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>
          </div>
        </div>

        <!-- Section 4: Video Transcoder Binaries & Presets -->
        <div class="glass-panel rounded-3xl p-6 space-y-5 border border-slate-200/80 dark:border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-slate-200/80 dark:border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">Video Transcoder Pipeline</h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">FFmpeg / FFprobe runtime binaries and default encoding profile</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-purple-500/10 text-purple-600 dark:text-purple-300 border border-purple-500/25">FFMPEG 7.x</span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
            <div class="space-y-2">
              <label for="input-ffmpeg-path" class="text-xs font-mono text-slate-700 dark:text-slate-300">FFmpeg Binary</label>
              <input
                id="input-ffmpeg-path"
                type="text"
                value={config.ffmpeg_binary_path}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>

            <div class="space-y-2">
              <label for="input-ffprobe-path" class="text-xs font-mono text-slate-700 dark:text-slate-300">FFprobe Binary</label>
              <input
                id="input-ffprobe-path"
                type="text"
                value={config.ffprobe_binary_path}
                readonly
                class="glass-input w-full opacity-60 rounded-2xl px-3.5 py-2 text-xs font-mono cursor-not-allowed"
              />
            </div>

            <div class="space-y-2">
              <label for="input-default-preset" class="text-xs font-mono text-slate-700 dark:text-slate-300">Default Encode Preset</label>
              <input
                id="input-default-preset"
                type="text"
                bind:value={config.encoder_default_preset}
                class="glass-input w-full rounded-2xl px-3.5 py-2 text-xs font-mono"
                placeholder="720p x265 Balanced"
              />
            </div>
          </div>
        </div>

        <!-- Section 5: AI Subtitle Translator (Google AI Studio) -->
        <div class="glass-panel rounded-3xl p-6 space-y-5 border border-slate-200/80 dark:border-white/10 shadow-2xl">
          <div class="flex items-center justify-between border-b border-slate-200/80 dark:border-white/5 pb-3">
            <div>
              <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white flex items-center gap-2">
                <Languages class="w-4 h-4 text-cyan-500 dark:text-cyan-400" />
                <span>AI Subtitle Translator (Google AI Studio)</span>
              </h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Configure Gemini API credentials and default model for automated subtitle translation</p>
            </div>
            <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full border font-bold {config.gemini_api_key_configured
              ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300 border-emerald-500/30'
              : 'bg-amber-500/10 text-amber-600 dark:text-amber-300 border-amber-500/30'}">
              {config.gemini_api_key_configured ? `CONFIGURED (${config.gemini_api_key_masked || 'SAVED'})` : 'NOT CONFIGURED'}
            </span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
            <div class="space-y-2">
              <label for="input-gemini-key" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span class="flex items-center gap-1.5">
                  <KeyRound class="w-3.5 h-3.5 text-cyan-500" />
                  <span>Google AI Studio API Key</span>
                </span>
                {#if config.gemini_api_key_configured}
                  <button
                    type="button"
                    onclick={() => {
                      clearGeminiKey = !clearGeminiKey;
                      if (clearGeminiKey) geminiApiKeyInput = '';
                    }}
                    class="text-[10px] underline cursor-pointer {clearGeminiKey ? 'text-rose-500 font-bold' : 'text-slate-400 hover:text-rose-400'}"
                  >
                    {clearGeminiKey ? 'Will clear key on Save (click to undo)' : 'Clear stored key'}
                  </button>
                {/if}
              </label>
              <div class="flex gap-2">
                <input
                  id="input-gemini-key"
                  type="password"
                  bind:value={geminiApiKeyInput}
                  disabled={clearGeminiKey}
                  class="glass-input flex-1 rounded-2xl px-3.5 py-2 text-xs font-mono disabled:opacity-50"
                  placeholder={config.gemini_api_key_configured
                    ? `Current: ${config.gemini_api_key_masked || '••••'} (enter new key to replace)`
                    : 'AIza...'}
                />
                <button
                  type="button"
                  onclick={testGeminiConnection}
                  disabled={testingGeminiKey}
                  class="btn-expressive glass-pill text-xs font-mono px-3.5 py-2 rounded-2xl text-cyan-600 dark:text-cyan-300 border-cyan-400/30 cursor-pointer whitespace-nowrap"
                >
                  {testingGeminiKey ? 'Testing...' : 'Test Key'}
                </button>
              </div>
              {#if geminiTestResult}
                <div class="text-[11px] font-mono mt-1 {geminiTestResult.ok ? 'text-emerald-500 dark:text-emerald-400' : 'text-rose-500 dark:text-rose-400'}">
                  {geminiTestResult.ok ? '✓' : '✗'} {geminiTestResult.text}
                </div>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="input-gemini-model" class="text-xs font-mono text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Default Gemini Model</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">e.g. gemini-2.5-flash</span>
              </label>
              <input
                id="input-gemini-model"
                type="text"
                bind:value={config.gemini_model}
                class="glass-input w-full rounded-2xl px-3.5 py-2 text-xs font-mono"
                placeholder="gemini-2.5-flash"
              />
            </div>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>
