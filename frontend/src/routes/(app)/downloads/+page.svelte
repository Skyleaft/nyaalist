<script lang="ts">
  import { onMount } from 'svelte';
  import { torrents } from '#lib/stores/torrents.svelte';
  import type { DownloadJob } from '#lib/api/types';
  import {
    DownloadCloud,
    UploadCloud,
    Layers,
    CheckCircle2,
    Play,
    Pause,
    Trash2,
    Plus,
    Search,
    X,
    Activity,
    Compass,
    Radio,
    Clock,
    Users,
    Hash
  } from 'lucide-svelte';

  let filterStatus = $state<'ALL' | 'DOWNLOADING' | 'PAUSED' | 'COMPLETED'>('ALL');
  let searchQuery = $state('');
  let isAddModalOpen = $state(false);
  let newMagnetUri = $state('');
  let addError = $state<string | null>(null);
  let isAdding = $state(false);

  // Delete modal state
  let isDeleteModalOpen = $state(false);
  let targetJob = $state<DownloadJob | null>(null);
  let deleteFilesOnDisk = $state(false);

  onMount(() => {
    torrents.init();
  });

  const filteredJobs = $derived(
    torrents.jobs.filter((job: any) => {
      if (filterStatus !== 'ALL' && job.status !== filterStatus) return false;
      if (searchQuery.trim().length > 0) {
        const query = searchQuery.toLowerCase();
        return (
          job.name.toLowerCase().includes(query) ||
          job.info_hash.toLowerCase().includes(query)
        );
      }
      return true;
    })
  );

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  }

  function formatSpeed(bytesPerSec: number): string {
    if (!bytesPerSec || bytesPerSec <= 0) return '0 B/s';
    return `${formatBytes(bytesPerSec)}/s`;
  }

  function formatEta(seconds: number | null): string {
    if (!seconds || seconds <= 0) return '--';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }

  async function handleAddTorrent(e: Event) {
    e.preventDefault();
    if (!newMagnetUri.trim()) return;

    isAdding = true;
    addError = null;
    try {
      await torrents.addTorrent(newMagnetUri.trim());
      newMagnetUri = '';
      isAddModalOpen = false;
    } catch (err: any) {
      addError = err.message || 'Failed to add torrent';
    } finally {
      isAdding = false;
    }
  }

  function openDeleteModal(job: DownloadJob) {
    targetJob = job;
    deleteFilesOnDisk = false;
    isDeleteModalOpen = true;
  }

  async function confirmDelete() {
    if (!targetJob) return;
    try {
      await torrents.deleteTorrent(targetJob.id, deleteFilesOnDisk);
      isDeleteModalOpen = false;
      targetJob = null;
    } catch (err: any) {
      alert(`Error deleting torrent: ${err.message}`);
    }
  }
</script>

<svelte:head>
  <title>Downloader Telemetry | Nyaalist</title>
