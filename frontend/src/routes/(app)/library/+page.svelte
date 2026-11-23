<script lang="ts">
  import { onMount } from 'svelte';
  import { library } from '#lib/stores/library.svelte';
  import type { LibraryFileItem } from '#lib/api/types';

  let filterStatus = $state<'ALL' | 'PENDING' | 'ORGANIZED'>('ALL');
  let searchQuery = $state('');

  // Custom move modal state
  let isMoveModalOpen = $state(false);
  let selectedFile = $state<LibraryFileItem | null>(null);
  let customTargetDir = $state('');
  let customFilename = $state('');
  let moveError = $state<string | null>(null);
  let isMoving = $state(false);

  // Template editor modal
  let customTemplate = $state('{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}');

  onMount(() => {
    library.init();
  });

  const filteredItems = $derived(
    library.items.filter(item => {
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

  function openMoveModal(item: LibraryFileItem) {
    selectedFile = item;
    // Derive recommended target folder
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
</script>

<svelte:head>
  <title>Library Management | Nyaalist</title>
</svelte:head>

<div class="space-y-6">
  <!-- Header & Actions -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-white flex items-center gap-3">
        <svg class="w-7 h-7 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
        </svg>
        Anime Library Management
      </h1>
      <p class="text-xs text-gray-400 mt-1 font-mono">
        Scan, match with AniList metadata, and organize media into structured Plex/Jellyfin directories
      </p>
    </div>

    <div class="flex items-center gap-3">
      <!-- Scan Button -->
      <button
        onclick={handleScan}
        disabled={library.scanning}
        class="flex items-center gap-2 px-3.5 py-2 rounded-lg bg-[#1E1E1E] hover:bg-[#252525] border border-[#2C2C2E] text-white font-mono text-xs transition-all disabled:opacity-50 cursor-pointer"
      >
        <span class={library.scanning ? 'animate-spin' : ''}>↻</span>
        {library.scanning ? 'Scanning...' : 'Scan Downloads'}
      </button>

      <!-- Auto-Organize All -->
      <button
        onclick={handleAutoOrganizeAll}
        disabled={library.organizing || library.pendingCount === 0}
        class="flex items-center gap-2 px-4 py-2 bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold text-xs font-mono rounded-lg transition-all shadow-[0_0_15px_rgba(0,229,255,0.3)] disabled:opacity-50 cursor-pointer"
      >
        {#if library.organizing}
          <span class="inline-block animate-spin w-3 h-3 border-2 border-black border-t-transparent rounded-full"></span>
          Organizing...
        {:else}
          ⚡ Auto-Organize All ({library.pendingCount})
        {/if}
      </button>
    </div>
  </div>

  <!-- Telemetry KPI Banners -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <!-- Total Files -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <span class="text-xs font-medium text-gray-400">TOTAL FILES</span>
      <div class="mt-2 text-2xl font-bold font-mono text-white">
        {library.totalCount}
      </div>
    </div>

    <!-- Organized -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">ORGANIZED & MOVED</span>
        <span class="text-xs font-mono text-[#32D74B]">✓ READY</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-[#32D74B]">
        {library.movedCount}
      </div>
    </div>

    <!-- Pending -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <div class="flex justify-between items-start">
        <span class="text-xs font-medium text-gray-400">PENDING MOVE</span>
        <span class="text-xs font-mono text-amber-400">IN DOWNLOADS</span>
      </div>
      <div class="mt-2 text-2xl font-bold font-mono text-amber-400">
        {library.pendingCount}
      </div>
    </div>

    <!-- Storage Used -->
    <div class="p-4 rounded-xl bg-[#1E1E1E] border border-[#2C2C2E]">
      <span class="text-xs font-medium text-gray-400">TRACKED STORAGE</span>
      <div class="mt-2 text-2xl font-bold font-mono text-[#00E5FF]">
        {formatBytes(library.totalSizeBytes)}
      </div>
    </div>
  </div>

  <!-- Template Info Bar -->
  <div class="p-3 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E] flex flex-col md:flex-row md:items-center justify-between gap-3 text-xs font-mono">
    <div class="flex items-center gap-2 overflow-x-auto">
      <span class="text-gray-400 font-semibold">PATTERN:</span>
      <code class="px-2 py-0.5 rounded bg-[#121212] text-[#00E5FF] border border-[#2C2C2E] whitespace-nowrap">
        {customTemplate}
      </code>
    </div>
    <div class="text-[11px] text-gray-500 whitespace-nowrap">
      Variables: <code>&#123;anime_title&#125;</code>, <code>&#123;season:02d&#125;</code>, <code>&#123;episode:02d&#125;</code>, <code>&#123;resolution&#125;</code>
    </div>
  </div>

  <!-- Filters & Controls Bar -->
  <div class="flex flex-col sm:flex-row justify-between items-center gap-4 bg-[#1E1E1E] p-3 rounded-xl border border-[#2C2C2E]">
    <div class="flex items-center gap-1.5 w-full sm:w-auto">
      {#each ['ALL', 'PENDING', 'ORGANIZED'] as const as st}
        <button
          onclick={() => (filterStatus = st)}
          class="px-3 py-1.5 rounded-lg text-xs font-mono font-medium transition-all cursor-pointer {filterStatus === st
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
        placeholder="Filter by title or filename..."
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

  <!-- Files List Table -->
  {#if library.loading && library.items.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <div class="inline-block animate-spin w-8 h-8 border-2 border-[#00E5FF] border-t-transparent rounded-full mb-3"></div>
      <p class="text-sm font-mono text-gray-400">Loading library files...</p>
    </div>
  {:else if filteredItems.length === 0}
    <div class="text-center py-16 bg-[#1E1E1E] rounded-xl border border-[#2C2C2E]">
      <svg class="w-12 h-12 mx-auto text-gray-600 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
      </svg>
      <h3 class="text-base font-semibold text-white">No library files found</h3>
      <p class="text-xs text-gray-400 mt-1 max-w-sm mx-auto">
        Scan your downloads folder to automatically detect completed video files.
      </p>
      <div class="mt-4">
        <button
          onclick={handleScan}
          class="px-4 py-2 bg-[#00E5FF] text-black font-semibold rounded-lg text-xs font-mono cursor-pointer"
        >
          Scan Downloads Now
        </button>
      </div>
    </div>
  {:else}
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-xl overflow-hidden shadow-xl">
      <div class="overflow-x-auto">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="border-b border-[#2C2C2E] bg-[#121212]/80 text-[10px] uppercase font-mono text-gray-400">
              <th class="py-3 px-4">Anime & Episode</th>
              <th class="py-3 px-4">File Path & Size</th>
              <th class="py-3 px-4">Destination Target</th>
              <th class="py-3 px-4">Status</th>
              <th class="py-3 px-4 text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[#2C2C2E]/60 text-xs">
            {#each filteredItems as item (item.id)}
              <tr class="hover:bg-white/5 transition-colors">
                <!-- Anime & Episode -->
                <td class="py-3.5 px-4">
                  <div class="flex items-start gap-3">
                    {#if item.anime_cover_path}
                      <img
                        src="/api/covers/{item.anime_cover_path}"
                        alt="Cover"
                        class="w-10 h-14 object-cover rounded border border-[#2C2C2E] shrink-0"
                      />
                    {:else}
                      <div class="w-10 h-14 bg-[#121212] rounded border border-[#2C2C2E] flex items-center justify-center shrink-0 text-gray-600 font-mono text-[10px]">
                        MEDIA
                      </div>
                    {/if}

                    <div class="min-w-0">
                      <div class="font-semibold text-white truncate max-w-sm" title={item.anime_title || item.original_filename}>
                        {item.anime_title || item.original_filename}
                      </div>
                      <div class="text-[11px] font-mono text-gray-400 truncate max-w-sm mt-0.5">
                        {item.original_filename}
                      </div>

                      <div class="flex items-center gap-1.5 mt-2 flex-wrap">
                        <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-gray-300">
                          S{item.season || 1}
                        </span>
                        <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-gray-300">
                          EP {item.episode || '01'}
                        </span>
                        <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-[#00E5FF]">
                          {item.resolution || '1080p'}
                        </span>
                      </div>
                    </div>
                  </div>
                </td>

                <!-- File Path & Size -->
                <td class="py-3.5 px-4 font-mono">
                  <div class="text-white text-xs font-bold">
                    {formatBytes(item.file_size_bytes)}
                  </div>
                  <div class="text-[10px] text-gray-500 truncate max-w-xs mt-1" title={item.current_path}>
                    {item.current_path}
                  </div>
                </td>

                <!-- Target Path -->
                <td class="py-3.5 px-4 font-mono">
                  {#if item.target_library_path}
                    <div class="text-[11px] text-[#00E5FF] truncate max-w-xs" title={item.target_library_path}>
                      {item.target_library_path}
                    </div>
                  {:else}
                    <span class="text-gray-500 italic text-[11px]">Unassigned</span>
                  {/if}
                </td>

                <!-- Status -->
                <td class="py-3.5 px-4 whitespace-nowrap">
                  {#if item.is_moved}
                    <span class="px-2.5 py-1 rounded text-[10px] font-mono font-bold bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/40 shadow-[0_0_8px_rgba(50,215,75,0.2)]">
                      ✓ ORGANIZED
                    </span>
                  {:else}
                    <span class="px-2.5 py-1 rounded text-[10px] font-mono font-bold bg-amber-500/20 text-amber-400 border border-amber-500/40">
                      PENDING MOVE
                    </span>
                  {/if}
                </td>

                <!-- Actions -->
                <td class="py-3.5 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5 font-mono">
                    {#if !item.is_moved}
                      <button
                        onclick={() => handleAutoOrganizeSingle(item)}
                        class="px-2.5 py-1 rounded-lg bg-[#00E5FF]/20 hover:bg-[#00E5FF]/30 text-[#00E5FF] border border-[#00E5FF]/40 text-xs transition-colors cursor-pointer"
                        title="Auto organize file into destination folder"
                      >
                        ⚡ Auto Move
                      </button>
                    {/if}

                    <button
                      onclick={() => openMoveModal(item)}
                      class="px-2.5 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 border border-[#2C2C2E] text-xs transition-colors cursor-pointer"
                      title="Move or rename file manually"
                    >
                      Move...
                    </button>

                    <button
                      onclick={() => handleDelete(item)}
                      class="p-1 rounded-lg bg-white/5 hover:bg-[#FF453A]/20 text-gray-400 hover:text-[#FF453A] border border-[#2C2C2E] transition-colors cursor-pointer"
                      title="Untrack file"
                    >
                      ✕
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
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-6 max-w-xl w-full shadow-2xl relative space-y-4">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-white flex items-center gap-2">
          <svg class="w-5 h-5 text-[#00E5FF]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
          </svg>
          Move or Rename Media File
        </h3>
        <button
          onclick={() => (isMoveModalOpen = false)}
          class="text-gray-400 hover:text-white cursor-pointer"
        >
          ✕
        </button>
      </div>

      <form onsubmit={handleCustomMove} class="space-y-4 text-xs font-mono">
        <div>
          <label for="current-path" class="block text-gray-400 mb-1">SOURCE PATH</label>
          <input
            id="current-path"
            type="text"
            readonly
            value={selectedFile.current_path}
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-gray-400 select-all"
          />
        </div>

        <div>
          <label for="target-dir" class="block text-gray-400 mb-1">DESTINATION DIRECTORY</label>
          <input
            id="target-dir"
            type="text"
            required
            bind:value={customTargetDir}
            placeholder="e.g. /data/library/Anime/Dragon Ball Super/Season 01"
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-[#00E5FF]"
          />
        </div>

        <div>
          <label for="new-filename" class="block text-gray-400 mb-1">NEW FILENAME</label>
          <input
            id="new-filename"
            type="text"
            required
            bind:value={customFilename}
            placeholder="e.g. Dragon Ball Super - S01E07 [1080p].mkv"
            class="w-full bg-[#121212] border border-[#2C2C2E] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-[#00E5FF]"
          />
        </div>

        {#if moveError}
          <div class="p-3 bg-[#FF453A]/10 border border-[#FF453A]/30 rounded-lg text-[#FF453A]">
            {moveError}
          </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isMoveModalOpen = false)}
            class="px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isMoving || !customTargetDir.trim() || !customFilename.trim()}
            class="px-5 py-2 rounded-lg bg-[#00E5FF] hover:bg-[#00E5FF]/90 text-black font-semibold transition-all disabled:opacity-50 cursor-pointer flex items-center gap-2"
          >
            {#if isMoving}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-black border-t-transparent rounded-full"></span>
              Moving...
            {:else}
              Execute Move
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
