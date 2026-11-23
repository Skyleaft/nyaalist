<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '#lib/api/client';
  import type { AppConfigDto, PathValidationResponse } from '#lib/api/types';

  let config = $state<AppConfigDto | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let saveSuccess = $state(false);
  let errorMessage = $state<string | null>(null);

  // Path validation states
  let validating = $state<Record<string, boolean>>({});
  let validationResults = $state<Record<string, PathValidationResponse | null>>({});

  onMount(async () => {
    await fetchConfig();
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
        encoder_default_preset: config.encoder_default_preset
      });
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

<div class="flex-1 p-6 max-w-6xl mx-auto w-full space-y-6">
  <!-- Top Bar -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-[#2C2C2E] pb-4">
    <div>
      <div class="flex items-center gap-3">
        <div class="w-3 h-3 rounded-full bg-[#00E5FF]"></div>
        <h1 class="text-xl font-bold font-mono text-white tracking-wide">SYSTEM SETTINGS</h1>
      </div>
      <p class="text-xs text-[#98989D] mt-1 font-mono">
        Configure storage paths, BitTorrent bandwidth caps, scraping parameters, and video presets
      </p>
    </div>

    <div class="flex items-center gap-3">
      {#if saveSuccess}
        <div class="text-xs font-mono text-[#32D74B] bg-[#1E1E1E] border border-[#32D74B]/40 px-3 py-1.5 rounded flex items-center gap-2">
          <span>✓</span>
          <span>Settings saved successfully</span>
        </div>
      {/if}
      <button
        onclick={saveConfig}
        disabled={saving || loading || !config}
        class="bg-[#00E5FF] hover:bg-[#00C2D8] disabled:opacity-50 text-black text-xs font-mono font-bold px-5 py-2 rounded-lg transition-colors flex items-center gap-2 cursor-pointer shadow-[0_0_15px_rgba(0,229,255,0.25)]"
      >
        {#if saving}
          <div class="w-3.5 h-3.5 border-2 border-black border-t-transparent rounded-full animate-spin"></div>
          <span>SAVING...</span>
        {:else}
          <span>SAVE CHANGES</span>
        {/if}
      </button>
    </div>
  </div>

  {#if errorMessage}
    <div class="bg-[#3A1C1C] border-l-4 border-[#FF453A] p-4 rounded-r-xl text-xs font-mono text-red-200">
      {errorMessage}
    </div>
  {/if}

  {#if loading}
    <div class="flex items-center justify-center p-16">
      <div class="w-8 h-8 border-2 border-[#00E5FF] border-t-transparent rounded-full animate-spin"></div>
    </div>
  {:else if config}
    <div class="space-y-6">
      <!-- Section 1: Storage Paths -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-5">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">Storage Directories</h2>
            <p class="text-xs text-[#98989D] mt-0.5">Filesystem target directories for torrent staging and library storage</p>
          </div>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-[#121212] text-[#00E5FF] border border-[#2C2C2E]">PATH ENGINES</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
          <!-- Downloads Dir -->
          <div class="space-y-2">
            <label for="input-dl-dir" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>Downloads Directory</span>
              <span class="text-[10px] text-[#98989D]">Staging area</span>
            </label>
            <div class="flex gap-2">
              <input
                id="input-dl-dir"
                type="text"
                bind:value={config.downloads_dir}
                class="flex-1 bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
                placeholder="/data/downloads"
              />
              <button
                type="button"
                onclick={() => validateDirectory('downloads', config?.downloads_dir || '')}
                disabled={validating['downloads']}
                class="bg-[#252525] hover:bg-[#323232] text-xs font-mono px-3 py-2 rounded-lg border border-[#2C2C2E] text-gray-200 transition-colors cursor-pointer"
              >
                {validating['downloads'] ? '...' : 'Validate'}
              </button>
            </div>
            {#if validationResults['downloads']}
              <div class="flex items-center gap-2 text-[11px] font-mono mt-1">
                {#if validationResults['downloads'].writable}
                  <span class="text-[#32D74B]">✓ Writable directory verified</span>
                {:else if validationResults['downloads'].exists}
                  <span class="text-[#FF9F0A]">⚠ Exists but not writable</span>
                {:else}
                  <span class="text-[#FF453A]">✗ Directory does not exist</span>
                {/if}
              </div>
            {/if}
          </div>

          <!-- Library Dir -->
          <div class="space-y-2">
            <label for="input-lib-dir" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>Organized Library Directory</span>
              <span class="text-[10px] text-[#98989D]">Media root</span>
            </label>
            <div class="flex gap-2">
              <input
                id="input-lib-dir"
                type="text"
                bind:value={config.library_dir}
                class="flex-1 bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
                placeholder="/data/library"
              />
              <button
                type="button"
                onclick={() => validateDirectory('library', config?.library_dir || '')}
                disabled={validating['library']}
                class="bg-[#252525] hover:bg-[#323232] text-xs font-mono px-3 py-2 rounded-lg border border-[#2C2C2E] text-gray-200 transition-colors cursor-pointer"
              >
                {validating['library'] ? '...' : 'Validate'}
              </button>
            </div>
            {#if validationResults['library']}
              <div class="flex items-center gap-2 text-[11px] font-mono mt-1">
                {#if validationResults['library'].writable}
                  <span class="text-[#32D74B]">✓ Writable directory verified</span>
                {:else if validationResults['library'].exists}
                  <span class="text-[#FF9F0A]">⚠ Exists but not writable</span>
                {:else}
                  <span class="text-[#FF453A]">✗ Directory does not exist</span>
                {/if}
              </div>
            {/if}
          </div>

          <!-- Temp Encode Dir -->
          <div class="space-y-2">
            <label for="input-temp-dir" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>Transcode Temp Directory</span>
              <span class="text-[10px] text-[#98989D]">FFmpeg scratch</span>
            </label>
            <input
              id="input-temp-dir"
              type="text"
              bind:value={config.temp_encode_dir}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>

          <!-- Covers Dir -->
          <div class="space-y-2">
            <label for="input-covers-dir" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>AniList Cover Cache</span>
              <span class="text-[10px] text-[#98989D]">Image assets</span>
            </label>
            <input
              id="input-covers-dir"
              type="text"
              bind:value={config.covers_dir}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>
        </div>
      </div>

      <!-- Section 2: BitTorrent Engine Limits -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-5">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">BitTorrent Engine</h2>
            <p class="text-xs text-[#98989D] mt-0.5">librqbit peer limits, ports, and global traffic shaping</p>
          </div>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-[#121212] text-[#32D74B] border border-[#2C2C2E]">LIBRQBIT 9.1</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-5">
          <div class="space-y-2">
            <label for="input-port" class="text-xs font-mono text-gray-300">Listen Port (TCP/UDP)</label>
            <input
              id="input-port"
              type="number"
              value={config.torrent_listen_port}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>

          <div class="space-y-2">
            <label for="input-dht" class="text-xs font-mono text-gray-300">DHT Network</label>
            <div class="h-[38px] flex items-center">
              <span class="text-xs font-mono px-3 py-1 rounded bg-[#121212] border border-[#2C2C2E] text-[#32D74B]">
                {config.torrent_enable_dht ? 'ENABLED' : 'DISABLED'}
              </span>
            </div>
          </div>

          <div class="space-y-2">
            <label for="input-dl-limit" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>Max Download Speed</span>
              <span class="text-[10px] text-[#98989D]">KB/s (0 = unlim)</span>
            </label>
            <input
              id="input-dl-limit"
              type="number"
              bind:value={config.torrent_global_download_limit_kbps}
              min="0"
              step="128"
              class="w-full bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
            />
          </div>

          <div class="space-y-2">
            <label for="input-ul-limit" class="text-xs font-mono text-gray-300 flex items-center justify-between">
              <span>Max Upload Speed</span>
              <span class="text-[10px] text-[#98989D]">KB/s (0 = unlim)</span>
            </label>
            <input
              id="input-ul-limit"
              type="number"
              bind:value={config.torrent_global_upload_limit_kbps}
              min="0"
              step="64"
              class="w-full bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
            />
          </div>
        </div>
      </div>

      <!-- Section 3: Scraper & Metadata Integration -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-5">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">Scraper & Metadata Sync</h2>
            <p class="text-xs text-[#98989D] mt-0.5">Nyaa.si scrape settings and AniList GraphQL connection</p>
          </div>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-[#121212] text-[#00E5FF] border border-[#2C2C2E]">ANILIST V2</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
          <div class="space-y-2">
            <label for="input-nyaa-url" class="text-xs font-mono text-gray-300">Nyaa Base URL</label>
            <input
              id="input-nyaa-url"
              type="text"
              value={config.nyaa_base_url}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>

          <div class="space-y-2">
            <label for="select-category" class="text-xs font-mono text-gray-300">Default Nyaa Category</label>
            <select
              id="select-category"
              bind:value={config.nyaa_default_category}
              class="w-full bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
            >
              <option value="1_2">Anime - English-translated (1_2)</option>
              <option value="1_0">Anime - All Categories (1_0)</option>
              <option value="1_1">Anime - Raw (1_1)</option>
              <option value="1_3">Anime - Non-English-translated (1_3)</option>
              <option value="1_4">Anime - Music Video (1_4)</option>
            </select>
          </div>

          <div class="space-y-2">
            <label for="input-anilist-url" class="text-xs font-mono text-gray-300">AniList GraphQL Endpoint</label>
            <input
              id="input-anilist-url"
              type="text"
              value={config.anilist_graphql_url}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>
        </div>
      </div>

      <!-- Section 4: Video Transcoder Binaries & Presets -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-5">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">Video Transcoder Pipeline</h2>
            <p class="text-xs text-[#98989D] mt-0.5">FFmpeg / FFprobe runtime binaries and default encoding profile</p>
          </div>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-[#121212] text-[#32D74B] border border-[#2C2C2E]">FFMPEG 7.x</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
          <div class="space-y-2">
            <label for="input-ffmpeg-path" class="text-xs font-mono text-gray-300">FFmpeg Binary</label>
            <input
              id="input-ffmpeg-path"
              type="text"
              value={config.ffmpeg_binary_path}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>

          <div class="space-y-2">
            <label for="input-ffprobe-path" class="text-xs font-mono text-gray-300">FFprobe Binary</label>
            <input
              id="input-ffprobe-path"
              type="text"
              value={config.ffprobe_binary_path}
              readonly
              class="w-full bg-[#121212]/60 border border-[#2C2C2E] text-gray-400 rounded-lg px-3 py-2 text-xs font-mono cursor-not-allowed"
            />
          </div>

          <div class="space-y-2">
            <label for="input-default-preset" class="text-xs font-mono text-gray-300">Default Encode Preset</label>
            <input
              id="input-default-preset"
              type="text"
              bind:value={config.encoder_default_preset}
              class="w-full bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-lg px-3 py-2 text-xs font-mono text-white transition-colors"
              placeholder="720p x265 Balanced"
            />
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
