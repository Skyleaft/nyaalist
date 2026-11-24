<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { torrents } from '#lib/stores/torrents.svelte';
  import { encoder } from '#lib/stores/encoder.svelte';
  import { library } from '#lib/stores/library.svelte';
  import { api } from '#lib/api/client';
  import {
    Activity,
    ArrowDownCircle,
    ArrowUpRight,
    Check,
    CheckCircle2,
    Compass,
    Cpu,
    DownloadCloud,
    FolderKanban,
    HardDrive,
    Layers,
    Play,
    Plus,
    Radio,
    Search,
    Settings,
    Sparkles,
    UploadCloud,
    Video,
    Zap
  } from 'lucide-svelte';

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

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-6">
  <!-- Top Expressive Glass Command Deck -->
  <div class="glass-panel rounded-3xl p-4 sm:p-5 flex flex-col lg:flex-row items-center justify-between gap-4 border border-white/10 shadow-2xl relative overflow-hidden">
    <!-- Fast Magnet Ingestion -->
    <div class="flex-1 w-full flex items-center gap-2.5">
      <div class="relative flex-1 group">
        <input
          type="text"
          bind:value={quickMagnet}
          placeholder="Paste magnet:?xt=urn:btih:... or torrent info-hash"
          class="glass-input w-full rounded-2xl px-4 py-2.5 text-xs font-mono placeholder-slate-500 transition-all pr-28"
          onkeydown={(e) => e.key === 'Enter' && handleAddMagnet()}
        />
        {#if magnetSuccess}
          <div class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1 text-[11px] font-mono text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20 animate-fade-in">
            <Check class="w-3 h-3" />
            <span>Queued</span>
          </div>
        {/if}
      </div>

      <button
        onclick={handleAddMagnet}
        disabled={isAddingMagnet || !quickMagnet.trim()}
        class="btn-expressive bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 disabled:opacity-40 text-slate-950 text-xs font-bold px-4 py-2.5 rounded-2xl transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap shadow-[0_0_20px_rgba(0,229,255,0.25)]"
      >
        <Plus class="w-4 h-4 stroke-[2.5]" />
        <span>Add Magnet</span>
      </button>
    </div>

    <!-- Quick Search & Live Status Indicator -->
    <div class="flex items-center gap-3 w-full lg:w-auto justify-between lg:justify-end">
      <form onsubmit={handleSearchSubmit} class="flex items-center gap-2 flex-1 sm:flex-none">
        <div class="relative w-full sm:w-56">
          <Search class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            bind:value={quickSearch}
            placeholder="Search Nyaa..."
            class="glass-input w-full rounded-2xl pl-9 pr-3 py-2.5 text-xs font-mono"
          />
        </div>
        <button
          type="submit"
          class="btn-expressive glass-pill px-3.5 py-2.5 rounded-2xl text-xs font-semibold text-slate-700 dark:text-slate-200 hover:text-slate-900 dark:hover:text-white cursor-pointer whitespace-nowrap"
        >
          Explore
        </button>
      </form>

      <!-- WebSocket Live Pill -->
      <div class="glass-pill px-3 py-2 rounded-2xl flex items-center gap-2 text-xs font-mono">
        <div class="relative flex items-center justify-center">
          <div class="w-2.5 h-2.5 rounded-full {torrents.wsConnected ? 'bg-emerald-400' : 'bg-rose-500'}"></div>
          {#if torrents.wsConnected}
            <div class="absolute w-4 h-4 rounded-full bg-emerald-400/30 animate-ping"></div>
          {/if}
        </div>
        <span class="text-xs font-medium {torrents.wsConnected ? 'text-emerald-500 dark:text-emerald-300' : 'text-rose-500 dark:text-rose-400'}">
          {torrents.wsConnected ? 'LIVE' : 'OFFLINE'}
        </span>
      </div>
    </div>
  </div>

  {#if magnetError}
    <div class="glass-panel border-l-4 border-rose-500 p-4 rounded-2xl text-xs font-mono text-rose-600 dark:text-rose-300 bg-rose-500/10 dark:bg-rose-950/20 flex items-center justify-between">
      <span>{magnetError}</span>
      <button onclick={() => (magnetError = null)} class="text-rose-500 hover:text-rose-700 dark:text-rose-400 dark:hover:text-white cursor-pointer font-bold">✕</button>
    </div>
  {/if}

  <!-- 4 Primary Expressive KPI Cards -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
    <!-- KPI 1: Download Throughput -->
    <div class="glass-card-interactive rounded-3xl p-5 space-y-3 relative overflow-hidden group">
      <div class="flex items-center justify-between text-xs font-semibold text-slate-500 dark:text-slate-400">
        <span class="tracking-wide uppercase text-[11px]">Download Speed</span>
        <div class="p-2 rounded-xl bg-cyan-500/10 text-cyan-300 border border-cyan-500/20 group-hover:scale-110 transition-transform">
          <DownloadCloud class="w-4 h-4" />
        </div>
      </div>
      <div>
        <div class="text-2xl sm:text-3xl font-extrabold font-mono tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-sky-300">
          {formatBytes(torrents.totalDownloadSpeed)}/s
        </div>
        <div class="flex items-center justify-between text-xs font-mono text-slate-500 dark:text-slate-400 pt-3 mt-3 border-t border-slate-200/60 dark:border-white/5">
          <span class="flex items-center gap-1.5">
            <UploadCloud class="w-3.5 h-3.5 text-emerald-400" />
            <span class="text-emerald-300">{formatBytes(torrents.totalUploadSpeed)}/s</span>
          </span>
          <span class="px-2 py-0.5 rounded-full text-[10px] bg-black/5 dark:bg-white/5 text-slate-600 dark:text-slate-300">
            {torrents.activeCount} active
          </span>
        </div>
      </div>
    </div>

    <!-- KPI 2: Active Torrents Pool -->
    <div class="glass-card-interactive rounded-3xl p-5 space-y-3 relative overflow-hidden group">
      <div class="flex items-center justify-between text-xs font-semibold text-slate-500 dark:text-slate-400">
        <span class="tracking-wide uppercase text-[11px]">Torrent Pool</span>
        <div class="p-2 rounded-xl bg-emerald-500/10 text-emerald-300 border border-emerald-500/20 group-hover:scale-110 transition-transform">
          <Layers class="w-4 h-4" />
        </div>
      </div>
      <div>
        <div class="text-2xl sm:text-3xl font-extrabold font-mono tracking-tight text-slate-900 dark:text-white">
          {torrents.activeCount} <span class="text-sm font-normal text-slate-500 dark:text-slate-400">DL /</span> {torrents.completedCount} <span class="text-sm font-normal text-emerald-500 dark:text-emerald-400">DONE</span>
        </div>
        <div class="flex items-center justify-between text-xs font-mono text-slate-500 dark:text-slate-400 pt-3 mt-3 border-t border-slate-200/60 dark:border-white/5">
          <span>Total: {torrents.jobs.length} jobs</span>
          <a href="/downloads" class="text-cyan-400 hover:text-cyan-300 font-semibold flex items-center gap-1 group-hover:translate-x-0.5 transition-transform">
            <span>View all</span>
            <ArrowUpRight class="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </div>

    <!-- KPI 3: Transcoding Pipeline -->
    <div class="glass-card-interactive rounded-3xl p-5 space-y-3 relative overflow-hidden group">
      <div class="flex items-center justify-between text-xs font-semibold text-slate-500 dark:text-slate-400">
        <span class="tracking-wide uppercase text-[11px]">Video Transcoder</span>
        <div class="p-2 rounded-xl bg-purple-500/10 text-purple-300 border border-purple-500/20 group-hover:scale-110 transition-transform">
          <Video class="w-4 h-4" />
        </div>
      </div>
      <div>
        <div class="text-2xl sm:text-3xl font-extrabold font-mono tracking-tight text-slate-900 dark:text-white">
          {#if encoder.activeJob}
            <span class="text-purple-600 dark:text-purple-400">1 ACTIVE</span>
            <span class="text-xs font-mono text-slate-600 dark:text-slate-300 block font-normal truncate mt-1">
              {encoder.activeTelemetry ? `${encoder.activeTelemetry.speed.toFixed(2)}x (${encoder.activeTelemetry.fps.toFixed(0)} fps)` : 'Transcoding...'}
            </span>
          {:else}
            <span class="text-slate-500 dark:text-slate-400 font-medium">IDLE</span>
            <span class="text-xs font-mono text-slate-400 dark:text-slate-500 block font-normal mt-1">{encoder.queuedCount} in queue</span>
          {/if}
        </div>
        <div class="flex items-center justify-between text-xs font-mono text-slate-500 dark:text-slate-400 pt-3 mt-3 border-t border-slate-200/60 dark:border-white/5">
          <span>Saved: <strong class="text-purple-600 dark:text-purple-300">{formatBytes(encoder.totalSavedBytes)}</strong></span>
          <a href="/encoder" class="text-cyan-600 dark:text-cyan-400 hover:underline font-semibold flex items-center gap-1 group-hover:translate-x-0.5 transition-transform">
            <span>Manage</span>
            <ArrowUpRight class="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </div>

    <!-- KPI 4: Media Collection -->
    <div class="glass-card-interactive rounded-3xl p-5 space-y-3 relative overflow-hidden group">
      <div class="flex items-center justify-between text-xs font-semibold text-slate-500 dark:text-slate-400">
        <span class="tracking-wide uppercase text-[11px]">Library Media</span>
        <div class="p-2 rounded-xl bg-sky-500/10 text-sky-600 dark:text-sky-300 border border-sky-500/20 group-hover:scale-110 transition-transform">
          <HardDrive class="w-4 h-4" />
        </div>
      </div>
      <div>
        <div class="text-2xl sm:text-3xl font-extrabold font-mono tracking-tight text-slate-900 dark:text-white">
          {library.items.length} <span class="text-sm font-normal text-slate-500 dark:text-slate-400">Files</span>
        </div>
        <div class="flex items-center justify-between text-xs font-mono text-slate-500 dark:text-slate-400 pt-3 mt-3 border-t border-slate-200/60 dark:border-white/5">
          <span>Total: <strong class="text-sky-600 dark:text-sky-300">{formatBytes(library.totalSizeBytes)}</strong></span>
          <a href="/library" class="text-cyan-600 dark:text-cyan-400 hover:underline font-semibold flex items-center gap-1 group-hover:translate-x-0.5 transition-transform">
            <span>Browse</span>
            <ArrowUpRight class="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </div>
  </div>

  <!-- Main Grid (8 Cols Left / 4 Cols Right) -->
  <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
    <!-- Left Column: Operations & Active Jobs -->
    <div class="lg:col-span-8 space-y-6">
      <!-- Active Downloads Panel -->
      <div class="glass-panel rounded-3xl p-5 sm:p-6 space-y-4 border border-white/10 shadow-2xl">
        <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-3">
          <div class="flex items-center gap-2.5">
            <div class="relative flex items-center justify-center">
              <span class="w-2.5 h-2.5 rounded-full bg-cyan-500 dark:bg-cyan-400 animate-pulse"></span>
              <span class="absolute w-5 h-5 rounded-full bg-cyan-500/25 dark:bg-cyan-400/25 animate-ping"></span>
            </div>
            <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">Active Downloads</h2>
          </div>
          <a href="/downloads" class="text-xs font-semibold text-cyan-600 dark:text-cyan-400 hover:underline flex items-center gap-1 transition-colors">
            <span>Open Manager</span>
            <ArrowUpRight class="w-3.5 h-3.5" />
          </a>
        </div>

        {#if torrents.jobs.filter((j: any) => j.status === 'DOWNLOADING').length === 0}
          <div class="p-8 text-center rounded-2xl bg-slate-900/5 dark:bg-slate-950/30 border border-slate-200/60 dark:border-white/5 space-y-3">
            <DownloadCloud class="w-8 h-8 text-slate-400 dark:text-slate-600 mx-auto" />
            <p class="text-xs font-mono text-slate-500 dark:text-slate-400">No active torrent downloads in progress</p>
            <a
              href="/explore"
              class="inline-flex items-center gap-1.5 bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-400/30 px-4 py-2 rounded-xl text-xs font-semibold transition-all btn-expressive"
            >
              <Compass class="w-3.5 h-3.5" />
              <span>Browse Latest Anime Releases</span>
            </a>
          </div>
        {:else}
          <div class="space-y-3">
            {#each torrents.jobs.filter((j: any) => j.status === 'DOWNLOADING') as job (job.id)}
              <div class="glass-card-interactive rounded-2xl p-4 space-y-3">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <h3 class="text-xs font-bold font-mono text-slate-900 dark:text-white truncate max-w-xl" title={job.name}>
                      {job.name}
                    </h3>
                    <div class="flex items-center gap-3 text-[11px] font-mono text-slate-500 dark:text-slate-400 mt-1.5 flex-wrap">
                      <span>{formatBytes(job.downloaded_bytes)} / {formatBytes(job.total_bytes)}</span>
                      <span class="text-slate-400 dark:text-slate-600">•</span>
                      <span class="text-cyan-600 dark:text-cyan-400 font-semibold">{formatBytes(job.download_speed)}/s</span>
                      <span class="text-slate-400 dark:text-slate-600">•</span>
                      <span>Seeds: <strong class="text-slate-800 dark:text-white">{job.seeders}</strong></span>
                      <span class="text-slate-400 dark:text-slate-600">•</span>
                      <span>ETA: <strong class="text-slate-800 dark:text-white">{formatDuration(job.eta_seconds)}</strong></span>
                    </div>
                  </div>
                  <span class="text-[11px] font-mono font-bold px-2.5 py-0.5 rounded-full bg-cyan-400/10 text-cyan-700 dark:text-cyan-300 border border-cyan-400/30 whitespace-nowrap">
                    {job.progress_pct.toFixed(1)}%
                  </span>
                </div>

                <!-- Expressive Glowing Progress Bar -->
                <div class="w-full h-2 rounded-full bg-slate-200 dark:bg-slate-900/80 overflow-hidden border border-black/5 dark:border-white/5 relative">
                  <div
                    class="h-full rounded-full bg-gradient-to-r from-cyan-500 via-sky-400 to-emerald-400 transition-all duration-300 relative shadow-[0_0_12px_rgba(0,229,255,0.4)]"
                    style="width: {Math.max(1, Math.min(100, job.progress_pct))}%"
                  ></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Active Transcode Pipeline Card -->
      <div class="glass-panel rounded-3xl p-5 sm:p-6 space-y-4 border border-white/10 shadow-2xl">
        <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-3">
          <div class="flex items-center gap-2.5">
            <div class="relative flex items-center justify-center">
              <span class="w-2.5 h-2.5 rounded-full {encoder.activeJob ? 'bg-purple-500 dark:bg-purple-400 animate-pulse' : 'bg-slate-400 dark:bg-slate-600'}"></span>
              {#if encoder.activeJob}
                <span class="absolute w-5 h-5 rounded-full bg-purple-500/25 dark:bg-purple-400/25 animate-ping"></span>
              {/if}
            </div>
            <h2 class="text-sm font-bold uppercase tracking-wider text-slate-900 dark:text-white">Transcoding Pipeline</h2>
          </div>
          <a href="/encoder" class="text-xs font-semibold text-cyan-600 dark:text-cyan-400 hover:underline flex items-center gap-1 transition-colors">
            <span>Queue ({encoder.queuedCount})</span>
            <ArrowUpRight class="w-3.5 h-3.5" />
          </a>
        </div>

        {#if encoder.activeJob}
          <div class="glass-card-interactive rounded-2xl p-5 space-y-4">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <div class="min-w-0">
                <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-purple-500/10 text-purple-600 dark:text-purple-300 border border-purple-500/30 uppercase font-bold tracking-wider">
                  ENCODING IN PROGRESS
                </span>
                <h3 class="text-xs font-bold font-mono text-slate-900 dark:text-white truncate mt-2 max-w-lg" title={encoder.activeJob.source_file_path}>
                  {encoder.activeJob.source_filename}
                </h3>
                <p class="text-[11px] font-mono text-slate-500 dark:text-slate-400 mt-1">
                  Preset: <strong class="text-cyan-600 dark:text-cyan-300">{encoder.activeJob.preset_name}</strong>
                </p>
              </div>

              <div class="text-right font-mono">
                <div class="text-xl font-bold text-purple-600 dark:text-purple-400">
                  {encoder.activeTelemetry ? encoder.activeTelemetry.progress_pct.toFixed(1) : encoder.activeJob.progress_pct.toFixed(1)}%
                </div>
                <div class="text-[11px] text-slate-500 dark:text-slate-400">
                  ETA: <strong class="text-slate-800 dark:text-white">{formatDuration(encoder.activeTelemetry?.eta_seconds || encoder.activeJob.eta_seconds)}</strong>
                </div>
              </div>
            </div>

            <!-- Progress Bar -->
            <div class="w-full h-2.5 rounded-full bg-slate-900/80 overflow-hidden border border-white/5 relative">
              <div
                class="h-full rounded-full bg-gradient-to-r from-purple-500 to-cyan-400 transition-all duration-300 shadow-[0_0_12px_rgba(168,85,247,0.4)]"
                style="width: {Math.max(1, Math.min(100, encoder.activeTelemetry?.progress_pct || encoder.activeJob.progress_pct))}%"
              ></div>
            </div>

            <!-- Live Telemetry Matrix -->
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-1 text-center font-mono">
              <div class="glass-panel p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5">
                <div class="text-[10px] text-slate-500 dark:text-slate-400 uppercase tracking-wider">Speed</div>
                <div class="text-xs font-bold text-cyan-600 dark:text-cyan-300 mt-0.5">
                  {encoder.activeTelemetry ? `${encoder.activeTelemetry.speed.toFixed(2)}x` : '--'}
                </div>
              </div>

              <div class="glass-panel p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5">
                <div class="text-[10px] text-slate-500 dark:text-slate-400 uppercase tracking-wider">Framerate</div>
                <div class="text-xs font-bold text-slate-900 dark:text-white mt-0.5">
                  {encoder.activeTelemetry ? `${encoder.activeTelemetry.fps.toFixed(1)} FPS` : '--'}
                </div>
              </div>

              <div class="glass-panel p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5">
                <div class="text-[10px] text-slate-500 dark:text-slate-400 uppercase tracking-wider">Time Elapsed</div>
                <div class="text-xs font-bold text-slate-700 dark:text-slate-300 mt-0.5">
                  {encoder.activeTelemetry ? formatDuration(encoder.activeTelemetry.current_time_sec) : '--'}
                </div>
              </div>

              <div class="glass-panel p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5">
                <div class="text-[10px] text-slate-500 dark:text-slate-400 uppercase tracking-wider">Space Saved</div>
                <div class="text-xs font-bold text-emerald-600 dark:text-emerald-400 mt-0.5">
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
          <div class="p-8 text-center rounded-2xl bg-slate-100 dark:bg-slate-950/30 border border-slate-200/80 dark:border-white/5 space-y-3">
            <Cpu class="w-8 h-8 text-slate-400 dark:text-slate-600 mx-auto" />
            <p class="text-xs font-mono text-slate-500 dark:text-slate-400">Transcoder engine is idle</p>
            <a
              href="/encoder"
              class="inline-flex items-center gap-1.5 glass-pill px-4 py-2 rounded-xl text-xs font-semibold text-slate-700 dark:text-slate-200 hover:text-slate-900 dark:hover:text-white transition-all btn-expressive"
            >
              <Plus class="w-3.5 h-3.5" />
              <span>Queue Video Transcode Job</span>
            </a>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right Column: Navigation Actions, Storage & System Status -->
    <div class="lg:col-span-4 space-y-6">
      <!-- Quick Navigation Actions -->
      <div class="glass-panel rounded-3xl p-5 space-y-4 border border-slate-200/80 dark:border-white/10 shadow-2xl">
        <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 border-b border-slate-200/80 dark:border-white/5 pb-2.5">
          Mission Control Actions
        </h2>

        <div class="grid grid-cols-1 gap-2">
          <a
            href="/explore"
            class="flex items-center justify-between p-3 rounded-2xl glass-pill hover:bg-cyan-500/10 hover:border-cyan-400/30 text-xs font-medium text-slate-700 dark:text-slate-200 hover:text-cyan-600 dark:hover:text-cyan-300 transition-all group btn-expressive"
          >
            <div class="flex items-center gap-3">
              <div class="p-1.5 rounded-xl bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-500/20">
                <Compass class="w-4 h-4" />
              </div>
              <span>Explore Nyaa Releases</span>
            </div>
            <ArrowUpRight class="w-4 h-4 text-slate-400 dark:text-slate-500 group-hover:text-cyan-600 dark:group-hover:text-cyan-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
          </a>

          <a
            href="/downloads"
            class="flex items-center justify-between p-3 rounded-2xl glass-pill hover:bg-emerald-500/10 hover:border-emerald-400/30 text-xs font-medium text-slate-700 dark:text-slate-200 hover:text-emerald-600 dark:hover:text-emerald-300 transition-all group btn-expressive"
          >
            <div class="flex items-center gap-3">
              <div class="p-1.5 rounded-xl bg-emerald-500/10 text-emerald-600 dark:text-emerald-300 border border-emerald-500/20">
                <DownloadCloud class="w-4 h-4" />
              </div>
              <span>Downloads Center</span>
            </div>
            <ArrowUpRight class="w-4 h-4 text-slate-400 dark:text-slate-500 group-hover:text-emerald-600 dark:group-hover:text-emerald-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
          </a>

          <a
            href="/library"
            class="flex items-center justify-between p-3 rounded-2xl glass-pill hover:bg-sky-500/10 hover:border-sky-400/30 text-xs font-medium text-slate-700 dark:text-slate-200 hover:text-sky-600 dark:hover:text-sky-300 transition-all group btn-expressive"
          >
            <div class="flex items-center gap-3">
              <div class="p-1.5 rounded-xl bg-sky-500/10 text-sky-600 dark:text-sky-300 border border-sky-500/20">
                <FolderKanban class="w-4 h-4" />
              </div>
              <span>Library & Auto-Organize</span>
            </div>
            <ArrowUpRight class="w-4 h-4 text-slate-400 dark:text-slate-500 group-hover:text-sky-600 dark:group-hover:text-sky-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
          </a>

          <a
            href="/encoder"
            class="flex items-center justify-between p-3 rounded-2xl glass-pill hover:bg-purple-500/10 hover:border-purple-400/30 text-xs font-medium text-slate-700 dark:text-slate-200 hover:text-purple-600 dark:hover:text-purple-300 transition-all group btn-expressive"
          >
            <div class="flex items-center gap-3">
              <div class="p-1.5 rounded-xl bg-purple-500/10 text-purple-600 dark:text-purple-300 border border-purple-500/20">
                <Video class="w-4 h-4" />
              </div>
              <span>Video Transcoder Queue</span>
            </div>
            <ArrowUpRight class="w-4 h-4 text-slate-400 dark:text-slate-500 group-hover:text-purple-600 dark:group-hover:text-purple-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
          </a>

          <a
            href="/settings"
            class="flex items-center justify-between p-3 rounded-2xl glass-pill hover:bg-slate-200/50 dark:hover:bg-slate-700/30 text-xs font-medium text-slate-700 dark:text-slate-200 hover:text-slate-900 dark:hover:text-white transition-all group btn-expressive"
          >
            <div class="flex items-center gap-3">
              <div class="p-1.5 rounded-xl bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/10">
                <Settings class="w-4 h-4" />
              </div>
              <span>System Settings</span>
            </div>
            <ArrowUpRight class="w-4 h-4 text-slate-400 dark:text-slate-500 group-hover:text-slate-900 dark:group-hover:text-white group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
          </a>
        </div>
      </div>

      <!-- Storage Breakdown -->
      <div class="glass-panel rounded-3xl p-5 space-y-4 border border-slate-200/80 dark:border-white/10 shadow-2xl">
        <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 border-b border-slate-200/80 dark:border-white/5 pb-2.5">
          Storage Breakdown
        </h2>

        <div class="space-y-4">
          <div class="space-y-2">
            <div class="flex justify-between text-xs font-mono">
              <span class="text-slate-600 dark:text-slate-300">Organized Library</span>
              <span class="text-cyan-600 dark:text-cyan-400 font-bold">{formatBytes(library.totalSizeBytes)}</span>
            </div>
            <div class="w-full h-2 rounded-full bg-slate-200 dark:bg-slate-900 overflow-hidden border border-slate-200 dark:border-white/5">
              <div class="h-full rounded-full bg-gradient-to-r from-cyan-400 to-sky-400" style="width: 65%"></div>
            </div>
          </div>

          <div class="space-y-2">
            <div class="flex justify-between text-xs font-mono">
              <span class="text-slate-600 dark:text-slate-300">Active Downloads</span>
              <span class="text-emerald-600 dark:text-emerald-400 font-bold">
                {formatBytes(torrents.jobs.reduce((acc: number, j: any) => acc + (j.downloaded_bytes || 0), 0))}
              </span>
            </div>
            <div class="w-full h-2 rounded-full bg-slate-200 dark:bg-slate-900 overflow-hidden border border-slate-200 dark:border-white/5">
              <div class="h-full rounded-full bg-gradient-to-r from-emerald-400 to-teal-400" style="width: 35%"></div>
            </div>
          </div>
        </div>
      </div>

      <!-- System Core Status -->
      <div class="glass-panel rounded-3xl p-5 space-y-4 border border-slate-200/80 dark:border-white/10 shadow-2xl">
        <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 border-b border-slate-200/80 dark:border-white/5 pb-2.5">
          System Core Telemetry
        </h2>

        <div class="space-y-2.5 text-xs font-mono">
          <div class="flex items-center justify-between p-2.5 rounded-2xl bg-slate-100 dark:bg-slate-950/40 border border-slate-200/60 dark:border-white/5">
            <span class="text-slate-700 dark:text-slate-300">Backend Engine</span>
            <span class="text-emerald-600 dark:text-emerald-400 font-semibold flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-emerald-500 dark:bg-emerald-400"></span>
              <span>ONLINE (Rust)</span>
            </span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-2xl bg-slate-100 dark:bg-slate-950/40 border border-slate-200/60 dark:border-white/5">
            <span class="text-slate-700 dark:text-slate-300">Database</span>
            <span class="text-emerald-600 dark:text-emerald-400 font-semibold flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-emerald-500 dark:bg-emerald-400"></span>
              <span>POSTGRESQL 19</span>
            </span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-2xl bg-slate-100 dark:bg-slate-950/40 border border-slate-200/60 dark:border-white/5">
            <span class="text-slate-700 dark:text-slate-300">BitTorrent Daemon</span>
            <span class="text-cyan-600 dark:text-cyan-400 font-semibold flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-cyan-500 dark:bg-cyan-400"></span>
              <span>LIBRQBIT 9.1</span>
            </span>
          </div>

          <div class="flex items-center justify-between p-2.5 rounded-2xl bg-slate-100 dark:bg-slate-950/40 border border-slate-200/60 dark:border-white/5">
            <span class="text-slate-700 dark:text-slate-300">FFmpeg Pipeline</span>
            <span class="text-purple-600 dark:text-purple-400 font-semibold flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-purple-500 dark:bg-purple-400"></span>
              <span>INITIALIZED</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
