<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { torrents } from '#lib/stores/torrents.svelte';
  import { encoder } from '#lib/stores/encoder.svelte';
  import { library } from '#lib/stores/library.svelte';
  import { api } from '#lib/api/client';

  let quickMagnet = $state('');
  let isAddingMagnet = $state(false);
  let magnetError = $state<string | null>(null);
  let magnetSuccess = $state(false);

  let quickSearch = $state('');

  onMount(async () => {
    await Promise.all([
      torrents.init(),
      encoder.init(),
      library.init()
    ]);
  });

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  }

  function formatDuration(seconds: number | null | undefined): string {
    if (!seconds || seconds <= 0 || !isFinite(seconds)) return '--';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) {
      return `${h}h ${m.toString().padStart(2, '0')}m`;
    }
    return `${m}m ${s.toString().padStart(2, '0')}s`;
  }

  async function handleAddMagnet() {
    if (!quickMagnet.trim()) return;
    isAddingMagnet = true;
    magnetError = null;
    magnetSuccess = false;

    try {
      await api.downloader.add({ magnet_uri: quickMagnet.trim() });
      quickMagnet = '';
      magnetSuccess = true;
      await torrents.fetchJobs();
      setTimeout(() => {
        magnetSuccess = false;
      }, 3000);
    } catch (err: any) {
      magnetError = err.message || 'Failed to add magnet link';
    } finally {
      isAddingMagnet = false;
    }
  }

  function handleSearchSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (quickSearch.trim()) {
      goto(`/explore?query=${encodeURIComponent(quickSearch.trim())}`);
    }
  }
</script>

<svelte:head>
  <title>Dashboard - Nyaalist Mission Control</title>
</svelte:head>

