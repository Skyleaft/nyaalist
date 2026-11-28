<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '#lib/api/client';
  import { auth } from '#lib/stores/auth.svelte';
  import { library } from '#lib/stores/library.svelte';
  import {
    TARGET_LANGUAGE_OPTIONS,
    translator
  } from '#lib/stores/translator.svelte';
  import type { SubtitleTrack, TranslationJobItem } from '#lib/api/types';
  import {
    AlertCircle,
    ArrowRight,
    Check,
    CheckCircle2,
    Clock,
    Cpu,
    Download,
    Film,
    KeyRound,
    Languages,
    Plus,
    RefreshCw,
    Search,
    Sparkles,
    Trash2,
    X,
    Zap
  } from 'lucide-svelte';

  let filterStatus = $state<string>('ALL');
  let searchQuery = $state('');

  // Queue Translation Modal state
  let isModalOpen = $state(false);
  let selectedLibraryFileId = $state<string>('');
  let sourceFilePath = $state('');
  let targetFilePath = $state('');
  let targetLangPreset = $state<string>('Indonesian');
  let customTargetLang = $state('');
  let targetLangCode = $state('ind');
  let embedMode = $state<'mux_keep_original' | 'mux_replace'>('mux_keep_original');

  // Subtitle track probing
  let probing = $state(false);
  let probeError = $state<string | null>(null);
  let probeWarnings = $state<string[]>([]);
  let probedTracks = $state<SubtitleTrack[]>([]);
  let selectedStreamIndex = $state<number | null>(null);

  let isQueueing = $state(false);
  let queueError = $state<string | null>(null);

  onMount(async () => {
    await Promise.all([translator.init(), library.fetchFiles()]);
  });

  const activeStatuses = new Set(['EXTRACTING', 'TRANSLATING', 'EMBEDDING']);

  const filteredJobs = $derived(
    translator.jobs.filter((job) => {
      if (filterStatus === 'RUNNING') {
        if (!activeStatuses.has(job.status)) return false;
      } else if (filterStatus !== 'ALL' && job.status !== filterStatus) {
        return false;
      }
      if (searchQuery.trim().length > 0) {
        const q = searchQuery.toLowerCase();
        return (
          job.source_filename.toLowerCase().includes(q) ||
          job.target_language.toLowerCase().includes(q) ||
          job.gemini_model.toLowerCase().includes(q) ||
          job.source_file_path.toLowerCase().includes(q)
        );
      }
      return true;
    })
  );

  const effectiveTargetLanguage = $derived(
    targetLangPreset === 'custom' ? customTargetLang.trim() : targetLangPreset
  );

  function handleTargetLangChange() {
    const found = TARGET_LANGUAGE_OPTIONS.find(o => o.label === targetLangPreset);
    if (found) {
      targetLangCode = found.code;
      updateSuggestedTargetPath();
    }
  }

  function handleLibrarySelect() {
    if (!selectedLibraryFileId) return;
    const item = library.items.find(f => f.id === selectedLibraryFileId);
    if (item) {
      sourceFilePath = item.current_path;
      updateSuggestedTargetPath();
      probeSubtitles();
    }
  }

  function updateSuggestedTargetPath() {
    if (!sourceFilePath.trim()) {
      targetFilePath = '';
      return;
    }
    const src = sourceFilePath.trim();
    const lastDot = src.lastIndexOf('.');
    const code = targetLangCode.trim() || 'sub';
    if (lastDot > 0) {
      targetFilePath = `${src.substring(0, lastDot)} [${code}]${src.substring(lastDot)}`;
    } else {
      targetFilePath = `${src} [${code}].mkv`;
    }
  }

  function openNewTranslationModal() {
    queueError = null;
    probeError = null;
    probeWarnings = [];
    probedTracks = [];
    selectedStreamIndex = null;
    isModalOpen = true;
  }

  async function probeSubtitles() {
    const path = sourceFilePath.trim();
    if (!path) return;
    probing = true;
    probeError = null;
    probeWarnings = [];
    probedTracks = [];
    selectedStreamIndex = null;

    try {
      const res = await api.translator.probeSubtitles(path);
      probedTracks = res.tracks;
      probeWarnings = res.warnings;
      selectedStreamIndex = res.recommended_stream_index ?? null;
    } catch (err: any) {
      probeError = err.message || 'Failed to inspect video subtitle streams';
    } finally {
      probing = false;
    }
  }

  async function handleQueueJob(e: Event) {
    e.preventDefault();
    if (!sourceFilePath.trim() || !effectiveTargetLanguage) return;

    isQueueing = true;
    queueError = null;
    try {
      await translator.queueJob({
        library_file_id: selectedLibraryFileId || undefined,
        source_file_path: sourceFilePath.trim(),
        target_file_path: targetFilePath.trim() || undefined,
        source_stream_index: selectedStreamIndex ?? undefined,
        target_language: effectiveTargetLanguage,
        target_language_code: targetLangCode.trim() || undefined,
        embed_mode: embedMode
      });
      isModalOpen = false;
      selectedLibraryFileId = '';
      sourceFilePath = '';
      targetFilePath = '';
      probedTracks = [];
      selectedStreamIndex = null;
    } catch (err: any) {
      queueError = err.message || 'Failed to queue translation job';
    } finally {
      isQueueing = false;
    }
  }

  async function handleCancel(job: TranslationJobItem) {
    try {
      await translator.cancelJob(job.id);
    } catch (err: any) {
      alert(`Cancel failed: ${err.message}`);
    }
  }

  async function handleDelete(job: TranslationJobItem) {
    if (!confirm(`Delete translation record for ${job.source_filename}?`)) return;
    try {
      await translator.deleteJob(job.id);
    } catch (err: any) {
      alert(`Delete failed: ${err.message}`);
    }
  }

  function statusBadgeClass(status: string): string {
    switch (status) {
      case 'EXTRACTING':
        return 'bg-sky-500/20 text-sky-600 dark:text-sky-300 border border-sky-500/40';
      case 'TRANSLATING':
        return 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 shadow-[0_0_10px_rgba(0,229,255,0.25)]';
      case 'EMBEDDING':
        return 'bg-purple-500/20 text-purple-600 dark:text-purple-300 border border-purple-500/40';
      case 'COMPLETED':
        return 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-500/40';
      case 'QUEUED':
        return 'bg-amber-500/20 text-amber-600 dark:text-amber-300 border border-amber-500/40';
      default:
        return 'bg-rose-500/20 text-rose-600 dark:text-rose-300 border border-rose-500/40';
    }
  }
