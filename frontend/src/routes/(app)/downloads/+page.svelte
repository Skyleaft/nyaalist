<script lang="ts">
  import { onMount } from 'svelte';
  import { torrents } from '#lib/stores/torrents.svelte';
  import type { DownloadJob } from '#lib/api/types';

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
    torrents.jobs.filter(job => {
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

<div class="space-y-6">
  <!-- Header & Telemetry KPI Banners -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-white flex items-center gap-3">
        <svg class="w-7 h-7 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
        </svg>
        BitTorrent Downloader
      </h1>
      <p class="text-xs text-gray-400 mt-1 font-mono">
        librqbit telemetry engine & real-time WebSocket monitoring
      </p>
    </div>

    <div class="flex items-center gap-3">
      <!-- WS Status Indicator -->
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-[#1E1E1E] border border-[#2C2C2E] text-xs font-mono">
        <span class="w-2 h-2 rounded-full {torrents.wsConnected ? 'bg-[#32D74B] shadow-[0_0_8px_#32D74B]' : 'bg-[#FF453A]'}"></span>
        <span class="{torrents.wsConnected ? 'text-[#32D74B]' : 'text-gray-400'}">
          {torrents.wsConnected ? 'WS LIVE' : 'OFFLINE'}
        </span>
      </div>

      <button
        onclick={() => (isAddModalOpen = true)}
        class="flex items-center gap-2 px-4 py-2 bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold text-sm rounded-lg transition-all shadow-[0_0_15px_rgba(0,229,255,0.3)] cursor-pointer"
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Add Magnet
      </button>
    </div>
  </div>

  <!-- Telemetry Gauges -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <!-- Active Downloads -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E] relative overflow-hidden">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">ACTIVE DOWNLOADS</span>
        <span class="text-[#00E5FF] text-xs font-mono">LIVE</span>
      </div>
      <div class="mt-2 flex items-baseline gap-2">
        <span class="text-2xl font-bold font-mono text-white">{torrents.activeCount}</span>
        <span class="text-xs text-gray-500 font-mono">/ {torrents.jobs.length} total</span>
      </div>
      <div class="absolute bottom-0 left-0 right-0 h-1 bg-[#2C2C2E]">
        <div
          class="h-full bg-[#00E5FF]"
          style="width: {torrents.jobs.length > 0 ? (torrents.activeCount / torrents.jobs.length) * 100 : 0}%"
        ></div>
      </div>
    </div>

    <!-- Download Speed -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">TOTAL DOWNLOAD RATE</span>
        <svg class="w-4 h-4 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3" />
        </svg>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-[#00E5FF]">
        {formatSpeed(torrents.totalDownloadSpeed)}
      </div>
    </div>

    <!-- Upload Speed -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">TOTAL UPLOAD RATE</span>
        <svg class="w-4 h-4 text-[#32D74B]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
        </svg>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-[#32D74B]">
        {formatSpeed(torrents.totalUploadSpeed)}
      </div>
    </div>

    <!-- Completed Torrents -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">COMPLETED JOBS</span>
        <svg class="w-4 h-4 text-[#32D74B]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-white">
        {torrents.completedCount}
      </div>
    </div>
  </div>

  <!-- Filters & Controls Bar -->
  <div class="flex flex-col sm:flex-row justify-between items-center gap-4 bg-[#1E1E1E] p-3 rounded-xl border border-[#2C2C2E]">
    <!-- Status Filter Buttons -->
    <div class="flex items-center gap-1.5 w-full sm:w-auto overflow-x-auto pb-1 sm:pb-0">
      {#each ['ALL', 'DOWNLOADING', 'PAUSED', 'COMPLETED'] as const as st}
        <button
          onclick={() => (filterStatus = st)}
          class="px-3 py-1.5 rounded-lg text-xs font-mono font-medium transition-all cursor-pointer whitespace-nowrap {filterStatus === st
            ? 'bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40'
            : 'text-gray-400 hover:text-white hover:bg-white/5 border border-transparent'}"
        >
          {st}
        </button>
      {/each}
    </div>

    <!-- Search Input -->
    <div class="relative w-full sm:w-72">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search torrents or info hash..."
        class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-1.5 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-[#00E5FF]"
      />
      {#if searchQuery}
        <button
          onclick={() => (searchQuery = '')}
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-500 hover:text-white text-xs cursor-pointer"
        >
          ✕
        </button>
      {/if}
    </div>
  </div>

  <!-- Torrents List -->
  {#if torrents.loading && torrents.jobs.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-[#00E5FF] border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-gray-400">Loading torrent engine jobs...</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <svg class="w-12 h-12 mx-auto text-gray-600 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10" />
      </svg>
      <h3 class="text-base font-semibold text-white">No torrents found</h3>
      <p class="text-xs text-gray-400 mt-1 max-w-sm mx-auto">
        {#if searchQuery || filterStatus !== 'ALL'}
          No torrent matches the applied filters.
        {:else}
          Your download queue is empty. Explore releases or add a magnet link directly.
        {/if}
      </p>
      <div class="mt-4 flex justify-center gap-3">
        <a
          href="/explore"
          class="px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-lg text-xs font-mono border border-[#2C2C2E]"
        >
          Explore Nyaa Releases
        </a>
        <button
          onclick={() => (isAddModalOpen = true)}
          class="px-4 py-2 bg-[#00E5FF] text-black font-semibold rounded-lg text-xs font-mono"
        >
          Add Magnet Link
        </button>
      </div>
    </div>
  {:else}
    <div class="space-y-3">
      {#each filteredJobs as job (job.id)}
        <div class="bg-[#1E1E1E] border border-[#2C2C2E] hover:border-[#3C3C3E] rounded-xl p-4 transition-all">
          <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3">
            <!-- Left Info -->
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2.5 flex-wrap">
                <!-- Status Badge -->
                <span class="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider {
                  job.status === 'DOWNLOADING'
                    ? 'bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40 shadow-[0_0_8px_rgba(0,229,255,0.2)]'
                    : job.status === 'COMPLETED'
                    ? 'bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/40'
                    : job.status === 'PAUSED'
                    ? 'bg-amber-500/20 text-amber-400 border border-amber-500/40'
                    : 'bg-[#FF453A]/20 text-[#FF453A] border border-[#FF453A]/40'
                }">
                  {job.status}
                </span>

                <h3 class="text-sm font-semibold text-white truncate max-w-xl" title={job.name}>
                  {job.name}
                </h3>
              </div>

              <!-- Meta info row -->
              <div class="flex items-center gap-4 text-xs font-mono text-gray-400 mt-2 flex-wrap">
                <span title="Info Hash" class="text-gray-500">
                  HASH: <span class="text-gray-400">{job.info_hash.substring(0, 12)}...</span>
                </span>
                <span>
                  SIZE: <span class="text-white">{formatBytes(job.downloaded_bytes)}</span> / {formatBytes(job.total_bytes)}
                </span>
                {#if job.status === 'DOWNLOADING'}
                  <span class="text-[#00E5FF]">
                    ↓ {formatSpeed(job.download_speed)}
                  </span>
                  <span class="text-[#32D74B]">
                    ↑ {formatSpeed(job.upload_speed)}
                  </span>
                  <span>
                    PEERS: {job.peers}
                  </span>
                  <span>
                    ETA: {formatEta(job.eta_seconds)}
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
                  class="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-amber-400 border border-[#2C2C2E] cursor-pointer"
                >
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
                  </svg>
                </button>
              {:else if job.status === 'PAUSED'}
                <button
                  onclick={() => torrents.resumeTorrent(job.id)}
                  title="Resume download"
                  class="p-2 rounded-lg bg-[#00E5FF]/10 hover:bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40 cursor-pointer"
                >
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M8 5v14l11-7z"/>
                  </svg>
                </button>
              {/if}

              <!-- Delete Button -->
              <button
                onclick={() => openDeleteModal(job)}
                title="Delete torrent"
                class="p-2 rounded-lg bg-white/5 hover:bg-[#FF453A]/20 text-gray-400 hover:text-[#FF453A] border border-[#2C2C2E] cursor-pointer"
              >
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="mt-3">
            <div class="flex justify-between items-center text-[11px] font-mono text-gray-400 mb-1">
              <span>Progress</span>
              <span class="font-bold {job.status === 'COMPLETED' ? 'text-[#32D74B]' : 'text-[#00E5FF]'}">
                {job.progress_pct.toFixed(1)}%
              </span>
            </div>
            <div class="w-full h-2 rounded-full bg-[#121212] overflow-hidden border border-[#2C2C2E]">
              <div
                class="h-full rounded-full transition-all duration-300 {
                  job.status === 'COMPLETED'
                    ? 'bg-[#32D74B]'
                    : job.status === 'PAUSED'
                    ? 'bg-amber-400'
                    : 'bg-gradient-to-r from-[#00E5FF] to-[#32D74B]'
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
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 max-w-lg w-full shadow-2xl relative">
      <div class="flex justify-between items-center mb-4">
        <h3 class="text-lg font-bold text-white flex items-center gap-2">
          <svg class="w-5 h-5 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          Add Magnet Link
        </h3>
        <button
          onclick={() => (isAddModalOpen = false)}
          class="text-gray-400 hover:text-white cursor-pointer"
        >
          ✕
        </button>
      </div>

      <form onsubmit={handleAddTorrent} class="space-y-4">
        <div>
          <label for="magnet-uri" class="block text-xs font-mono text-gray-400 mb-1.5">MAGNET URI OR URL</label>
          <textarea
            id="magnet-uri"
            bind:value={newMagnetUri}
            required
            rows="4"
            placeholder="magnet:?xt=urn:btih:..."
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg p-3 text-xs font-mono text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] resize-none"
          ></textarea>
        </div>

        {#if addError}
          <div class="p-3 bg-[#FF453A]/10 border border-[#FF453A]/30 rounded-lg text-xs font-mono text-[#FF453A]">
            {addError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isAddModalOpen = false)}
            class="px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 text-xs font-mono cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isAdding || !newMagnetUri.trim()}
            class="px-5 py-2 rounded-lg bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold text-xs font-mono transition-all disabled:opacity-50 cursor-pointer flex items-center gap-2"
          >
            {#if isAdding}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-black border-t-transparent rounded-full"></span>
              Adding...
            {:else}
              Download
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if isDeleteModalOpen && targetJob}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 max-w-md w-full shadow-2xl">
      <h3 class="text-lg font-bold text-white mb-2">Delete Torrent?</h3>
      <p class="text-xs text-gray-400 font-mono mb-4 truncate" title={targetJob.name}>
        {targetJob.name}
      </p>

      <div class="mb-5 p-3 bg-[#121212] rounded-lg border border-[#2C2C2E]">
        <label class="flex items-center gap-2.5 text-xs text-gray-300 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={deleteFilesOnDisk}
            class="rounded bg-[#1E1E1E] border-gray-600 text-[#FF453A] focus:ring-0"
          />
          <span>Also delete downloaded files from disk</span>
        </label>
      </div>

      <div class="flex justify-end gap-3">
        <button
          onclick={() => {
            isDeleteModalOpen = false;
            targetJob = null;
          }}
          class="px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 text-xs font-mono cursor-pointer"
        >
          Cancel
        </button>
        <button
          onclick={confirmDelete}
          class="px-4 py-2 rounded-lg bg-[#FF453A] hover:bg-[#FF453A]/90 text-white font-semibold text-xs font-mono cursor-pointer"
        >
          Confirm Delete
        </button>
      </div>
    </div>
  </div>
{/if}