<div class="flex-1 p-6 max-w-7xl mx-auto w-full space-y-6">
  <!-- Top Quick Bar (Fast Magnet Adder & Quick Search) -->
  <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-4 flex flex-col lg:flex-row items-center justify-between gap-4">
    <!-- Fast Magnet Ingestion -->
    <div class="flex-1 w-full flex items-center gap-2">
      <div class="relative flex-1">
        <input
          type="text"
          bind:value={quickMagnet}
          placeholder="Paste magnet:?xt=urn:btih:... or torrent info-hash"
          class="w-full bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-xl px-4 py-2 text-xs font-mono text-white placeholder-gray-500 transition-colors"
          onkeydown={(e) => e.key === 'Enter' && handleAddMagnet()}
        />
        {#if magnetSuccess}
          <span class="absolute right-3 top-2 text-[10px] font-mono text-[#32D74B]">✓ Torrent queued</span>
        {/if}
      </div>
      <button
        onclick={handleAddMagnet}
        disabled={isAddingMagnet || !quickMagnet.trim()}
        class="bg-[#00E5FF] hover:bg-[#00C2D8] disabled:opacity-50 text-black text-xs font-mono font-bold px-4 py-2 rounded-xl transition-colors flex items-center gap-1.5 cursor-pointer whitespace-nowrap"
      >
        <span>+ Add Magnet</span>
      </button>
    </div>

    <!-- Quick Release Search & WebSocket Status -->
    <div class="flex items-center gap-4 w-full lg:w-auto justify-between lg:justify-end">
      <form onsubmit={handleSearchSubmit} class="flex items-center gap-2">
        <input
          type="text"
          bind:value={quickSearch}
          placeholder="Search Nyaa..."
          class="w-44 lg:w-56 bg-[#121212] border border-[#2C2C2E] focus:border-[#00E5FF] outline-none rounded-xl px-3 py-2 text-xs font-mono text-white placeholder-gray-500 transition-colors"
        />
        <button
          type="submit"
          class="bg-[#252525] hover:bg-[#323232] text-xs font-mono px-3 py-2 rounded-xl border border-[#2C2C2E] text-gray-200 transition-colors cursor-pointer"
        >
          Explore
        </button>
      </form>

      <!-- Live WS Indicator -->
      <div class="flex items-center gap-2 text-xs font-mono bg-[#121212] border border-[#2C2C2E] px-3 py-1.5 rounded-xl">
        <div class="w-2.5 h-2.5 rounded-full {torrents.wsConnected ? 'bg-[#32D74B] animate-pulse' : 'bg-[#FF453A]'}"></div>
        <span class="text-gray-300">{torrents.wsConnected ? 'WS LIVE' : 'WS OFFLINE'}</span>
      </div>
    </div>
  </div>

  {#if magnetError}
    <div class="bg-[#3A1C1C] border-l-4 border-[#FF453A] p-4 rounded-r-xl text-xs font-mono text-red-200">
      {magnetError}
    </div>
  {/if}

  <!-- 4 Primary KPI Cards (WattVision Technical Dark Theme) -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
    <!-- KPI 1: Download Throughput -->
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-5 space-y-2 relative overflow-hidden">
      <div class="flex items-center justify-between text-xs font-mono text-[#98989D]">
        <span>DOWNLOAD SPEED</span>
        <span class="text-[#00E5FF] font-bold">LIBRQBIT</span>
      </div>
      <div class="text-2xl font-bold font-mono text-[#00E5FF] tracking-tight">
        {formatBytes(torrents.totalDownloadSpeed)}/s
      </div>
      <div class="flex items-center justify-between text-[11px] font-mono text-gray-400 pt-1 border-t border-[#2C2C2E]">
        <span>Upload: <span class="text-[#32D74B]">{formatBytes(torrents.totalUploadSpeed)}/s</span></span>
        <span>{torrents.activeCount} active</span>
      </div>
    </div>

    <!-- KPI 2: Active Torrents -->
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-5 space-y-2 relative overflow-hidden">
      <div class="flex items-center justify-between text-xs font-mono text-[#98989D]">
        <span>TORRENT POOL</span>
        <span class="text-[#32D74B] font-bold">SWARM</span>
      </div>
      <div class="text-2xl font-bold font-mono text-white tracking-tight">
        {torrents.activeCount} <span class="text-sm font-normal text-[#98989D]">DL /</span> {torrents.completedCount} <span class="text-sm font-normal text-[#98989D]">DONE</span>
      </div>
      <div class="flex items-center justify-between text-[11px] font-mono text-gray-400 pt-1 border-t border-[#2C2C2E]">
        <span>Total: {torrents.jobs.length} jobs</span>
        <a href="/downloads" class="text-[#00E5FF] hover:underline">View all →</a>
      </div>
    </div>

    <!-- KPI 3: Transcoding Pipeline -->
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-5 space-y-2 relative overflow-hidden">
      <div class="flex items-center justify-between text-xs font-mono text-[#98989D]">
        <span>VIDEO TRANSCODER</span>
        <span class="text-[#32D74B] font-bold">FFMPEG</span>
      </div>
      <div class="text-2xl font-bold font-mono text-white tracking-tight">
        {#if encoder.activeJob}
          <span class="text-[#32D74B]">1 ACTIVE</span>
          <span class="text-xs font-mono text-gray-400 block font-normal truncate mt-0.5">
            {encoder.activeTelemetry ? `${encoder.activeTelemetry.speed.toFixed(2)}x (${encoder.activeTelemetry.fps.toFixed(0)} fps)` : 'Transcoding...'}
          </span>
        {:else}
          <span class="text-gray-400 font-normal text-lg">IDLE</span>
          <span class="text-xs font-mono text-[#98989D] block font-normal mt-0.5">{encoder.queuedCount} in queue</span>
        {/if}
      </div>
      <div class="flex items-center justify-between text-[11px] font-mono text-gray-400 pt-1 border-t border-[#2C2C2E]">
        <span>Saved: <span class="text-[#32D74B]">{formatBytes(encoder.totalSavedBytes)}</span></span>
        <a href="/encoder" class="text-[#00E5FF] hover:underline">Manage →</a>
      </div>
    </div>

    <!-- KPI 4: Media Collection -->
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-5 space-y-2 relative overflow-hidden">
      <div class="flex items-center justify-between text-xs font-mono text-[#98989D]">
        <span>LIBRARY MEDIA</span>
        <span class="text-[#00E5FF] font-bold">STORAGE</span>
      </div>
      <div class="text-2xl font-bold font-mono text-white tracking-tight">
        {library.items.length} <span class="text-sm font-normal text-[#98989D]">Files</span>
      </div>
      <div class="flex items-center justify-between text-[11px] font-mono text-gray-400 pt-1 border-t border-[#2C2C2E]">
        <span>Total: {formatBytes(library.totalSizeBytes)}</span>
        <a href="/library" class="text-[#00E5FF] hover:underline">Browse →</a>
      </div>
    </div>
  </div>

  <!-- Main Content Grid (8 Columns Left / 4 Columns Right) -->
  <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
    <!-- Left Column (8 cols): Active Operations & Live Pipelines -->
    <div class="lg:col-span-8 space-y-6">
      <!-- Active Downloads Panel -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-4">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div class="flex items-center gap-2">
            <div class="w-2.5 h-2.5 rounded-full bg-[#00E5FF] animate-pulse"></div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">Active Downloads</h2>
          </div>
          <a href="/downloads" class="text-xs font-mono text-[#00E5FF] hover:underline">Open Downloads Manager →</a>
        </div>

        {#if torrents.jobs.filter(j => j.status === 'DOWNLOADING').length === 0}
          <div class="p-8 text-center bg-[#121212] rounded-xl border border-[#2C2C2E]/60 space-y-3">
            <p class="text-xs font-mono text-[#98989D]">No active torrent downloads in progress</p>
            <a
              href="/explore"
              class="inline-block bg-[#00E5FF]/10 text-[#00E5FF] hover:bg-[#00E5FF]/20 border border-[#00E5FF]/30 px-4 py-2 rounded-lg text-xs font-mono font-semibold transition-colors"
            >
              Browse Latest Anime Releases
            </a>
          </div>
        {:else}
          <div class="space-y-4">
            {#each torrents.jobs.filter(j => j.status === 'DOWNLOADING') as job (job.id)}
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-3">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <h3 class="text-xs font-bold font-mono text-white truncate" title={job.name}>
                      {job.name}
                    </h3>
                    <div class="flex items-center gap-3 text-[11px] font-mono text-[#98989D] mt-1">
                      <span>{formatBytes(job.downloaded_bytes)} / {formatBytes(job.total_bytes)}</span>
                      <span>•</span>
                      <span class="text-[#00E5FF] font-semibold">{formatBytes(job.download_speed)}/s</span>
                      <span>•</span>
                      <span>Seeds: <strong class="text-white">{job.seeders}</strong></span>
                      <span>•</span>
                      <span>ETA: <strong class="text-white">{formatDuration(job.eta_seconds)}</strong></span>
                    </div>
                  </div>
                  <span class="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-[#00E5FF]/10 text-[#00E5FF] border border-[#00E5FF]/30 whitespace-nowrap">
                    {job.progress_pct.toFixed(1)}%
                  </span>
                </div>

                <!-- Progress Bar -->
                <div class="w-full h-2 bg-[#2C2C2E] rounded-full overflow-hidden">
                  <div
                    class="h-full bg-gradient-to-r from-[#00E5FF] to-[#32D74B] transition-all duration-300"
                    style="width: {Math.max(1, Math.min(100, job.progress_pct))}%"
                  ></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Active Transcode Pipeline Card -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-4">
        <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-3">
          <div class="flex items-center gap-2">
            <div class="w-2.5 h-2.5 rounded-full {encoder.activeJob ? 'bg-[#32D74B] animate-pulse' : 'bg-gray-500'}"></div>
            <h2 class="text-sm font-bold font-mono text-white uppercase tracking-wider">Transcoding Pipeline</h2>
          </div>
          <a href="/encoder" class="text-xs font-mono text-[#00E5FF] hover:underline">Encoder Queue ({encoder.queuedCount}) →</a>
        </div>

        {#if encoder.activeJob}
          <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-5 space-y-4">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <div class="min-w-0">
                <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-[#32D74B]/10 text-[#32D74B] border border-[#32D74B]/30 uppercase font-bold">
                  ENCODING IN PROGRESS
                </span>
                <h3 class="text-xs font-bold font-mono text-white truncate mt-2" title={encoder.activeJob.source_file_path}>
                  {encoder.activeJob.source_filename}
                </h3>
                <p class="text-[11px] font-mono text-[#98989D] mt-0.5">
                  Target Preset: <strong class="text-[#00E5FF]">{encoder.activeJob.preset_name}</strong>
                </p>
              </div>

              <div class="text-right font-mono">
                <div class="text-xl font-bold text-[#32D74B]">
                  {encoder.activeTelemetry ? encoder.activeTelemetry.progress_pct.toFixed(1) : encoder.activeJob.progress_pct.toFixed(1)}%
                </div>
                <div class="text-[11px] text-[#98989D]">
                  ETA: <strong class="text-white">{formatDuration(encoder.activeTelemetry?.eta_seconds || encoder.activeJob.eta_seconds)}</strong>
                </div>
              </div>
            </div>

            <!-- Progress Bar -->
            <div class="w-full h-2.5 bg-[#2C2C2E] rounded-full overflow-hidden">
              <div
                class="h-full bg-gradient-to-r from-[#00E5FF] to-[#32D74B] transition-all duration-300"
                style="width: {Math.max(1, Math.min(100, encoder.activeTelemetry?.progress_pct || encoder.activeJob.progress_pct))}%"
              ></div>
            </div>

            <!-- Live Telemetry Stats -->
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-2 text-center font-mono">
              <div class="bg-[#1E1E1E] border border-[#2C2C2E] p-2.5 rounded-lg">
                <div class="text-[10px] text-[#98989D] uppercase">Speed</div>
                <div class="text-xs font-bold text-[#00E5FF] mt-0.5">
                  {encoder.activeTelemetry ? `${encoder.activeTelemetry.speed.toFixed(2)}x` : '--'}
                </div>
              </div>

              <div class="bg-[#1E1E1E] border border-[#2C2C2E] p-2.5 rounded-lg">
                <div class="text-[10px] text-[#98989D] uppercase">Framerate</div>
                <div class="text-xs font-bold text-white mt-0.5">
                  {encoder.activeTelemetry ? `${encoder.activeTelemetry.fps.toFixed(1)} FPS` : '--'}
                </div>
              </div>

              <div class="bg-[#1E1E1E] border border-[#2C2C2E] p-2.5 rounded-lg">
                <div class="text-[10px] text-[#98989D] uppercase">Time Elapsed</div>
                <div class="text-xs font-bold text-gray-300 mt-0.5">
                  {encoder.activeTelemetry ? formatDuration(encoder.activeTelemetry.current_time_sec) : '--'}
                </div>
              </div>

              <div class="bg-[#1E1E1E] border border-[#2C2C2E] p-2.5 rounded-lg">
                <div class="text-[10px] text-[#98989D] uppercase">Space Saved</div>
                <div class="text-xs font-bold text-[#32D74B] mt-0.5">
                  {#if encoder.activeTelemetry && encoder.activeTelemetry.source_size_bytes > 0 && encoder.activeTelemetry.current_size_bytes > 0}
                    {formatBytes(Math.max(0, encoder.activeTelemetry.source_size_bytes - encoder.activeTelemetry.current_size_bytes))}
                  {:else}
                    --
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {:else}
          <div class="p-6 text-center bg-[#121212] rounded-xl border border-[#2C2C2E]/60 space-y-2">
            <p class="text-xs font-mono text-[#98989D]">Transcoder engine is idle</p>
            <a
              href="/encoder"
              class="inline-block bg-[#252525] hover:bg-[#323232] border border-[#2C2C2E] text-gray-200 text-xs font-mono px-4 py-2 rounded-lg transition-colors"
            >
              Queue Video Transcode Job
            </a>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right Column (4 cols): Quick Controls, Storage, System Alerts -->
    <div class="lg:col-span-4 space-y-6">
      <!-- Quick Navigation Actions -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-4">
        <h2 class="text-xs font-bold font-mono text-[#98989D] uppercase tracking-wider border-b border-[#2C2C2E] pb-2">
          Mission Control Actions
        </h2>

        <div class="grid grid-cols-1 gap-2.5">
          <a
            href="/explore"
            class="flex items-center justify-between p-3 rounded-xl bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-xs font-mono text-gray-200 hover:text-white transition-colors"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-[#00E5FF]">✦</span>
              <span>Explore Nyaa Releases</span>
            </div>
            <span class="text-gray-500">→</span>
          </a>

          <a
            href="/downloads"
            class="flex items-center justify-between p-3 rounded-xl bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-xs font-mono text-gray-200 hover:text-white transition-colors"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-[#32D74B]">↓</span>
              <span>Downloads Center</span>
            </div>
            <span class="text-gray-500">→</span>
          </a>

          <a
            href="/library"
            class="flex items-center justify-between p-3 rounded-xl bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-xs font-mono text-gray-200 hover:text-white transition-colors"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-[#00E5FF]">📁</span>
              <span>Library & Auto-Organize</span>
            </div>
            <span class="text-gray-500">→</span>
          </a>

          <a
            href="/encoder"
            class="flex items-center justify-between p-3 rounded-xl bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-xs font-mono text-gray-200 hover:text-white transition-colors"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-[#32D74B]">⚙</span>
              <span>Video Transcoder Queue</span>
            </div>
            <span class="text-gray-500">→</span>
          </a>

          <a
            href="/settings"
            class="flex items-center justify-between p-3 rounded-xl bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-xs font-mono text-gray-200 hover:text-white transition-colors"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-[#98989D]">🛠</span>
              <span>System Settings</span>
            </div>
            <span class="text-gray-500">→</span>
          </a>
        </div>
      </div>

      <!-- Storage Breakdown -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-4">
        <h2 class="text-xs font-bold font-mono text-[#98989D] uppercase tracking-wider border-b border-[#2C2C2E] pb-2">
          Storage Breakdown
        </h2>

        <div class="space-y-4">
          <div class="space-y-1.5">
            <div class="flex justify-between text-xs font-mono">
              <span class="text-gray-300">Organized Library</span>
              <span class="text-[#00E5FF] font-bold">{formatBytes(library.totalSizeBytes)}</span>
            </div>
            <div class="w-full h-1.5 bg-[#2C2C2E] rounded-full overflow-hidden">
              <div class="h-full bg-[#00E5FF]" style="width: 65%"></div>
            </div>
          </div>

          <div class="space-y-1.5">
            <div class="flex justify-between text-xs font-mono">
              <span class="text-gray-300">Active Downloads</span>
              <span class="text-[#32D74B] font-bold">
                {formatBytes(torrents.jobs.reduce((acc, j) => acc + j.downloaded_bytes, 0))}
              </span>
            </div>
            <div class="w-full h-1.5 bg-[#2C2C2E] rounded-full overflow-hidden">
              <div class="h-full bg-[#32D74B]" style="width: 35%"></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Telemetry Health & Live Status -->
      <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 space-y-4">
        <h2 class="text-xs font-bold font-mono text-[#98989D] uppercase tracking-wider border-b border-[#2C2C2E] pb-2">
          System Core Status
        </h2>

        <div class="space-y-2.5 text-xs font-mono">
          <div class="flex items-center justify-between p-2.5 rounded-lg bg-[#121212] border border-[#2C2C2E]">
            <span class="text-gray-300">Backend Engine</span>
            <span class="text-[#32D74B] font-semibold">ONLINE (Rust)</span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-lg bg-[#121212] border border-[#2C2C2E]">
            <span class="text-gray-300">Database</span>
            <span class="text-[#32D74B] font-semibold">POSTGRESQL 19</span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-lg bg-[#121212] border border-[#2C2C2E]">
            <span class="text-gray-300">BitTorrent Daemon</span>
            <span class="text-[#00E5FF] font-semibold">LIBRQBIT 9.1</span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-lg bg-[#121212] border border-[#2C2C2E]">
            <span class="text-gray-300">FFmpeg Pipeline</span>
            <span class="text-[#32D74B] font-semibold">INITIALIZED</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