</svelte:head>

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-6">
  <!-- Header & WS Status -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-white/10">
    <div class="flex items-center gap-4">
      <div class="p-3.5 rounded-2xl bg-cyan-500/10 text-cyan-300 border border-cyan-400/25 shadow-[0_0_20px_rgba(0,229,255,0.2)]">
        <DownloadCloud class="w-7 h-7" />
      </div>
      <div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white">
          BitTorrent Downloader
        </h1>
        <p class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-0.5">
          librqbit telemetry engine & real-time WebSocket monitoring
        </p>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <!-- WS Status Pill -->
      <div class="glass-pill px-3.5 py-2 rounded-2xl flex items-center gap-2 text-xs font-mono">
        <div class="relative flex items-center justify-center">
          <div class="w-2.5 h-2.5 rounded-full {torrents.wsConnected ? 'bg-emerald-400' : 'bg-rose-500'}"></div>
          {#if torrents.wsConnected}
            <div class="absolute w-4 h-4 rounded-full bg-emerald-400/30 animate-ping"></div>
          {/if}
        </div>
        <span class="font-medium {torrents.wsConnected ? 'text-emerald-300' : 'text-rose-400'}">
          {torrents.wsConnected ? 'WS LIVE' : 'OFFLINE'}
        </span>
      </div>

      <button
        onclick={() => (isAddModalOpen = true)}
        class="btn-expressive bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 font-bold text-xs font-mono px-4 py-2.5 rounded-2xl transition-all shadow-[0_0_20px_rgba(0,229,255,0.25)] flex items-center gap-2 cursor-pointer"
      >
        <Plus class="w-4 h-4 stroke-[2.5]" />
        <span>Add Magnet</span>
      </button>
    </div>
  </div>

  <!-- Telemetry Gauges (4 M3 Cards) -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <!-- Active Downloads -->
    <div class="glass-card-interactive rounded-3xl p-5 relative overflow-hidden group">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">ACTIVE DOWNLOADS</span>
        <div class="p-2 rounded-xl bg-cyan-500/10 text-cyan-300 border border-cyan-500/20">
          <DownloadCloud class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 flex items-baseline gap-2">
        <span class="text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">{torrents.activeCount}</span>
        <span class="text-xs text-slate-500 dark:text-slate-400 font-mono">/ {torrents.jobs.length} total</span>
      </div>
      <div class="absolute bottom-0 left-0 right-0 h-1.5 bg-slate-200 dark:bg-slate-900">
        <div
          class="h-full bg-cyan-400 shadow-[0_0_8px_rgba(0,229,255,0.6)]"
          style="width: {torrents.jobs.length > 0 ? (torrents.activeCount / torrents.jobs.length) * 100 : 0}%"
        ></div>
      </div>
    </div>

    <!-- Download Speed -->
    <div class="glass-card-interactive rounded-3xl p-5 relative overflow-hidden group">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">TOTAL DOWNLOAD</span>
        <div class="p-2 rounded-xl bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-500/20">
          <Activity class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-transparent bg-clip-text bg-gradient-to-r from-cyan-500 to-sky-400 dark:from-cyan-400 dark:to-sky-300">
        {formatSpeed(torrents.totalDownloadSpeed)}
      </div>
    </div>

    <!-- Upload Speed -->
    <div class="glass-card-interactive rounded-3xl p-5 relative overflow-hidden group">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">TOTAL UPLOAD</span>
        <div class="p-2 rounded-xl bg-emerald-500/10 text-emerald-600 dark:text-emerald-300 border border-emerald-500/20">
          <UploadCloud class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-emerald-600 dark:text-emerald-400">
        {formatSpeed(torrents.totalUploadSpeed)}
      </div>
    </div>

    <!-- Completed Torrents -->
    <div class="glass-card-interactive rounded-3xl p-5 relative overflow-hidden group">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">COMPLETED JOBS</span>
        <div class="p-2 rounded-xl bg-purple-500/10 text-purple-600 dark:text-purple-300 border border-purple-500/20">
          <CheckCircle2 class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">
        {torrents.completedCount}
      </div>
    </div>
  </div>

  <!-- Filters & Controls Deck -->
  <div class="glass-panel p-3.5 rounded-3xl border border-slate-200/80 dark:border-white/10 flex flex-col sm:flex-row justify-between items-center gap-3">
    <!-- Status Filter Buttons -->
    <div class="flex items-center gap-1.5 w-full sm:w-auto overflow-x-auto scrollbar-none pb-1 sm:pb-0">
      {#each ['ALL', 'DOWNLOADING', 'PAUSED', 'COMPLETED'] as const as st}
        <button
          onclick={() => (filterStatus = st)}
          class="btn-expressive px-4 py-1.5 rounded-full text-xs font-mono font-semibold transition-all cursor-pointer whitespace-nowrap {filterStatus === st
            ? 'bg-cyan-500/20 text-cyan-600 dark:text-cyan-300 border border-cyan-400/40 shadow-[0_0_12px_rgba(0,229,255,0.25)]'
            : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 border border-transparent'}"
        >
          {st}
        </button>
      {/each}
    </div>

    <!-- Search Input -->
    <div class="relative w-full sm:w-72">
      <Search class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter by name or hash..."
        class="glass-input w-full rounded-2xl pl-9 pr-8 py-2 text-xs font-mono"
      />
      {#if searchQuery}
        <button
          onclick={() => (searchQuery = '')}
          class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-700 dark:hover:text-white text-xs cursor-pointer"
        >
          ✕
        </button>
      {/if}
    </div>
  </div>

  <!-- Torrents List -->
  {#if torrents.loading && torrents.jobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-cyan-500 dark:border-cyan-400 border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-slate-500 dark:text-slate-400">Loading torrent engine jobs...</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10 space-y-4">
      <DownloadCloud class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">No torrents found</h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto font-mono">
        {#if searchQuery || filterStatus !== 'ALL'}
          No torrent matches the applied filters.
        {:else}
          Your download queue is empty. Explore releases or add a magnet link directly.
        {/if}
      </p>
      <div class="pt-2 flex justify-center gap-3">
        <a
          href="/explore"
          class="btn-expressive px-4 py-2 glass-pill text-slate-700 dark:text-white rounded-2xl text-xs font-mono flex items-center gap-1.5"
        >
          <Compass class="w-3.5 h-3.5" />
          <span>Explore Nyaa Releases</span>
        </a>
        <button
          onclick={() => (isAddModalOpen = true)}
          class="btn-expressive px-4 py-2 bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white font-bold rounded-2xl text-xs font-mono shadow-[0_0_15px_rgba(0,229,255,0.2)]"
        >
          Add Magnet Link
        </button>
      </div>
    </div>
  {:else}
    <div class="space-y-3.5">
      {#each filteredJobs as job (job.id)}
        <div class="glass-card-interactive rounded-3xl p-5 border border-slate-200/80 dark:border-white/10 space-y-3.5">
          <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
            <!-- Left Info -->
            <div class="min-w-0 flex-1 space-y-2">
              <div class="flex items-center gap-2.5 flex-wrap">
                <!-- Status Badge -->
                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold tracking-wider {
                  job.status === 'DOWNLOADING'
                    ? 'bg-cyan-500/20 text-cyan-600 dark:text-cyan-300 border border-cyan-400/40 shadow-[0_0_10px_rgba(0,229,255,0.3)]'
                    : job.status === 'COMPLETED'
                    ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-400/40'
                    : job.status === 'PAUSED'
                    ? 'bg-amber-500/20 text-amber-600 dark:text-amber-300 border border-amber-500/40'
                    : 'bg-rose-500/20 text-rose-600 dark:text-rose-300 border border-rose-500/40'
                }">
                  {job.status}
                </span>

                <h3 class="text-sm font-bold text-slate-900 dark:text-white truncate max-w-xl font-mono" title={job.name}>
                  {job.name}
                </h3>
              </div>

              <!-- Meta info row -->
              <div class="flex items-center gap-4 text-xs font-mono text-slate-500 dark:text-slate-400 flex-wrap">
                <span title="Info Hash" class="flex items-center gap-1 text-slate-400 dark:text-slate-500">
                  <Hash class="w-3 h-3" />
                  <span class="text-slate-600 dark:text-slate-400">{job.info_hash.substring(0, 12)}...</span>
                </span>
                <span>
                  Size: <strong class="text-slate-800 dark:text-white">{formatBytes(job.downloaded_bytes)}</strong> / {formatBytes(job.total_bytes)}
                </span>
                {#if job.status === 'DOWNLOADING'}
                  <span class="text-cyan-600 dark:text-cyan-400 font-bold flex items-center gap-1">
                    <DownloadCloud class="w-3.5 h-3.5" />
                    <span>{formatSpeed(job.download_speed)}</span>
                  </span>
                  <span class="text-emerald-600 dark:text-emerald-400 font-bold flex items-center gap-1">
                    <UploadCloud class="w-3.5 h-3.5" />
                    <span>{formatSpeed(job.upload_speed)}</span>
                  </span>
                  <span class="flex items-center gap-1">
                    <Users class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500" />
                    <span>{job.peers} peers</span>
                  </span>
                  <span class="flex items-center gap-1">
                    <Clock class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500" />
                    <span>ETA: <strong class="text-slate-800 dark:text-white">{formatEta(job.eta_seconds)}</strong></span>
                  </span>
                {/if}
              </div>
            </div>

            <!-- Right Controls -->
            <div class="flex items-center gap-2 self-end lg:self-center">
              {#if job.status === 'DOWNLOADING'}
                <button
                  onclick={() => torrents.pauseTorrent(job.id)}
                  title="Pause download"
                  class="btn-expressive p-2.5 rounded-2xl glass-pill text-amber-300 hover:bg-amber-500/20 border-amber-500/30 cursor-pointer"
                >
                  <Pause class="w-4 h-4 fill-current" />
                </button>
              {:else if job.status === 'PAUSED'}
                <button
                  onclick={() => torrents.resumeTorrent(job.id)}
                  title="Resume download"
                  class="btn-expressive p-2.5 rounded-2xl glass-pill text-cyan-300 hover:bg-cyan-500/20 border-cyan-400/30 cursor-pointer"
                >
                  <Play class="w-4 h-4 fill-current" />
                </button>
              {/if}

              <!-- Delete Button -->
              <button
                onclick={() => openDeleteModal(job)}
                title="Delete torrent"
                class="btn-expressive p-2.5 rounded-2xl glass-pill text-slate-500 dark:text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/30 cursor-pointer"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="space-y-1.5">
            <div class="flex justify-between items-center text-[11px] font-mono text-slate-500 dark:text-slate-400">
              <span>Progress</span>
              <span class="font-bold {job.status === 'COMPLETED' ? 'text-emerald-400' : 'text-cyan-400'}">
                {job.progress_pct.toFixed(1)}%
              </span>
            </div>
            <div class="w-full h-2.5 rounded-full bg-slate-200 dark:bg-slate-950/80 overflow-hidden border border-slate-200/80 dark:border-white/5 relative">
              <div
                class="h-full rounded-full transition-all duration-300 relative {
                  job.status === 'COMPLETED'
                    ? 'bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.5)]'
                    : job.status === 'PAUSED'
                    ? 'bg-amber-400'
                    : 'bg-gradient-to-r from-cyan-400 via-sky-400 to-emerald-400 shadow-[0_0_12px_rgba(0,229,255,0.4)]'
                }"
                style="width: {Math.min(100, Math.max(0, job.progress_pct))}%"
              ></div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Add Magnet Modal -->
{#if isAddModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl animate-fade-in">
    <div class="glass-panel-elevated rounded-3xl p-6 max-w-lg w-full shadow-2xl relative border border-slate-200/80 dark:border-white/15 animate-scale-in">
      <div class="flex justify-between items-center mb-4">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <DownloadCloud class="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
          <span>Add Magnet Link</span>
        </h3>
        <button
          onclick={() => (isAddModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <form onsubmit={handleAddTorrent} class="space-y-4">
        <div>
          <label for="magnet-uri" class="block text-xs font-mono text-slate-600 dark:text-slate-400 mb-1.5 uppercase font-bold">
            MAGNET URI OR TORRENT HASH
          </label>
          <textarea
            id="magnet-uri"
            bind:value={newMagnetUri}
            required
            rows="4"
            placeholder="magnet:?xt=urn:btih:..."
            class="glass-input w-full rounded-2xl p-3.5 text-xs font-mono resize-none"
          ></textarea>
        </div>

        {#if addError}
          <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-xl text-xs font-mono text-rose-600 dark:text-rose-300">
            {addError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isAddModalOpen = false)}
            class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 text-xs font-mono cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isAdding || !newMagnetUri.trim()}
            class="btn-expressive px-5 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white font-bold text-xs font-mono shadow-[0_0_15px_rgba(0,229,255,0.25)] disabled:opacity-40 cursor-pointer flex items-center gap-2"
          >
            {#if isAdding}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Adding...</span>
            {:else}
              <DownloadCloud class="w-4 h-4 stroke-[2.5]" />
              <span>Download</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if isDeleteModalOpen && targetJob}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl animate-fade-in">
    <div class="glass-panel-elevated rounded-3xl p-6 max-w-md w-full shadow-2xl border border-slate-200/80 dark:border-white/15 animate-scale-in space-y-4">
      <div class="flex items-center gap-3">
        <div class="p-2.5 rounded-2xl bg-rose-500/10 text-rose-500 dark:text-rose-400 border border-rose-500/25">
          <Trash2 class="w-5 h-5" />
        </div>
        <div>
          <h3 class="text-lg font-bold text-slate-900 dark:text-white">Delete Torrent?</h3>
          <p class="text-xs text-slate-500 dark:text-slate-400 font-mono truncate max-w-xs" title={targetJob.name}>
            {targetJob.name}
          </p>
        </div>
      </div>

      <div class="p-3.5 glass-panel rounded-2xl border border-slate-200/60 dark:border-white/5">
        <label class="flex items-center gap-2.5 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={deleteFilesOnDisk}
            class="rounded bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700 text-rose-500 focus:ring-0 w-4 h-4"
          />
          <span>Also delete downloaded files from disk</span>
        </label>
      </div>

      <div class="flex justify-end gap-3 pt-2">
        <button
          onclick={() => {
            isDeleteModalOpen = false;
            targetJob = null;
          }}
          class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 text-xs font-mono cursor-pointer"
        >
          Cancel
        </button>
        <button
          onclick={confirmDelete}
          class="btn-expressive px-5 py-2.5 rounded-2xl bg-rose-500 hover:bg-rose-600 text-white font-bold text-xs font-mono shadow-[0_0_15px_rgba(244,63,94,0.3)] cursor-pointer"
        >
          Confirm Delete
        </button>
      </div>
    </div>
  </div>
{/if}
