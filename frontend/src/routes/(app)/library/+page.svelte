<script lang="ts">
  import { onMount } from 'svelte';
  import { library } from '#lib/stores/library.svelte';
  import { api } from '#lib/api/client';
  import type { LibraryFileItem, EncodingPreset } from '#lib/api/types';
  import {
    Library,
    RefreshCw,
    Zap,
    FolderKanban,
    HardDrive,
    CheckCircle2,
    Clock,
    Film,
    Search,
    X,
    Move,
    Trash2,
    Code2,
    Sparkles,
    Check
  } from 'lucide-svelte';

  let filterStatus = $state<'ALL' | 'PENDING' | 'ORGANIZED'>('ALL');
  let searchQuery = $state('');

  // Custom move modal state
  let isMoveModalOpen = $state(false);
  let selectedFile = $state<LibraryFileItem | null>(null);
  let customTargetDir = $state('');
  let customFilename = $state('');
  let moveError = $state<string | null>(null);
  let isMoving = $state(false);

  // Encode modal state
  let isEncodeModalOpen = $state(false);
  let encodeTargetFile = $state<LibraryFileItem | null>(null);
  let presets = $state<EncodingPreset[]>([]);
  let selectedPresetId = $state<string>('');
  let customTargetFilePath = $state('');
  let isEncoding = $state(false);
  let encodeError = $state<string | null>(null);
  let encodeSuccess = $state<string | null>(null);

  const selectedPreset = $derived(
    presets.find(p => p.id === selectedPresetId)
  );

  // Template editor modal
  let customTemplate = $state('{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}');

  onMount(() => {
    library.init();
    loadPresets();
  });

  async function loadPresets() {
    try {
      presets = await api.encoder.getPresets();
      if (!selectedPresetId) {
        const defaultPreset = presets.find(p => p.is_default) || presets[0];
        if (defaultPreset) {
          selectedPresetId = defaultPreset.id;
        }
      }
    } catch (e) {
      console.warn('Failed to load presets for library:', e);
    }
  }

  const filteredItems = $derived(
    library.items.filter((item: any) => {
      if (filterStatus === 'PENDING' && item.is_moved) return false;
      if (filterStatus === 'ORGANIZED' && !item.is_moved) return false;
      if (searchQuery.trim().length > 0) {
        const q = searchQuery.toLowerCase();
        return (
          item.original_filename.toLowerCase().includes(q) ||
          (item.anime_title && item.anime_title.toLowerCase().includes(q)) ||
          item.current_path.toLowerCase().includes(q)
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

  function getCoverUrl(path: string): string {
    if (path.startsWith('/') || path.startsWith('http://') || path.startsWith('https://')) {
      return path;
    }
    return `/api/covers/${path}`;
  }

  function openMoveModal(item: LibraryFileItem) {
    selectedFile = item;
    const targetPath = item.target_library_path || item.current_path;
    const lastSlash = Math.max(targetPath.lastIndexOf('/'), targetPath.lastIndexOf('\\'));
    if (lastSlash > 0) {
      customTargetDir = targetPath.substring(0, lastSlash);
      customFilename = targetPath.substring(lastSlash + 1);
    } else {
      customTargetDir = '';
      customFilename = item.original_filename;
    }
    moveError = null;
    isMoveModalOpen = true;
  }

  async function handleCustomMove(e: Event) {
    e.preventDefault();
    if (!selectedFile) return;

    isMoving = true;
    moveError = null;
    try {
      await library.moveFile({
        file_id: selectedFile.id,
        source_path: selectedFile.current_path,
        target_path: customTargetDir.trim(),
        new_filename: customFilename.trim() || undefined
      });
      isMoveModalOpen = false;
      selectedFile = null;
    } catch (err: any) {
      moveError = err.message || 'Move operation failed';
    } finally {
      isMoving = false;
    }
  }

  async function handleAutoOrganizeSingle(item: LibraryFileItem) {
    try {
      await library.autoOrganize(item.id, customTemplate);
    } catch (err: any) {
      alert(`Auto-organize failed: ${err.message}`);
    }
  }

  async function handleAutoOrganizeAll() {
    if (library.pendingCount === 0) return;
    if (!confirm(`Auto-organize ${library.pendingCount} pending files using current naming template?`)) {
      return;
    }
    try {
      await library.autoOrganize(undefined, customTemplate);
    } catch (err: any) {
      alert(`Auto-organize batch failed: ${err.message}`);
    }
  }

  async function handleScan() {
    try {
      await library.scan();
    } catch (err: any) {
      alert(`Scan failed: ${err.message}`);
    }
  }

  async function handleDelete(item: LibraryFileItem) {
    if (!confirm(`Remove tracking for ${item.original_filename}? The file on disk will NOT be deleted.`)) {
      return;
    }
    try {
      await library.deleteFile(item.id);
    } catch (err: any) {
      alert(`Delete failed: ${err.message}`);
    }
  }

  function openEncodeModal(item: LibraryFileItem) {
    encodeTargetFile = item;
    encodeError = null;
    encodeSuccess = null;

    const source = item.current_path;
    const lastDot = source.lastIndexOf('.');
    if (lastDot > 0) {
      customTargetFilePath = `${source.substring(0, lastDot)}_encoded.mkv`;
    } else {
      customTargetFilePath = `${source}_encoded.mkv`;
    }

    if (!selectedPresetId && presets.length > 0) {
      const defaultPreset = presets.find(p => p.is_default) || presets[0];
      if (defaultPreset) {
        selectedPresetId = defaultPreset.id;
      }
    }

    isEncodeModalOpen = true;
  }

  async function handleQueueEncode(e: Event) {
    e.preventDefault();
    if (!encodeTargetFile) return;

    isEncoding = true;
    encodeError = null;
    encodeSuccess = null;

    try {
      await library.encodeFile(
        encodeTargetFile.id,
        selectedPresetId || undefined,
        customTargetFilePath.trim() || undefined
      );
      encodeSuccess = `Encoding job queued for "${encodeTargetFile.original_filename}"!`;
      setTimeout(() => {
        isEncodeModalOpen = false;
        encodeTargetFile = null;
        encodeSuccess = null;
      }, 1500);
    } catch (err: any) {
      encodeError = err.message || 'Failed to queue encode job';
    } finally {
      isEncoding = false;
    }
  }
</script>

<svelte:head>
  <title>Library Management | Nyaalist</title>
</svelte:head>

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-6">
  <!-- Header & Actions Deck -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-white/10">
    <div class="flex items-center gap-4">
      <div class="p-3.5 rounded-2xl bg-sky-500/10 text-sky-300 border border-sky-500/25 shadow-[0_0_20px_rgba(56,189,248,0.2)]">
        <FolderKanban class="w-7 h-7" />
      </div>
      <div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white">
          Anime Library Management
        </h1>
        <p class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-0.5">
          Scan, match with AniList metadata, and organize media into structured Plex/Jellyfin directories
        </p>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <!-- Scan Button -->
      <button
        onclick={handleScan}
        disabled={library.scanning}
        class="btn-expressive px-4 py-2.5 rounded-2xl glass-pill text-slate-700 dark:text-white font-mono text-xs transition-all disabled:opacity-50 cursor-pointer flex items-center gap-2"
      >
        <RefreshCw class="w-3.5 h-3.5 {library.scanning ? 'animate-spin text-cyan-600 dark:text-cyan-400' : ''}" />
        <span>{library.scanning ? 'Scanning...' : 'Scan Downloads'}</span>
      </button>

      <!-- Auto-Organize All -->
      <button
        onclick={handleAutoOrganizeAll}
        disabled={library.organizing || library.pendingCount === 0}
        class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white font-bold text-xs font-mono rounded-2xl transition-all shadow-[0_0_20px_rgba(0,229,255,0.25)] disabled:opacity-40 cursor-pointer flex items-center gap-2"
      >
        {#if library.organizing}
          <span class="inline-block animate-spin w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full"></span>
          <span>Organizing...</span>
        {:else}
          <Zap class="w-4 h-4 stroke-[2.5]" />
          <span>Auto-Organize All ({library.pendingCount})</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Telemetry KPI Banners (5 M3 Cards) -->
  <div class="grid grid-cols-2 lg:grid-cols-5 gap-3.5">
    <div class="glass-card-interactive rounded-3xl p-5">
      <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">TOTAL FILES</span>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">
        {library.totalCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">ORGANIZED</span>
        <CheckCircle2 class="w-4 h-4 text-emerald-500 dark:text-emerald-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-emerald-600 dark:text-emerald-400">
        {library.movedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">PENDING MOVE</span>
        <Clock class="w-4 h-4 text-amber-500 dark:text-amber-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-amber-600 dark:text-amber-300">
        {library.pendingCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <div class="flex justify-between items-start">
        <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">ENCODED</span>
        <Film class="w-4 h-4 text-purple-500 dark:text-purple-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-purple-600 dark:text-purple-300">
        {library.encodedCount}
      </div>
    </div>

    <div class="glass-card-interactive rounded-3xl p-5">
      <span class="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase text-[11px]">TRACKED STORAGE</span>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-cyan-600 dark:text-cyan-300">
        {formatBytes(library.totalSizeBytes)}
      </div>
    </div>
  </div>

  <!-- Template Info Bar -->
  <div class="glass-panel p-3.5 rounded-2xl border border-slate-200/80 dark:border-white/10 flex flex-col md:flex-row md:items-center justify-between gap-3 text-xs font-mono">
    <div class="flex items-center gap-2 overflow-x-auto">
      <Code2 class="w-4 h-4 text-cyan-600 dark:text-cyan-400 shrink-0" />
      <span class="text-slate-600 dark:text-slate-400 font-bold">PATTERN:</span>
      <code class="px-2.5 py-0.5 rounded-full bg-slate-100 dark:bg-slate-950/60 text-cyan-600 dark:text-cyan-300 border border-slate-200/80 dark:border-white/5 whitespace-nowrap">
        {customTemplate}
      </code>
    </div>
    <div class="text-[11px] text-slate-500 dark:text-slate-400 whitespace-nowrap">
      Variables: <code>&#123;anime_title&#125;</code>, <code>&#123;season:02d&#125;</code>, <code>&#123;episode:02d&#125;</code>, <code>&#123;resolution&#125;</code>
    </div>
  </div>

  <!-- Filters & Controls Deck -->
  <div class="glass-panel p-3.5 rounded-3xl border border-slate-200/80 dark:border-white/10 flex flex-col sm:flex-row justify-between items-center gap-3">
    <div class="flex items-center gap-1.5 w-full sm:w-auto">
      {#each ['ALL', 'PENDING', 'ORGANIZED'] as const as st}
        <button
          onclick={() => (filterStatus = st)}
          class="btn-expressive px-4 py-1.5 rounded-full text-xs font-mono font-semibold transition-all cursor-pointer {filterStatus === st
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
        placeholder="Filter by title or filename..."
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

  <!-- Files List Table -->
  {#if library.loading && library.items.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-cyan-500 dark:border-cyan-400 border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-slate-500 dark:text-slate-400">Loading library files...</p>
    </div>
  {:else if filteredItems.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10 space-y-4">
      <FolderKanban class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">No library files found</h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto font-mono">
        Scan your downloads folder to automatically detect completed video files.
      </p>
      <div class="pt-2">
        <button
          onclick={handleScan}
          class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-cyan-400 to-sky-400 text-slate-950 font-bold rounded-2xl text-xs font-mono shadow-[0_0_15px_rgba(0,229,255,0.25)] cursor-pointer"
        >
          Scan Downloads Now
        </button>
      </div>
    </div>
  {:else}
    <div class="glass-panel rounded-3xl overflow-hidden shadow-2xl border border-slate-200/80 dark:border-white/10">
      <div class="overflow-x-auto">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="border-b border-slate-200/80 dark:border-white/5 bg-slate-100/90 dark:bg-slate-950/70 text-[10px] uppercase font-mono text-slate-600 dark:text-slate-300 backdrop-blur-md">
              <th class="py-3.5 px-4">Anime & Episode</th>
              <th class="py-3.5 px-4">File Path & Size</th>
              <th class="py-3.5 px-4">Destination Target</th>
              <th class="py-3.5 px-4">Status</th>
              <th class="py-3.5 px-4 text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/5 text-xs">
            {#each filteredItems as item (item.id)}
              <tr class="hover:bg-white/[0.04] transition-colors">
                <!-- Anime & Episode -->
                <td class="py-3.5 px-4">
                  <div class="flex items-start gap-3">
                    {#if item.anime_cover_path}
                      <img
                        src={getCoverUrl(item.anime_cover_path)}
                        alt="Cover"
                        class="w-10 h-14 object-cover rounded-xl border border-white/10 shrink-0 shadow-md"
                      />
                    {:else}
                      <div class="w-10 h-14 bg-slate-200 dark:bg-slate-900 rounded-xl border border-slate-300/60 dark:border-white/10 flex items-center justify-center shrink-0 text-slate-400 dark:text-slate-500 font-mono text-[10px]">
                        MEDIA
                      </div>
                    {/if}

                    <div class="min-w-0">
                      <div class="font-bold text-slate-900 dark:text-white truncate max-w-sm" title={item.anime_title || item.original_filename}>
                        {item.anime_title || item.original_filename}
                      </div>
                      <div class="text-[11px] font-mono text-slate-500 dark:text-slate-400 truncate max-w-sm mt-0.5">
                        {item.original_filename}
                      </div>

                      <div class="flex items-center gap-1.5 mt-2 flex-wrap">
                        <span class="px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-[10px] font-mono text-slate-700 dark:text-slate-300">
                          S{item.season || 1}
                        </span>
                        <span class="px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/5 border border-slate-200/80 dark:border-white/10 text-[10px] font-mono text-slate-700 dark:text-slate-300">
                          EP {item.episode || '01'}
                        </span>
                        <span class="px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-400/25 text-[10px] font-mono text-cyan-600 dark:text-cyan-300 font-semibold">
                          {item.resolution || '1080p'}
                        </span>
                      </div>
                    </div>
                  </div>
                </td>

                <!-- File Path & Size -->
                <td class="py-3.5 px-4 font-mono">
                  <div class="text-slate-900 dark:text-white text-xs font-bold">
                    {formatBytes(item.file_size_bytes)}
                  </div>
                  <div class="text-[10px] text-slate-400 dark:text-slate-500 truncate max-w-xs mt-1" title={item.current_path}>
                    {item.current_path}
                  </div>
                </td>

                <!-- Target Path -->
                <td class="py-3.5 px-4 font-mono">
                  {#if item.target_library_path}
                    <div class="text-[11px] text-cyan-600 dark:text-cyan-300 truncate max-w-xs" title={item.target_library_path}>
                      {item.target_library_path}
                    </div>
                  {:else}
                    <span class="text-slate-400 dark:text-slate-500 italic text-[11px]">Unassigned</span>
                  {/if}
                </td>

                <!-- Status -->
                <td class="py-3.5 px-4 whitespace-nowrap">
                  <div class="space-y-1">
                    <div>
                      {#if item.is_moved}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 shadow-[0_0_8px_rgba(50,215,75,0.2)]">
                          ✓ ORGANIZED
                        </span>
                      {:else}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/20 text-amber-300 border border-amber-500/40">
                          PENDING MOVE
                        </span>
                      {/if}
                    </div>
                    <div>
                      {#if item.is_encoded}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-purple-500/20 text-purple-300 border border-purple-500/40">
                          ✓ ENCODED
                        </span>
                      {:else}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono text-slate-700 dark:text-slate-300 bg-slate-200/60 dark:bg-white/5 border border-slate-300/60 dark:border-white/10">
                          RAW SOURCE
                        </span>
                      {/if}
                    </div>
                  </div>
                </td>

                <!-- Actions -->
                <td class="py-3.5 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5 font-mono">
                    {#if !item.is_moved}
                      <button
                        onclick={() => handleAutoOrganizeSingle(item)}
                        class="btn-expressive px-3 py-1 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-400/40 text-xs transition-colors cursor-pointer"
                        title="Auto organize file into destination folder"
                      >
                        ⚡ Auto Move
                      </button>
                    {/if}

                    <button
                      onclick={() => openEncodeModal(item)}
                      class="btn-expressive px-3 py-1 rounded-xl bg-purple-500/20 hover:bg-purple-500/30 text-purple-300 border border-purple-500/40 text-xs transition-colors cursor-pointer flex items-center gap-1"
                      title="Encode / transcode file with FFmpeg"
                    >
                      <Film class="w-3.5 h-3.5" />
                      <span>Encode...</span>
                    </button>

                    <button
                      onclick={() => openMoveModal(item)}
                      class="btn-expressive px-3 py-1 rounded-xl glass-pill text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white text-xs transition-colors cursor-pointer"
                      title="Move or rename file manually"
                    >
                      Move...
                    </button>

                    <button
                      onclick={() => handleDelete(item)}
                      class="btn-expressive p-1.5 rounded-xl glass-pill text-slate-400 hover:text-rose-500 dark:hover:text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/30 transition-colors cursor-pointer"
                      title="Untrack file"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

<!-- Manual Move / Rename Modal -->
{#if isMoveModalOpen && selectedFile}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl animate-fade-in">
    <div class="glass-panel-elevated rounded-3xl p-6 max-w-xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15 animate-scale-in">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Move class="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
          <span>Move or Rename Media File</span>
        </h3>
        <button
          onclick={() => (isMoveModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <form onsubmit={handleCustomMove} class="space-y-4 text-xs font-mono">
        <div class="space-y-1.5">
          <label for="current-path" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">SOURCE PATH</label>
          <input
            id="current-path"
            type="text"
            readonly
            value={selectedFile.current_path}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5 text-slate-500 dark:text-slate-400 select-all"
          />
        </div>

        <div class="space-y-1.5">
          <label for="target-dir" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">DESTINATION DIRECTORY</label>
          <input
            id="target-dir"
            type="text"
            required
            bind:value={customTargetDir}
            placeholder="e.g. /data/library/Anime/Dragon Ball Super/Season 01"
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        <div class="space-y-1.5">
          <label for="new-filename" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">NEW FILENAME</label>
          <input
            id="new-filename"
            type="text"
            required
            bind:value={customFilename}
            placeholder="e.g. Dragon Ball Super - S01E07 [1080p].mkv"
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        {#if moveError}
          <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-xl text-rose-600 dark:text-rose-300">
            {moveError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isMoveModalOpen = false)}
            class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isMoving || !customTargetDir.trim() || !customFilename.trim()}
            class="btn-expressive px-5 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white font-bold transition-all disabled:opacity-40 cursor-pointer flex items-center gap-2 shadow-[0_0_15px_rgba(0,229,255,0.25)]"
          >
            {#if isMoving}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Moving...</span>
            {:else}
              <Move class="w-4 h-4 stroke-[2.5]" />
              <span>Execute Move</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Encode File Modal -->
{#if isEncodeModalOpen && encodeTargetFile}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl animate-fade-in">
    <div class="glass-panel-elevated rounded-3xl p-6 max-w-xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15 animate-scale-in">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Film class="w-5 h-5 text-purple-600 dark:text-purple-400" />
          <span>Encode Media File</span>
        </h3>
        <button
          onclick={() => (isEncodeModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      {#if encodeSuccess}
        <div class="p-3 bg-emerald-500/10 border border-emerald-500/40 rounded-2xl text-xs font-mono text-emerald-600 dark:text-emerald-300 flex items-center justify-between">
          <span class="flex items-center gap-1.5">
            <Check class="w-4 h-4" />
            <span>{encodeSuccess}</span>
          </span>
          <a
            href="/encoder"
            class="underline font-bold text-cyan-600 dark:text-cyan-300 ml-2"
          >
            View in Transcoder →
          </a>
        </div>
      {/if}

      {#if encodeError}
        <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-2xl text-xs font-mono text-rose-600 dark:text-rose-300">
          {encodeError}
        </div>
      {/if}

      <form onsubmit={handleQueueEncode} class="space-y-4 text-xs font-mono">
        <div class="space-y-1.5">
          <label for="encode-source-path" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">SOURCE FILE</label>
          <input
            id="encode-source-path"
            type="text"
            readonly
            value={encodeTargetFile.current_path}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5 text-slate-500 dark:text-slate-400 select-all"
          />
          <div class="flex items-center gap-2 mt-1 text-[11px] text-slate-500 dark:text-slate-400">
            <span>Size: <strong class="text-slate-800 dark:text-white">{formatBytes(encodeTargetFile.file_size_bytes)}</strong></span>
            <span>•</span>
            <span>Resolution: <strong class="text-slate-800 dark:text-white">{encodeTargetFile.resolution || 'Original'}</strong></span>
          </div>
        </div>

        <div class="space-y-1.5">
          <label for="encode-preset" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">ENCODING PRESET</label>
          <select
            id="encode-preset"
            bind:value={selectedPresetId}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          >
            {#each presets as preset (preset.id)}
              <option value={preset.id} class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
                {preset.name} {preset.is_default ? '(Default)' : ''}
              </option>
            {/each}
          </select>
        </div>

        {#if selectedPreset}
          <div class="p-4 rounded-2xl glass-panel border border-slate-200/60 dark:border-white/5 space-y-2 text-[11px]">
            <div class="text-slate-800 dark:text-slate-300 font-semibold">{selectedPreset.description || selectedPreset.name}</div>
            <div class="grid grid-cols-2 sm:grid-cols-3 gap-2 text-slate-500 dark:text-slate-400">
              <div><span class="text-slate-400 dark:text-slate-500">Video:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.video_codec}</strong></div>
              <div><span class="text-slate-400 dark:text-slate-500">CRF:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.crf}</strong></div>
              <div><span class="text-slate-400 dark:text-slate-500">Speed:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.preset}</strong></div>
              <div><span class="text-slate-400 dark:text-slate-500">Scale:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.target_resolution || 'Original'}</strong></div>
              <div><span class="text-slate-400 dark:text-slate-500">Audio:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.audio_codec}</strong></div>
              <div><span class="text-slate-400 dark:text-slate-500">Container:</span> <strong class="text-slate-800 dark:text-white">{selectedPreset.container}</strong></div>
            </div>
          </div>
        {/if}

        <div class="space-y-1.5">
          <label for="encode-target-path" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">DESTINATION PATH (OPTIONAL)</label>
          <input
            id="encode-target-path"
            type="text"
            bind:value={customTargetFilePath}
            placeholder="Leave default or specify custom output path..."
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isEncodeModalOpen = false)}
            class="btn-expressive px-4 py-2 glass-pill text-slate-700 dark:text-slate-300 rounded-2xl cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isEncoding || !selectedPresetId}
            class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-purple-500 to-indigo-500 text-white font-bold rounded-2xl transition-all cursor-pointer disabled:opacity-40 flex items-center gap-2 shadow-[0_0_15px_rgba(168,85,247,0.3)]"
          >
            {#if isEncoding}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Queueing...</span>
            {:else}
              <Zap class="w-4 h-4" />
              <span>Queue Encode Job</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
