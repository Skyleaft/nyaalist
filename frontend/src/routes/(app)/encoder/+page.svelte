<script lang="ts">
  import { onMount } from 'svelte';
  import { encoder } from '#lib/stores/encoder.svelte';
  import type { EncodingJobItem, EncodingPreset } from '#lib/api/types';

  let filterStatus = $state<string>('ALL');
  let searchQuery = $state('');

  // Queue Job Modal state
  let isQueueModalOpen = $state(false);
  let newSourcePath = $state('');
  let newPresetId = $state('');
  let newTargetPath = $state('');
  let queueError = $state<string | null>(null);
  let isQueueing = $state(false);

  // Preset Modal state
  let isPresetsModalOpen = $state(false);
  let isCreatingPreset = $state(false);
  let presetError = $state<string | null>(null);
  let presetName = $state('');
  let presetDesc = $state('');
  let presetCodec = $state('libx265');
  let presetSpeed = $state('medium');
  let presetCrf = $state(24);
  let presetRes = $state('1280x720');
  let presetAudio = $state('copy');
  let presetContainer = $state('mkv');
  let presetIsDefault = $state(false);

  onMount(() => {
    encoder.init();
  });

  const filteredJobs = $derived(
    encoder.jobs.filter(job => {
      if (filterStatus !== 'ALL' && job.status !== filterStatus) return false;
      if (searchQuery.trim().length > 0) {
        const q = searchQuery.toLowerCase();
        return (
          job.source_filename.toLowerCase().includes(q) ||
          job.preset_name.toLowerCase().includes(q) ||
          job.source_file_path.toLowerCase().includes(q)
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

  function formatTime(sec: number): string {
    if (!sec || sec <= 0) return '00:00';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  function formatEta(seconds: number | null | undefined): string {
    if (!seconds || seconds <= 0) return '--';
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}m ${s}s`;
  }

  function calcSavingsPct(source: number, encoded: number): number {
    if (!source || source <= 0 || !encoded || encoded <= 0 || encoded >= source) return 0;
    return Math.round(((source - encoded) / source) * 100);
  }

  async function handleQueueJob(e: Event) {
    e.preventDefault();
    if (!newSourcePath.trim()) return;

    isQueueing = true;
    queueError = null;
    try {
      await encoder.queueJob({
        source_file_path: newSourcePath.trim(),
        preset_id: newPresetId || undefined,
        target_file_path: newTargetPath.trim() || undefined,
      });
      newSourcePath = '';
      newTargetPath = '';
      isQueueModalOpen = false;
    } catch (err: any) {
      queueError = err.message || 'Failed to queue encoding job';
    } finally {
      isQueueing = false;
    }
  }

  async function handleCreatePreset(e: Event) {
    e.preventDefault();
    if (!presetName.trim()) return;

    isCreatingPreset = true;
    presetError = null;
    try {
      await encoder.createPreset({
        name: presetName.trim(),
        description: presetDesc.trim() || undefined,
        video_codec: presetCodec,
        preset: presetSpeed,
        crf: presetCrf,
        target_resolution: presetRes.trim() || undefined,
        audio_codec: presetAudio,
        container: presetContainer,
        is_default: presetIsDefault,
      });
      presetName = '';
      presetDesc = '';
      presetIsDefault = false;
    } catch (err: any) {
      presetError = err.message || 'Failed to create preset';
    } finally {
      isCreatingPreset = false;
    }
  }

  async function handleCancel(job: EncodingJobItem) {
    try {
      await encoder.cancelJob(job.id);
    } catch (err: any) {
      alert(`Cancel failed: ${err.message}`);
    }
  }

  async function handleDelete(job: EncodingJobItem) {
    if (!confirm(`Delete job record for ${job.source_filename}?`)) return;
    try {
      await encoder.deleteJob(job.id);
    } catch (err: any) {
      alert(`Delete failed: ${err.message}`);
    }
  }
</script>

<svelte:head>
  <title>Video Encoder Telemetry | Nyaalist</title>
</svelte:head>

<div class="space-y-6">
  <!-- Header & Action Buttons -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-white flex items-center gap-3">
        <svg class="w-7 h-7 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
        </svg>
        FFmpeg Video Encoder
      </h1>
      <p class="text-xs text-gray-400 mt-1 font-mono">
        Hardware-accelerated anime transcode pipeline with live telemetry streaming
      </p>
    </div>

    <div class="flex items-center gap-3">
      <!-- WS Status -->
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-[#1E1E1E] border border-[#2C2C2E] text-xs font-mono">
        <span class="w-2 h-2 rounded-full {encoder.wsConnected ? 'bg-[#32D74B] shadow-[0_0_8px_#32D74B]' : 'bg-[#FF453A]'}"></span>
        <span class="{encoder.wsConnected ? 'text-[#32D74B]' : 'text-gray-400'}">
          {encoder.wsConnected ? 'WS LIVE' : 'OFFLINE'}
        </span>
      </div>

      <button
        onclick={() => (isPresetsModalOpen = true)}
        class="px-3.5 py-2 rounded-lg bg-[#1E1E1E] hover:bg-[#252525] border border-[#2C2C2E] text-white font-mono text-xs cursor-pointer"
      >
        Presets ({encoder.presets.length})
      </button>

      <button
        onclick={() => (isQueueModalOpen = true)}
        class="flex items-center gap-2 px-4 py-2 bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold text-xs font-mono rounded-lg transition-all shadow-[0_0_15px_rgba(0,229,255,0.3)] cursor-pointer"
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Queue Transcode
      </button>
    </div>
  </div>

  <!-- Active Transcoding Hero Telemetry (Wireframe 1 Style) -->
  {#if encoder.activeJob || (encoder.activeTelemetry && encoder.activeTelemetry.status === 'ENCODING')}
    {@const job = encoder.activeJob}
    {@const tel = encoder.activeTelemetry}
    <div class="bg-[#1E1E1E] border border-[#00E5FF]/40 rounded-2xl p-5 shadow-[0_0_20px_rgba(0,229,255,0.15)] relative overflow-hidden">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-3 mb-4">
        <div>
          <div class="flex items-center gap-2.5">
            <span class="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40 shadow-[0_0_8px_rgba(0,229,255,0.3)] animate-pulse">
              TRANSCODING LIVE
            </span>
            <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-white/5 border border-[#2C2C2E] text-gray-300">
              {tel?.preset_name || job?.preset_name}
            </span>
          </div>
          <h2 class="text-base font-bold text-white mt-2 truncate max-w-2xl">
            {tel?.file_name || job?.source_filename}
          </h2>
        </div>

        {#if job}
          <button
            onclick={() => handleCancel(job)}
            class="self-start md:self-center px-3 py-1.5 rounded-lg bg-[#FF453A]/20 hover:bg-[#FF453A]/30 text-[#FF453A] border border-[#FF453A]/40 text-xs font-mono cursor-pointer"
          >
            Stop Transcode
          </button>
        {/if}
      </div>

      <!-- Live Progress Bar -->
      <div class="space-y-1.5">
        <div class="flex justify-between text-xs font-mono">
          <span class="text-gray-400">Progress</span>
          <span class="text-[#00E5FF] font-bold">
            {(tel?.progress_pct ?? job?.progress_pct ?? 0).toFixed(1)}%
          </span>
        </div>
        <div class="w-full h-3 rounded-full bg-[#121212] overflow-hidden border border-[#2C2C2E]">
          <div
            class="h-full rounded-full bg-gradient-to-r from-[#00E5FF] to-[#32D74B] transition-all duration-300"
            style="width: {Math.min(100, Math.max(0, tel?.progress_pct ?? job?.progress_pct ?? 0))}%"
          ></div>
        </div>
      </div>

      <!-- Live Telemetry Matrix -->
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 mt-4 pt-4 border-t border-[#2C2C2E]/60 text-xs font-mono">
        <div>
          <span class="text-gray-500 block text-[10px]">FPS</span>
          <span class="text-white font-bold">{(tel?.fps ?? job?.fps ?? 0).toFixed(1)}</span>
        </div>
        <div>
          <span class="text-gray-500 block text-[10px]">SPEED</span>
          <span class="text-[#00E5FF] font-bold">{(tel?.speed ?? job?.speed ?? 0).toFixed(2)}x</span>
        </div>
        <div>
          <span class="text-gray-500 block text-[10px]">FRAME</span>
          <span class="text-gray-300">{tel?.current_frame ?? job?.current_frame ?? 0}</span>
        </div>
        <div>
          <span class="text-gray-500 block text-[10px]">TIME / DURATION</span>
          <span class="text-gray-300">
            {formatTime(tel?.current_time_sec ?? job?.current_time_sec ?? 0)} / {formatTime(tel?.total_duration_sec ?? job?.total_duration_sec ?? 0)}
          </span>
        </div>
        <div>
          <span class="text-gray-500 block text-[10px]">ETA</span>
          <span class="text-[#32D74B] font-bold">{formatEta(tel?.eta_seconds ?? job?.eta_seconds)}</span>
        </div>
        <div>
          <span class="text-gray-500 block text-[10px]">CURRENT SIZE</span>
          <span class="text-white">
            {formatBytes(tel?.current_size_bytes ?? job?.encoded_size_bytes ?? 0)}
          </span>
        </div>
      </div>
    </div>
  {/if}

  <!-- Telemetry KPI Banners -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <!-- Active Transcoding -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">ACTIVE ENCODER</span>
        <span class="text-xs font-mono text-[#00E5FF]">GPU / CPU</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-white">
        {encoder.activeJob ? 1 : 0}
      </div>
    </div>

    <!-- Queue Depth -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">QUEUE DEPTH</span>
        <span class="text-xs font-mono text-amber-400">PENDING</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-amber-400">
        {encoder.queuedCount}
      </div>
    </div>

    <!-- Completed Transcodes -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">COMPLETED JOBS</span>
        <span class="text-xs font-mono text-[#32D74B]">FINISHED</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-[#32D74B]">
        {encoder.completedCount}
      </div>
    </div>

    <!-- Space Saved -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">TOTAL SPACE SAVED</span>
        <span class="text-xs font-mono text-[#32D74B]">EFFICIENCY</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-[#32D74B]">
        {formatBytes(encoder.totalSavedBytes)}
      </div>
    </div>
  </div>

  <!-- Filters & Controls Bar -->
  <div class="flex flex-col sm:flex-row justify-between items-center gap-4 bg-[#1E1E1E] p-3 rounded-xl border border-[#2C2C2E]">
    <div class="flex items-center gap-1.5 w-full sm:w-auto overflow-x-auto pb-1 sm:pb-0">
      {#each ['ALL', 'QUEUED', 'ENCODING', 'COMPLETED', 'FAILED'] as const as st}
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

    <div class="relative w-full sm:w-72">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search jobs by filename or preset..."
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

  <!-- Jobs Queue & History Table -->
  {#if encoder.loading && encoder.jobs.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-[#00E5FF] border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-gray-400">Loading encoding queue...</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <svg class="w-12 h-12 mx-auto text-gray-600 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
      </svg>
      <h3 class="text-base font-semibold text-white">No encoding jobs</h3>
      <p class="text-xs text-gray-400 mt-1 max-w-sm mx-auto">
        Queue a video transcode to compress large anime releases using HEVC / x265 presets.
      </p>
      <div class="mt-4">
        <button
          onclick={() => (isQueueModalOpen = true)}
          class="px-4 py-2 bg-[#00E5FF] text-black font-semibold rounded-lg text-xs font-mono cursor-pointer"
        >
          Queue First Transcode
        </button>
      </div>
    </div>
  {:else}
    <div class="space-y-3">
      {#each filteredJobs as item (item.id)}
        <div class="bg-[#1E1E1E] border border-[#2C2C2E] hover:border-[#3C3C3E] rounded-xl p-4 transition-all">
          <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2.5 flex-wrap">
                <!-- Status Badge -->
                <span class="px-2 py-0.5 rounded text-[10px] font-mono font-bold {
                  item.status === 'ENCODING'
                    ? 'bg-[#00E5FF]/20 text-[#00E5FF] border border-[#00E5FF]/40 shadow-[0_0_8px_rgba(0,229,255,0.2)]'
                    : item.status === 'COMPLETED'
                    ? 'bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/40'
                    : item.status === 'QUEUED'
                    ? 'bg-amber-500/20 text-amber-400 border border-amber-500/40'
                    : 'bg-[#FF453A]/20 text-[#FF453A] border border-[#FF453A]/40'
                }">
                  {item.status}
                </span>

                <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-white/5 border border-[#2C2C2E] text-gray-300">
                  {item.preset_name}
                </span>

                <h3 class="text-sm font-semibold text-white truncate max-w-xl" title={item.source_file_path}>
                  {item.source_filename}
                </h3>
              </div>

              <!-- Metrics Row -->
              <div class="flex items-center gap-4 text-xs font-mono text-gray-400 mt-2 flex-wrap">
                <span>
                  SOURCE: <span class="text-gray-300">{formatBytes(item.source_size_bytes)}</span>
                </span>
                {#if item.encoded_size_bytes > 0}
                  <span>
                    OUTPUT: <span class="text-[#00E5FF]">{formatBytes(item.encoded_size_bytes)}</span>
                  </span>
                  {#if item.status === 'COMPLETED'}
                    {@const saved = calcSavingsPct(item.source_size_bytes, item.encoded_size_bytes)}
                    {#if saved > 0}
                      <span class="text-[#32D74B] font-bold">
                        Saved: {saved}% size
                      </span>
                    {/if}
                  {/if}
                {/if}

                {#if item.status === 'ENCODING'}
                  <span class="text-[#00E5FF]">
                    {item.speed.toFixed(2)}x speed
                  </span>
                  <span>
                    {item.fps.toFixed(1)} fps
                  </span>
                  <span>
                    ETA: {formatEta(item.eta_seconds)}
                  </span>
                {/if}

                {#if item.error_message}
                  <span class="text-[#FF453A] truncate max-w-sm" title={item.error_message}>
                    Error: {item.error_message}
                  </span>
                {/if}
              </div>
            </div>

            <!-- Controls -->
            <div class="flex items-center gap-2 self-end lg:self-center font-mono">
              {#if item.status === 'QUEUED' || item.status === 'ENCODING'}
                <button
                  onclick={() => handleCancel(item)}
                  class="px-2.5 py-1 rounded-lg bg-white/5 hover:bg-[#FF453A]/20 text-gray-300 hover:text-[#FF453A] border border-[#2C2C2E] text-xs cursor-pointer"
                  title="Cancel Job"
                >
                  Cancel
                </button>
              {/if}

              <button
                onclick={() => handleDelete(item)}
                class="p-1.5 rounded-lg bg-white/5 hover:bg-[#FF453A]/20 text-gray-400 hover:text-[#FF453A] border border-[#2C2C2E] text-xs cursor-pointer"
                title="Delete Record"
              >
                ✕
              </button>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="mt-3">
            <div class="flex justify-between items-center text-[11px] font-mono text-gray-400 mb-1">
              <span>Transcode Progress</span>
              <span class="font-bold {item.status === 'COMPLETED' ? 'text-[#32D74B]' : 'text-[#00E5FF]'}">
                {item.progress_pct.toFixed(1)}%
              </span>
            </div>
            <div class="w-full h-2 rounded-full bg-[#121212] overflow-hidden border border-[#2C2C2E]">
              <div
                class="h-full rounded-full transition-all duration-300 {
                  item.status === 'COMPLETED'
                    ? 'bg-[#32D74B]'
                    : item.status === 'FAILED'
                    ? 'bg-[#FF453A]'
                    : item.status === 'QUEUED'
                    ? 'bg-amber-400'
                    : 'bg-gradient-to-r from-[#00E5FF] to-[#32D74B]'
                }"
                style="width: {Math.min(100, Math.max(0, item.progress_pct))}%"
              ></div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Queue Transcode Modal -->
{#if isQueueModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 max-w-lg w-full shadow-2xl relative space-y-4">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-white flex items-center gap-2">
          <svg class="w-5 h-5 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          Queue Video Transcode
        </h3>
        <button
          onclick={() => (isQueueModalOpen = false)}
          class="text-gray-400 hover:text-white cursor-pointer"
        >
          ✕
        </button>
      </div>

      <form onsubmit={handleQueueJob} class="space-y-4 text-xs font-mono">
        <div>
          <label for="source-path" class="block text-gray-400 mb-1">SOURCE FILE PATH</label>
          <input
            id="source-path"
            type="text"
            required
            bind:value={newSourcePath}
            placeholder="e.g. /data/downloads/Odekake Kozame - 93 [1080p].mkv"
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF]"
          />
        </div>

        <div>
          <label for="preset-select" class="block text-gray-400 mb-1">ENCODING PRESET</label>
          <select
            id="preset-select"
            bind:value={newPresetId}
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-[#00E5FF]"
          >
            {#each encoder.presets as preset}
              <option value={preset.id}>
                {preset.name} ({preset.video_codec}, CRF {preset.crf}, {preset.target_resolution || 'source res'}) {preset.is_default ? '★ Default' : ''}
              </option>
            {/each}
          </select>
        </div>

        <div>
          <label for="target-path" class="block text-gray-400 mb-1">TARGET OUTPUT PATH (OPTIONAL)</label>
          <input
            id="target-path"
            type="text"
            bind:value={newTargetPath}
            placeholder="Auto-generated in /data/temp_encode if left empty"
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF]"
          />
        </div>

        {#if queueError}
          <div class="p-3 bg-[#FF453A]/10 border border-[#FF453A]/30 rounded-lg text-[#FF453A]">
            {queueError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isQueueModalOpen = false)}
            class="px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isQueueing || !newSourcePath.trim()}
            class="px-5 py-2 rounded-lg bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold transition-all disabled:opacity-50 cursor-pointer flex items-center gap-2"
          >
            {#if isQueueing}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-black border-t-transparent rounded-full"></span>
              Queueing...
            {:else}
              Start Transcode
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Presets Management Modal -->
{#if isPresetsModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 max-w-2xl w-full shadow-2xl relative space-y-5">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-white flex items-center gap-2">
          <svg class="w-5 h-5 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          </svg>
          Encoding Presets
        </h3>
        <button
          onclick={() => (isPresetsModalOpen = false)}
          class="text-gray-400 hover:text-white cursor-pointer"
        >
          ✕
        </button>
      </div>

      <!-- Presets List -->
      <div class="space-y-2 max-h-56 overflow-y-auto pr-1">
        {#each encoder.presets as preset}
          <div class="p-3 bg-[#121212] border border-[#2C2C2E] rounded-xl flex items-center justify-between text-xs font-mono">
            <div>
              <div class="flex items-center gap-2">
                <span class="font-bold text-white">{preset.name}</span>
                {#if preset.is_default}
                  <span class="px-1.5 py-0.2 rounded bg-[#00E5FF]/20 text-[#00E5FF] text-[10px]">DEFAULT</span>
                {/if}
              </div>
              <div class="text-[11px] text-gray-400 mt-1">
                {preset.video_codec} • {preset.preset} • CRF {preset.crf} • {preset.target_resolution || 'source'} • Audio: {preset.audio_codec}
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Add New Preset Form -->
      <form onsubmit={handleCreatePreset} class="p-4 bg-[#121212] rounded-xl border border-[#2C2C2E] space-y-3 text-xs font-mono">
        <h4 class="font-bold text-white uppercase text-[11px] text-[#00E5FF]">Create Custom Preset</h4>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label for="p-name" class="block text-gray-400 mb-1">NAME</label>
            <input
              id="p-name"
              type="text"
              required
              bind:value={presetName}
              placeholder="e.g. 720p x265 Anime"
              class="w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded px-2.5 py-1.5 text-white"
            />
          </div>
          <div>
            <label for="p-res" class="block text-gray-400 mb-1">RESOLUTION (e.g. 1280x720)</label>
            <input
              id="p-res"
              type="text"
              bind:value={presetRes}
              placeholder="1280x720 (empty for source)"
              class="w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded px-2.5 py-1.5 text-white"
            />
          </div>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <div>
            <label for="p-codec" class="block text-gray-400 mb-1">CODEC</label>
            <select
              id="p-codec"
              bind:value={presetCodec}
              class="w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded px-2 py-1.5 text-white"
            >
              <option value="libx265">libx265 (HEVC)</option>
              <option value="libx264">libx264 (AVC)</option>
              <option value="libsvtav1">libsvtav1 (AV1)</option>
            </select>
          </div>
          <div>
            <label for="p-crf" class="block text-gray-400 mb-1">CRF (18-28)</label>
            <input
              id="p-crf"
              type="number"
              bind:value={presetCrf}
              min="0"
              max="51"
              class="w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded px-2.5 py-1.5 text-white"
            />
          </div>
          <div>
            <label for="p-speed" class="block text-gray-400 mb-1">PRESET SPEED</label>
            <select
              id="p-speed"
              bind:value={presetSpeed}
              class="w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded px-2 py-1.5 text-white"
            >
              <option value="fast">fast</option>
              <option value="medium">medium</option>
              <option value="slow">slow</option>
            </select>
          </div>
        </div>

        <div class="flex items-center gap-2 pt-1">
          <input
            id="p-default"
            type="checkbox"
            bind:checked={presetIsDefault}
            class="rounded bg-[#1E1E1E] border-gray-600 text-[#00E5FF]"
          />
          <label for="p-default" class="text-gray-300">Set as default preset</label>
        </div>

        {#if presetError}
          <div class="p-2 bg-[#FF453A]/10 text-[#FF453A] rounded border border-[#FF453A]/30">
            {presetError}
          </div>
        {/if}

        <div class="flex justify-end pt-2">
          <button
            type="submit"
            disabled={isCreatingPreset || !presetName.trim()}
            class="px-4 py-1.5 rounded bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold disabled:opacity-50 cursor-pointer"
          >
            Save Preset
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
