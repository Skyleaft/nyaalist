<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { encoder } from '#lib/stores/encoder.svelte';
  import type { EncodingJobItem, EncodingPreset } from '#lib/api/types';
  import {
    Film,
    Cpu,
    Clock,
    Zap,
    Plus,
    Sliders,
    Search,
    X,
    CheckCircle2,
    Play,
    Trash2,
    Activity,
    HardDrive,
    AlertCircle
  } from 'lucide-svelte';

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
    encoder.jobs.filter((job: any) => {
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

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-6">
  <!-- Header & Telemetry Actions -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-white/10">
    <div class="flex items-center gap-4">
      <div class="p-3.5 rounded-2xl bg-purple-500/10 text-purple-300 border border-purple-500/25 shadow-[0_0_20px_rgba(168,85,247,0.2)]">
        <Film class="w-7 h-7" />
      </div>
      <div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white">
          FFmpeg Video Encoder
        </h1>
        <p class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-0.5">
          Hardware-accelerated anime transcode pipeline with live telemetry streaming
        </p>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <!-- WS Status Pill -->
      <div class="glass-pill px-3.5 py-2 rounded-2xl flex items-center gap-2 text-xs font-mono">
        <div class="relative flex items-center justify-center">
          <div class="w-2.5 h-2.5 rounded-full {encoder.wsConnected ? 'bg-emerald-400' : 'bg-rose-500'}"></div>
          {#if encoder.wsConnected}
            <div class="absolute w-4 h-4 rounded-full bg-emerald-400/30 animate-ping"></div>
          {/if}
        </div>
        <span class="font-medium {encoder.wsConnected ? 'text-emerald-500 dark:text-emerald-300' : 'text-rose-500 dark:text-rose-400'}">
          {encoder.wsConnected ? 'WS LIVE' : 'OFFLINE'}
        </span>
      </div>

      <button
        onclick={() => (isPresetsModalOpen = true)}
        class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-white font-mono text-xs cursor-pointer flex items-center gap-1.5"
      >
        <Sliders class="w-3.5 h-3.5" />
        <span>Presets ({encoder.presets.length})</span>
      </button>

      <button
        onclick={() => (isQueueModalOpen = true)}
        class="btn-expressive px-4 py-2.5 rounded-2xl bg-gradient-to-r from-purple-500 to-indigo-500 hover:from-purple-400 hover:to-indigo-400 text-white font-bold text-xs font-mono shadow-[0_0_20px_rgba(168,85,247,0.3)] flex items-center gap-2 cursor-pointer"
      >
        <Plus class="w-4 h-4 stroke-[2.5]" />
        <span>Queue Transcode</span>
      </button>
    </div>
  </div>

  <!-- Active Transcoding Hero Telemetry -->
  {#if encoder.activeJob || (encoder.activeTelemetry && encoder.activeTelemetry.status === 'ENCODING')}
    {@const job = encoder.activeJob}
    {@const tel = encoder.activeTelemetry}
    <div class="glass-panel-elevated rounded-3xl p-6 shadow-2xl relative overflow-hidden border border-purple-500/30 space-y-4">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
        <div>
          <div class="flex items-center gap-2.5">
            <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-purple-500/20 text-purple-600 dark:text-purple-300 border border-purple-500/40 shadow-[0_0_12px_rgba(168,85,247,0.3)] animate-pulse">
              TRANSCODING LIVE
            </span>
            <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-slate-700 dark:text-slate-300">
              {tel?.preset_name || job?.preset_name}
            </span>
          </div>
          <h2 class="text-base sm:text-lg font-bold text-slate-900 dark:text-white mt-2 truncate max-w-2xl font-mono">
            {tel?.file_name || job?.source_filename}
          </h2>
        </div>

        {#if job}
          <button
            onclick={() => handleCancel(job)}
            class="btn-expressive self-start md:self-center px-4 py-1.5 rounded-2xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-600 dark:text-rose-300 border border-rose-500/40 text-xs font-mono cursor-pointer"
          >
            Stop Transcode
          </button>
        {/if}
      </div>

      <!-- Live Progress Bar -->
      <div class="space-y-1.5">
        <div class="flex justify-between text-xs font-mono">
          <span class="text-slate-500 dark:text-slate-400">Progress</span>
          <span class="text-purple-600 dark:text-purple-400 font-bold">
            {(tel?.progress_pct ?? job?.progress_pct ?? 0).toFixed(1)}%
          </span>
        </div>
        <div class="w-full h-3 rounded-full bg-slate-200 dark:bg-slate-950/80 overflow-hidden border border-slate-200 dark:border-white/5 relative">
          <div
            class="h-full rounded-full bg-gradient-to-r from-purple-500 via-indigo-400 to-cyan-400 transition-all duration-300 shadow-[0_0_15px_rgba(168,85,247,0.5)]"
            style="width: {Math.min(100, Math.max(0, tel?.progress_pct ?? job?.progress_pct ?? 0))}%"
          ></div>
        </div>
      </div>

      <!-- Live Telemetry Matrix -->
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 pt-3 border-t border-slate-200/80 dark:border-white/5 text-xs font-mono text-center">
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">FPS</span>
          <span class="text-slate-900 dark:text-white font-bold text-sm mt-0.5">{(tel?.fps ?? job?.fps ?? 0).toFixed(1)}</span>
        </div>
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">SPEED</span>
          <span class="text-purple-600 dark:text-purple-300 font-bold text-sm mt-0.5">{(tel?.speed ?? job?.speed ?? 0).toFixed(2)}x</span>
        </div>
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">FRAME</span>
          <span class="text-slate-800 dark:text-slate-200 text-sm mt-0.5">{tel?.current_frame ?? job?.current_frame ?? 0}</span>
        </div>
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">TIME / DURATION</span>
          <span class="text-slate-700 dark:text-slate-300 text-xs mt-1 block">
            {formatTime(tel?.current_time_sec ?? job?.current_time_sec ?? 0)} / {formatTime(tel?.total_duration_sec ?? job?.total_duration_sec ?? 0)}
          </span>
        </div>
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">ETA</span>
          <span class="text-emerald-600 dark:text-emerald-400 font-bold text-sm mt-0.5">{formatEta(tel?.eta_seconds ?? job?.eta_seconds)}</span>
        </div>
        <div class="glass-panel p-2.5 rounded-2xl border border-slate-200/60 dark:border-white/5">
          <span class="text-slate-500 dark:text-slate-400 block text-[10px] uppercase font-bold">CURRENT SIZE</span>
          <span class="text-cyan-600 dark:text-cyan-300 font-bold text-sm mt-0.5">
            {formatBytes(tel?.current_size_bytes ?? job?.encoded_size_bytes ?? 0)}
          </span>
        </div>
      </div>
    </div>
  {/if}

  <!-- Telemetry KPI Banners -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">ACTIVE ENCODER</span>
        <div class="p-2 rounded-xl bg-purple-500/10 text-purple-600 dark:text-purple-300 border border-purple-500/20">
          <Cpu class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">
        {encoder.activeJob ? 1 : 0}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">QUEUE DEPTH</span>
        <div class="p-2 rounded-xl bg-amber-500/10 text-amber-300 border border-amber-500/20">
          <Clock class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-amber-300">
        {encoder.queuedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">COMPLETED JOBS</span>
        <div class="p-2 rounded-xl bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
          <CheckCircle2 class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-emerald-400">
        {encoder.completedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">TOTAL SPACE SAVED</span>
        <div class="p-2 rounded-xl bg-cyan-500/10 text-cyan-300 border border-cyan-500/20">
          <HardDrive class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-cyan-300">
        {formatBytes(encoder.totalSavedBytes)}
      </div>
    </div>
  </div>

  <!-- Filters & Controls Bar -->
  <div class="glass-panel p-3.5 rounded-3xl border border-slate-200/80 dark:border-white/10 flex flex-col sm:flex-row justify-between items-center gap-3">
    <div class="flex items-center gap-1.5 w-full sm:w-auto overflow-x-auto scrollbar-none pb-1 sm:pb-0">
      {#each ['ALL', 'QUEUED', 'ENCODING', 'COMPLETED', 'FAILED'] as const as st}
        <button
          onclick={() => (filterStatus = st)}
          class="btn-expressive px-4 py-1.5 rounded-full text-xs font-mono font-semibold transition-all cursor-pointer whitespace-nowrap {filterStatus === st
            ? 'bg-purple-500/20 text-purple-600 dark:text-purple-300 border border-purple-500/40 shadow-[0_0_12px_rgba(168,85,247,0.25)]'
            : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 border border-transparent'}"
        >
          {st}
        </button>
      {/each}
    </div>

    <div class="relative w-full sm:w-72">
      <Search class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter by filename or preset..."
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

  <!-- Jobs Queue & History -->
  {#if encoder.loading && encoder.jobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-purple-500 dark:border-purple-400 border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-slate-500 dark:text-slate-400">Loading encoding queue...</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10 space-y-4">
      <Film class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">No encoding jobs</h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto font-mono">
        Queue a video transcode to compress large anime releases using HEVC / x265 presets.
      </p>
      <div class="pt-2">
        <button
          onclick={() => (isQueueModalOpen = true)}
          class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-purple-500 to-indigo-500 text-white font-bold rounded-2xl text-xs font-mono shadow-[0_0_15px_rgba(168,85,247,0.3)] cursor-pointer"
        >
          Queue First Transcode
        </button>
      </div>
    </div>
  {:else}
    <div class="space-y-3.5">
      {#each filteredJobs as item (item.id)}
        <div class="glass-card-interactive rounded-3xl p-5 border border-white/10 space-y-3.5">
          <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
            <div class="min-w-0 flex-1 space-y-2">
              <div class="flex items-center gap-2.5 flex-wrap">
                <!-- Status Badge -->
                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold {
                  item.status === 'ENCODING'
                    ? 'bg-purple-500/20 text-purple-300 border border-purple-500/40 shadow-[0_0_10px_rgba(168,85,247,0.3)]'
                    : item.status === 'COMPLETED'
                    ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                    : item.status === 'QUEUED'
                    ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                    : 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
                }">
                  {item.status}
                </span>

                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-slate-700 dark:text-slate-300">
                  {item.preset_name}
                </span>

                <h3 class="text-sm font-bold text-slate-900 dark:text-white truncate max-w-xl font-mono" title={item.source_file_path}>
                  {item.source_filename}
                </h3>
              </div>

              <!-- Metrics Row -->
              <div class="flex items-center gap-4 text-xs font-mono text-slate-500 dark:text-slate-400 flex-wrap">
                <span>
                  Source: <strong class="text-slate-800 dark:text-slate-200">{formatBytes(item.source_size_bytes)}</strong>
                </span>
                {#if item.encoded_size_bytes > 0}
                  <span>
                    Output: <strong class="text-cyan-300">{formatBytes(item.encoded_size_bytes)}</strong>
                  </span>
                  {#if item.status === 'COMPLETED'}
                    {@const saved = calcSavingsPct(item.source_size_bytes, item.encoded_size_bytes)}
                    {#if saved > 0}
                      <span class="text-emerald-400 font-bold px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20">
                        Saved: {saved}%
                      </span>
                    {/if}
                  {/if}
                {/if}

                {#if item.status === 'ENCODING'}
                  <span class="text-purple-300 font-bold">{item.speed.toFixed(2)}x speed</span>
                  <span>{item.fps.toFixed(1)} fps</span>
                  <span>ETA: <strong class="text-slate-800 dark:text-white">{formatEta(item.eta_seconds)}</strong></span>
                {/if}

                {#if item.error_message}
                  <span class="text-rose-400 truncate max-w-sm">Error: {item.error_message}</span>
                {/if}
              </div>
            </div>

            <!-- Controls -->
            <div class="flex items-center gap-2 self-end lg:self-center font-mono">
              {#if item.status === 'QUEUED' || item.status === 'ENCODING'}
                <button
                  onclick={() => handleCancel(item)}
                  class="btn-expressive px-3 py-1.5 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 hover:text-rose-600 dark:hover:text-rose-400 hover:bg-rose-500/20 border-rose-500/30 text-xs cursor-pointer"
                  title="Cancel Job"
                >
                  Cancel
                </button>
              {/if}

              <button
                onclick={() => handleDelete(item)}
                class="btn-expressive p-2 rounded-2xl glass-pill text-slate-500 dark:text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/30 text-xs cursor-pointer"
                title="Delete Record"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="space-y-1.5">
            <div class="flex justify-between items-center text-[11px] font-mono text-slate-500 dark:text-slate-400">
              <span>Transcode Progress</span>
              <span class="font-bold {item.status === 'COMPLETED' ? 'text-emerald-400' : 'text-purple-400'}">
                {item.progress_pct.toFixed(1)}%
              </span>
            </div>
            <div class="w-full h-2.5 rounded-full bg-slate-200 dark:bg-slate-950/80 overflow-hidden border border-slate-200/80 dark:border-white/5 relative">
              <div
                class="h-full rounded-full transition-all duration-300 relative {
                  item.status === 'COMPLETED'
                    ? 'bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.5)]'
                    : item.status === 'FAILED'
                    ? 'bg-rose-500'
                    : item.status === 'QUEUED'
                    ? 'bg-amber-400'
                    : 'bg-gradient-to-r from-purple-500 via-indigo-400 to-cyan-400 shadow-[0_0_12px_rgba(168,85,247,0.4)]'
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
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-lg w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Film class="w-5 h-5 text-purple-600 dark:text-purple-400" />
          <span>Queue Video Transcode</span>
        </h3>
        <button
          onclick={() => (isQueueModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <form onsubmit={handleQueueJob} class="space-y-4 text-xs font-mono">
        <div class="space-y-1.5">
          <label for="source-path" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">SOURCE FILE PATH</label>
          <input
            id="source-path"
            type="text"
            required
            bind:value={newSourcePath}
            placeholder="e.g. /data/downloads/Odekake Kozame - 93 [1080p].mkv"
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        <div class="space-y-1.5">
          <label for="preset-select" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">ENCODING PRESET</label>
          <select
            id="preset-select"
            bind:value={newPresetId}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          >
            {#each encoder.presets as preset}
              <option value={preset.id} class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
                {preset.name} ({preset.video_codec}, CRF {preset.crf}, {preset.target_resolution || 'source res'}) {preset.is_default ? '★ Default' : ''}
              </option>
            {/each}
          </select>
        </div>

        <div class="space-y-1.5">
          <label for="target-path" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">TARGET OUTPUT PATH (OPTIONAL)</label>
          <input
            id="target-path"
            type="text"
            bind:value={newTargetPath}
            placeholder="Auto-generated in /data/temp_encode if left empty"
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        {#if queueError}
          <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-xl text-rose-600 dark:text-rose-300">
            {queueError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isQueueModalOpen = false)}
            class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isQueueing || !newSourcePath.trim()}
            class="btn-expressive px-5 py-2.5 rounded-2xl bg-gradient-to-r from-purple-500 to-indigo-500 text-white font-bold transition-all disabled:opacity-40 cursor-pointer flex items-center gap-2 shadow-[0_0_15px_rgba(168,85,247,0.3)]"
          >
            {#if isQueueing}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Queueing...</span>
            {:else}
              <Zap class="w-4 h-4" />
              <span>Start Transcode</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Presets Management Modal -->
{#if isPresetsModalOpen}
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-2xl w-full shadow-2xl relative space-y-5 border border-slate-200/80 dark:border-white/15"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Sliders class="w-5 h-5 text-purple-600 dark:text-purple-400" />
          <span>Encoding Presets</span>
        </h3>
        <button
          onclick={() => (isPresetsModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Presets List -->
      <div class="space-y-2 max-h-56 overflow-y-auto pr-1">
        {#each encoder.presets as preset}
          <div class="p-3.5 glass-panel rounded-2xl flex items-center justify-between text-xs font-mono border border-slate-200/60 dark:border-white/5">
            <div>
              <div class="flex items-center gap-2">
                <span class="font-bold text-slate-900 dark:text-white">{preset.name}</span>
                {#if preset.is_default}
                  <span class="px-2 py-0.5 rounded-full bg-purple-500/20 text-purple-600 dark:text-purple-300 text-[10px] font-bold border border-purple-500/30">DEFAULT</span>
                {/if}
              </div>
              <div class="text-[11px] text-slate-500 dark:text-slate-400 mt-1">
                {preset.video_codec} • {preset.preset} • CRF {preset.crf} • {preset.target_resolution || 'source'} • Audio: {preset.audio_codec}
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Add New Preset Form -->
      <form onsubmit={handleCreatePreset} class="p-4.5 glass-panel rounded-2xl border border-slate-200/60 dark:border-white/5 space-y-3.5 text-xs font-mono">
        <h4 class="font-bold uppercase text-[11px] text-purple-600 dark:text-purple-300 tracking-wider">Create Custom Preset</h4>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div class="space-y-1">
            <label for="p-name" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">NAME</label>
            <input
              id="p-name"
              type="text"
              required
              bind:value={presetName}
              placeholder="e.g. 720p x265 Anime"
              class="glass-input w-full rounded-xl px-3 py-2"
            />
          </div>
          <div class="space-y-1">
            <label for="p-res" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">RESOLUTION</label>
            <input
              id="p-res"
              type="text"
              bind:value={presetRes}
              placeholder="1280x720 (empty for source)"
              class="glass-input w-full rounded-xl px-3 py-2"
            />
          </div>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <div class="space-y-1">
            <label for="p-codec" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">CODEC</label>
            <select
              id="p-codec"
              bind:value={presetCodec}
              class="glass-input w-full rounded-xl px-2.5 py-2"
            >
              <option value="libx265" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">libx265 (HEVC)</option>
              <option value="libx264" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">libx264 (AVC)</option>
              <option value="libsvtav1" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">libsvtav1 (AV1)</option>
            </select>
          </div>
          <div class="space-y-1">
            <label for="p-crf" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">CRF (18-28)</label>
            <input
              id="p-crf"
              type="number"
              bind:value={presetCrf}
              min="0"
              max="51"
              class="glass-input w-full rounded-xl px-3 py-2"
            />
          </div>
          <div class="space-y-1">
            <label for="p-speed" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">SPEED</label>
            <select
              id="p-speed"
              bind:value={presetSpeed}
              class="glass-input w-full rounded-xl px-2.5 py-2"
            >
              <option value="fast" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">fast</option>
              <option value="medium" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">medium</option>
              <option value="slow" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">slow</option>
            </select>
          </div>
        </div>

        <div class="flex items-center gap-2 pt-1">
          <input
            id="p-default"
            type="checkbox"
            bind:checked={presetIsDefault}
            class="rounded bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700 text-purple-500 w-4 h-4"
          />
          <label for="p-default" class="text-slate-700 dark:text-slate-300 cursor-pointer">Set as default preset</label>
        </div>

        {#if presetError}
          <div class="p-2.5 bg-rose-500/10 text-rose-600 dark:text-rose-300 rounded-xl border border-rose-500/40">
            {presetError}
          </div>
        {/if}

        <div class="flex justify-end pt-2">
          <button
            type="submit"
            disabled={isCreatingPreset || !presetName.trim()}
            class="btn-expressive px-5 py-2 rounded-2xl bg-purple-500 hover:bg-purple-600 text-white font-bold disabled:opacity-40 cursor-pointer shadow-[0_0_15px_rgba(168,85,247,0.3)]"
          >
            Save Preset
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