</script>

<svelte:head>
  <title>AI Subtitle Translator | Nyaalist</title>
</svelte:head>

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-6">
  <!-- Header -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-slate-200/80 dark:border-white/10">
    <div class="flex items-center gap-4">
      <div class="p-3.5 rounded-2xl bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-500/25 shadow-[0_0_20px_rgba(0,229,255,0.2)]">
        <Languages class="w-7 h-7" />
      </div>
      <div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
          <span>AI Subtitle Translator</span>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-600 dark:text-cyan-300 border border-cyan-400/30 uppercase">
            Google AI Studio
          </span>
        </h1>
        <p class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-0.5">
          Extract embedded subtitles (ASS/SRT), translate via Gemini, and re-mux back into video losslessly
        </p>
      </div>
    </div>

    <div class="flex items-center gap-3 flex-wrap">
      <!-- WS Live Pill -->
      <div class="glass-pill px-3.5 py-2 rounded-2xl flex items-center gap-2 text-xs font-mono">
        <div class="relative flex items-center justify-center">
          <div class="w-2.5 h-2.5 rounded-full {translator.wsConnected ? 'bg-emerald-400' : 'bg-rose-500'}"></div>
          {#if translator.wsConnected}
            <div class="absolute w-4 h-4 rounded-full bg-emerald-400/30 animate-ping"></div>
          {/if}
        </div>
        <span class="font-medium {translator.wsConnected ? 'text-emerald-600 dark:text-emerald-300' : 'text-rose-500 dark:text-rose-400'}">
          {translator.wsConnected ? 'WS LIVE' : 'OFFLINE'}
        </span>
      </div>

      <!-- API Key & Model Status Pill -->
      {#if auth.user?.role === 'admin'}
        <a
          href="/settings"
          class="btn-expressive px-4 py-2 rounded-2xl glass-pill font-mono text-xs cursor-pointer flex items-center gap-2 {translator.status?.gemini_configured
            ? 'text-emerald-600 dark:text-emerald-300 border-emerald-500/30'
            : 'text-amber-600 dark:text-amber-300 border-amber-500/30'}"
          title="Configure Google AI Studio API key & model in System Settings"
        >
          <KeyRound class="w-3.5 h-3.5" />
          <span>
            {translator.status?.gemini_configured
              ? `Gemini (${translator.status.default_model})`
              : 'Configure in Settings'}
          </span>
        </a>
      {:else}
        <div class="glass-pill px-3.5 py-2 rounded-2xl font-mono text-xs flex items-center gap-2">
          <KeyRound class="w-3.5 h-3.5 {translator.status?.gemini_configured ? 'text-emerald-400' : 'text-amber-400'}" />
          <span>
            {translator.status?.gemini_configured
              ? `Gemini (${translator.status.default_model})`
              : 'Key Missing'}
          </span>
        </div>
      {/if}

      <button
        onclick={openNewTranslationModal}
        class="btn-expressive px-4 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-500 to-indigo-500 hover:from-cyan-400 hover:to-indigo-400 text-white font-bold text-xs font-mono shadow-[0_0_20px_rgba(0,229,255,0.28)] flex items-center gap-2 cursor-pointer"
      >
        <Plus class="w-4 h-4 stroke-[2.5]" />
        <span>Translate Video</span>
      </button>
    </div>
  </div>

  <!-- Missing API Key Banner -->
  {#if translator.status && !translator.status.gemini_configured}
    <div class="glass-panel rounded-2xl p-4 border-l-4 border-amber-500 bg-amber-500/10 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs font-mono">
      <div class="flex items-center gap-2.5 text-amber-700 dark:text-amber-200">
        <AlertCircle class="w-4 h-4 shrink-0 text-amber-500" />
        <span>
          Google AI Studio API key is not configured yet. Set your key and model in System Settings to start translating subtitles.
        </span>
      </div>
      {#if auth.user?.role === 'admin'}
        <a
          href="/settings"
          class="btn-expressive px-3.5 py-1.5 rounded-xl bg-amber-500 text-slate-950 font-bold cursor-pointer whitespace-nowrap"
        >
          Open Settings &rarr;
        </a>
      {/if}
    </div>
  {/if}

  <!-- Active Translation Hero Telemetry -->
  {#if translator.activeJob || (translator.activeTelemetry && activeStatuses.has(translator.activeTelemetry.status))}
    {@const job = translator.activeJob}
    {@const tel = translator.activeTelemetry}
    {@const curStatus = tel?.status || job?.status || 'TRANSLATING'}
    {@const pct = tel?.progress_pct ?? job?.progress_pct ?? 0}
    <div class="glass-panel-elevated rounded-3xl p-6 shadow-2xl border border-cyan-500/30 space-y-4">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
        <div>
          <div class="flex items-center gap-2.5 flex-wrap">
            <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold animate-pulse {statusBadgeClass(curStatus)}">
              {curStatus} LIVE
            </span>
            <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-slate-700 dark:text-slate-300">
              {tel?.gemini_model || job?.gemini_model}
            </span>
            <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-cyan-500/10 border border-cyan-400/25 text-cyan-600 dark:text-cyan-300 font-bold">
              &rarr; {tel?.target_language || job?.target_language}
            </span>
          </div>
          <h2 class="text-base sm:text-lg font-bold text-slate-900 dark:text-white mt-2 truncate max-w-2xl font-mono">
            {tel?.file_name || job?.source_filename}
          </h2>
          <p class="text-xs font-mono text-slate-500 dark:text-slate-400 mt-0.5">
            {tel?.stage_message || job?.stage_message || 'Processing...'}
          </p>
        </div>

        {#if job}
          <button
            onclick={() => handleCancel(job)}
            class="btn-expressive self-start md:self-center px-4 py-1.5 rounded-2xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-600 dark:text-rose-300 border border-rose-500/40 text-xs font-mono cursor-pointer"
          >
            Cancel Job
          </button>
        {/if}
      </div>

      <!-- Pipeline Stage Stepper -->
      <div class="grid grid-cols-3 gap-2 text-[11px] font-mono">
        <div class="p-2 rounded-xl border text-center {curStatus === 'EXTRACTING' ? 'border-sky-400/60 bg-sky-500/15 text-sky-600 dark:text-sky-300 font-bold' : 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300'}">
          1. Extract Sub
        </div>
        <div class="p-2 rounded-xl border text-center {curStatus === 'TRANSLATING' ? 'border-cyan-400/60 bg-cyan-500/15 text-cyan-600 dark:text-cyan-300 font-bold' : curStatus === 'EMBEDDING' ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300' : 'border-slate-200/60 dark:border-white/10 text-slate-400'}">
          2. Gemini Translate
        </div>
        <div class="p-2 rounded-xl border text-center {curStatus === 'EMBEDDING' ? 'border-purple-400/60 bg-purple-500/15 text-purple-600 dark:text-purple-300 font-bold' : 'border-slate-200/60 dark:border-white/10 text-slate-400'}">
          3. Embed in Video
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="space-y-1.5">
        <div class="flex justify-between text-xs font-mono">
          <span class="text-slate-500 dark:text-slate-400">
            Lines: <strong class="text-slate-800 dark:text-white">{tel?.translated_cues ?? job?.translated_cues ?? 0} / {tel?.total_cues ?? job?.total_cues ?? 0}</strong>
            {#if (tel?.total_batches ?? job?.total_batches ?? 0) > 0}
              <span class="ml-2">(Batch {tel?.current_batch ?? job?.current_batch ?? 0}/{tel?.total_batches ?? job?.total_batches ?? 0})</span>
            {/if}
          </span>
          <span class="text-cyan-600 dark:text-cyan-400 font-bold">
            {pct.toFixed(1)}%
          </span>
        </div>
        <div class="w-full h-3 rounded-full bg-slate-200 dark:bg-slate-950/80 overflow-hidden border border-slate-200 dark:border-white/5">
          <div
            class="h-full rounded-full bg-gradient-to-r from-sky-500 via-cyan-400 to-emerald-400 transition-all duration-300 shadow-[0_0_15px_rgba(0,229,255,0.45)]"
            style="width: {Math.min(100, Math.max(0, pct))}%"
          ></div>
        </div>
      </div>
    </div>
  {/if}

  <!-- KPI Cards -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">ACTIVE TRANSLATOR</span>
        <div class="p-2 rounded-xl bg-cyan-500/10 text-cyan-600 dark:text-cyan-300 border border-cyan-500/20">
          <Cpu class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">
        {translator.activeJob ? 1 : 0}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">QUEUE DEPTH</span>
        <div class="p-2 rounded-xl bg-amber-500/10 text-amber-500 dark:text-amber-300 border border-amber-500/20">
          <Clock class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-amber-500 dark:text-amber-300">
        {translator.queuedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">COMPLETED</span>
        <div class="p-2 rounded-xl bg-emerald-500/10 text-emerald-500 dark:text-emerald-300 border border-emerald-500/20">
          <CheckCircle2 class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-emerald-600 dark:text-emerald-400">
        {translator.completedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">LINES TRANSLATED</span>
        <div class="p-2 rounded-xl bg-purple-500/10 text-purple-500 dark:text-purple-300 border border-purple-500/20">
          <Sparkles class="w-4 h-4" />
        </div>
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-purple-600 dark:text-purple-300">
        {translator.totalLinesTranslated.toLocaleString()}
      </div>
    </div>
  </div>

  <!-- Filter & Search -->
  <div class="glass-panel p-3.5 rounded-3xl border border-slate-200/80 dark:border-white/10 flex flex-col sm:flex-row justify-between items-center gap-3">
    <div class="flex items-center gap-1.5 w-full sm:w-auto overflow-x-auto scrollbar-none pb-1 sm:pb-0">
      {#each ['ALL', 'QUEUED', 'RUNNING', 'COMPLETED', 'FAILED'] as const as st}
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

    <div class="relative w-full sm:w-72">
      <Search class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter by file, language, model..."
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

  <!-- Jobs List -->
  {#if translator.loading && translator.jobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-cyan-500 dark:border-cyan-400 border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-slate-500 dark:text-slate-400">Loading translation jobs...</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10 space-y-4">
      <Languages class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">No translation jobs yet</h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-md mx-auto font-mono">
        Extract an existing subtitle track (e.g. English ASS/SRT), translate it with Google AI Studio Gemini, and embed it right back into your video.
      </p>
      <div class="pt-2">
        <button
          onclick={openNewTranslationModal}
          class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-indigo-500 text-white font-bold rounded-2xl text-xs font-mono shadow-[0_0_15px_rgba(0,229,255,0.28)] cursor-pointer"
        >
          Translate First Video
        </button>
      </div>
    </div>
  {:else}
    <div class="space-y-3.5">
      {#each filteredJobs as item (item.id)}
        <div class="glass-card-interactive rounded-3xl p-5 border border-slate-200/80 dark:border-white/10 space-y-3.5">
          <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
            <div class="min-w-0 flex-1 space-y-2">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold {statusBadgeClass(item.status)}">
                  {item.status}
                </span>

                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-cyan-500/10 border border-cyan-400/25 text-cyan-600 dark:text-cyan-300 flex items-center gap-1 font-bold">
                  <span>{(item.source_language || 'auto').toUpperCase()}</span>
                  <ArrowRight class="w-3 h-3" />
                  <span>{item.target_language} ({item.target_language_code})</span>
                </span>

                <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-slate-700 dark:text-slate-300">
                  {item.gemini_model}
                </span>

                {#if item.source_sub_codec}
                  <span class="px-2 py-0.5 rounded-full text-[10px] font-mono bg-purple-500/10 border border-purple-500/25 text-purple-600 dark:text-purple-300 uppercase">
                    {item.source_sub_codec}
                  </span>
                {/if}
              </div>

              <h3 class="text-sm font-bold text-slate-900 dark:text-white truncate max-w-2xl font-mono" title={item.source_file_path}>
                {item.source_filename}
              </h3>

              <div class="flex items-center gap-3 text-[11px] font-mono text-slate-500 dark:text-slate-400 flex-wrap">
                <span class="truncate max-w-md" title={item.target_file_path}>
                  Output: <strong class="text-slate-800 dark:text-slate-200">{item.target_filename}</strong>
                </span>
                {#if item.total_cues > 0}
                  <span>•</span>
                  <span>
                    Lines: <strong class="text-cyan-600 dark:text-cyan-300">{item.translated_cues}/{item.total_cues}</strong>
                  </span>
                {/if}
                {#if item.stage_message}
                  <span>•</span>
                  <span class="truncate max-w-md">{item.stage_message}</span>
                {/if}
              </div>

              {#if item.error_message}
                <div class="text-xs font-mono text-rose-500 dark:text-rose-400">
                  Error: {item.error_message}
                </div>
              {/if}
            </div>

            <!-- Controls -->
            <div class="flex items-center gap-2 self-end lg:self-center font-mono">
              {#if item.has_translated_sub}
                <a
                  href={api.translator.getSubtitleDownloadUrl(item.id)}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="btn-expressive px-3 py-1.5 rounded-2xl glass-pill text-cyan-600 dark:text-cyan-300 hover:bg-cyan-500/15 border-cyan-400/30 text-xs flex items-center gap-1.5"
                  title="Download translated subtitle file"
                >
                  <Download class="w-3.5 h-3.5" />
                  <span>Sub</span>
                </a>
              {/if}

              {#if item.status === 'QUEUED' || activeStatuses.has(item.status)}
                <button
                  onclick={() => handleCancel(item)}
                  class="btn-expressive px-3 py-1.5 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 hover:text-rose-500 hover:bg-rose-500/20 border-rose-500/30 text-xs cursor-pointer"
                >
                  Cancel
                </button>
              {/if}

              <button
                onclick={() => handleDelete(item)}
                class="btn-expressive p-2 rounded-2xl glass-pill text-slate-500 dark:text-slate-400 hover:text-rose-500 hover:bg-rose-500/20 hover:border-rose-500/30 text-xs cursor-pointer"
                title="Delete Job"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="space-y-1.5">
            <div class="flex justify-between items-center text-[11px] font-mono text-slate-500 dark:text-slate-400">
              <span>Pipeline Progress</span>
              <span class="font-bold {item.status === 'COMPLETED' ? 'text-emerald-500 dark:text-emerald-400' : 'text-cyan-600 dark:text-cyan-400'}">
                {item.progress_pct.toFixed(1)}%
              </span>
            </div>
            <div class="w-full h-2 rounded-full bg-slate-200 dark:bg-slate-950/80 overflow-hidden border border-slate-200/80 dark:border-white/5">
              <div
                class="h-full rounded-full transition-all duration-300 {item.status === 'COMPLETED'
                  ? 'bg-emerald-400'
                  : item.status === 'FAILED'
                  ? 'bg-rose-500'
                  : item.status === 'QUEUED'
                  ? 'bg-amber-400'
                  : 'bg-gradient-to-r from-sky-500 via-cyan-400 to-emerald-400'}"
                style="width: {Math.min(100, Math.max(0, item.progress_pct))}%"
              ></div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- New AI Translation Modal -->
{#if isModalOpen}
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl overflow-y-auto"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-2xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15 my-8"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Languages class="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
          <span>New AI Subtitle Translation</span>
        </h3>
        <button
          onclick={() => (isModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <form onsubmit={handleQueueJob} class="space-y-4 text-xs font-mono">
        <!-- Library File Picker -->
        {#if library.items.length > 0}
          <div class="space-y-1.5">
            <label for="tr-library-select" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
              PICK FROM LIBRARY (OPTIONAL)
            </label>
            <select
              id="tr-library-select"
              bind:value={selectedLibraryFileId}
              onchange={handleLibrarySelect}
              class="glass-input w-full rounded-2xl px-3.5 py-2.5"
            >
              <option value="" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">-- Or enter a custom file path below --</option>
              {#each library.items as item (item.id)}
                <option value={item.id} class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
                  {item.anime_title ? `${item.anime_title} - EP ${item.episode || '?'}` : item.original_filename}
                </option>
              {/each}
            </select>
          </div>
        {/if}

        <!-- Source Video Path + Inspect Subtitles -->
        <div class="space-y-1.5">
          <label for="tr-source-path" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
            SOURCE VIDEO FILE PATH
          </label>
          <div class="flex gap-2">
            <input
              id="tr-source-path"
              type="text"
              required
              bind:value={sourceFilePath}
              oninput={() => updateSuggestedTargetPath()}
              placeholder="e.g. ./data/downloads/Anime - 01 [1080p].mkv"
              class="glass-input flex-1 rounded-2xl px-3.5 py-2.5"
            />
            <button
              type="button"
              onclick={probeSubtitles}
              disabled={probing || !sourceFilePath.trim()}
              class="btn-expressive glass-pill px-4 py-2 rounded-2xl text-cyan-600 dark:text-cyan-300 border-cyan-400/30 font-bold cursor-pointer disabled:opacity-40 flex items-center gap-1.5 whitespace-nowrap"
            >
              <RefreshCw class="w-3.5 h-3.5 {probing ? 'animate-spin' : ''}" />
              <span>{probing ? 'Probing...' : 'Inspect Subs'}</span>
            </button>
          </div>
        </div>

        <!-- Probed Subtitle Tracks -->
        {#if probeError}
          <div class="p-3 rounded-2xl bg-rose-500/10 border border-rose-500/40 text-rose-600 dark:text-rose-300">
            {probeError}
          </div>
        {/if}

        {#if probeWarnings.length > 0}
          <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/40 text-amber-700 dark:text-amber-300 space-y-1">
            {#each probeWarnings as warn}
              <div>⚠ {warn}</div>
            {/each}
          </div>
        {/if}

        {#if probedTracks.length > 0}
          <div class="space-y-1.5">
            <span class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
              SOURCE SUBTITLE TRACK ({probedTracks.length} FOUND)
            </span>
            <div class="space-y-1.5 max-h-40 overflow-y-auto pr-1">
              {#each probedTracks as track (track.stream_index)}
                <button
                  type="button"
                  disabled={!track.is_text_based}
                  onclick={() => (selectedStreamIndex = track.stream_index)}
                  class="w-full p-2.5 rounded-2xl border text-left transition-all flex items-center justify-between gap-2 {selectedStreamIndex === track.stream_index
                    ? 'border-cyan-400/60 bg-cyan-500/15 text-slate-900 dark:text-white'
                    : track.is_text_based
                    ? 'border-slate-200/80 dark:border-white/10 glass-panel hover:border-cyan-400/30 cursor-pointer'
                    : 'border-slate-200/40 dark:border-white/5 opacity-45 cursor-not-allowed'}"
                >
                  <div class="flex items-center gap-2 min-w-0">
                    <span class="px-2 py-0.5 rounded-full text-[10px] bg-black/10 dark:bg-white/10 font-bold">
                      #{track.stream_index}
                    </span>
                    <span class="px-2 py-0.5 rounded-full text-[10px] bg-purple-500/15 text-purple-600 dark:text-purple-300 uppercase font-bold">
                      {track.codec_name}
                    </span>
                    <span class="font-bold text-cyan-600 dark:text-cyan-300 uppercase">
                      {track.language || 'und'}
                    </span>
                    <span class="truncate text-slate-600 dark:text-slate-300">
                      {track.title || 'Untitled Track'}
                    </span>
                  </div>
                  <div class="flex items-center gap-1.5 shrink-0">
                    {#if !track.is_text_based}
                      <span class="text-[10px] text-amber-500">Bitmap (Unsupported)</span>
                    {:else if track.is_default}
                      <span class="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-500">Default</span>
                    {/if}
                  </div>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Target Language & Code -->
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <div class="sm:col-span-2 space-y-1.5">
            <label for="tr-target-lang" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
              TARGET LANGUAGE
            </label>
            <select
              id="tr-target-lang"
              bind:value={targetLangPreset}
              onchange={handleTargetLangChange}
              class="glass-input w-full rounded-2xl px-3.5 py-2.5"
            >
              {#each TARGET_LANGUAGE_OPTIONS as opt}
                <option value={opt.label} class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
                  {opt.label} ({opt.code})
                </option>
              {/each}
              <option value="custom" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Custom Language...</option>
            </select>
            {#if targetLangPreset === 'custom'}
              <input
                type="text"
                required
                bind:value={customTargetLang}
                placeholder="e.g. Sundanese, Dutch, Turkish..."
                class="glass-input w-full rounded-2xl px-3.5 py-2 mt-1.5"
              />
            {/if}
          </div>

          <div class="space-y-1.5">
            <label for="tr-lang-code" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
              ISO 639-2 CODE
            </label>
            <input
              id="tr-lang-code"
              type="text"
              bind:value={targetLangCode}
              oninput={() => updateSuggestedTargetPath()}
              placeholder="ind"
              maxlength="8"
              class="glass-input w-full rounded-2xl px-3.5 py-2.5 lowercase"
            />
          </div>
        </div>

        <!-- Embed Mode & Active Model Info -->
        <div class="space-y-1.5">
          <label for="tr-embed-mode" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
            VIDEO EMBED MODE
          </label>
          <select
            id="tr-embed-mode"
            bind:value={embedMode}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          >
            <option value="mux_keep_original" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
              Keep Original + Add Translated (Default)
            </option>
            <option value="mux_replace" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
              Replace Existing Subtitle Tracks
            </option>
          </select>
          {#if translator.status?.default_model}
            <div class="flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400 pt-1">
              <span>Active Model: <strong class="text-cyan-600 dark:text-cyan-300">{translator.status.default_model}</strong></span>
              {#if auth.user?.role === 'admin'}
                <a href="/settings" class="underline hover:text-cyan-500">Change in Settings &rarr;</a>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Target Output Video Path -->
        <div class="space-y-1.5">
          <label for="tr-target-path" class="block text-slate-600 dark:text-slate-400 font-bold uppercase text-[10px]">
            OUTPUT VIDEO PATH (OPTIONAL)
          </label>
          <input
            id="tr-target-path"
            type="text"
            bind:value={targetFilePath}
            placeholder="Auto-generated next to source file if left empty"
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        {#if queueError}
          <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-2xl text-rose-600 dark:text-rose-300">
            {queueError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isModalOpen = false)}
            class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isQueueing || !sourceFilePath.trim() || !effectiveTargetLanguage}
            class="btn-expressive px-5 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-500 to-indigo-500 text-white font-bold transition-all disabled:opacity-40 cursor-pointer flex items-center gap-2 shadow-[0_0_15px_rgba(0,229,255,0.28)]"
          >
            {#if isQueueing}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Queueing...</span>
            {:else}
              <Sparkles class="w-4 h-4" />
              <span>Start AI Translation</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
