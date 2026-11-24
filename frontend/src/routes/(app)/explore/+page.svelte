<script lang="ts">
  import { api, getWebSocketUrl } from '#lib/api/client';
  import { auth } from '#lib/stores/auth.svelte';
  import { goto } from '$app/navigation';
  import type { NyaaRelease, NyaaReleaseDetail, NyaaFileItem, AnimeMetadata } from '#lib/api/types';
  import { onMount } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import {
    Compass,
    Search,
    RefreshCw,
    DownloadCloud,
    Copy,
    Check,
    ExternalLink,
    Folder,
    FileText,
    Star,
    Layers,
    Filter,
    X,
    ArrowUpDown,
    CheckCircle2,
    Sparkles,
    AlertCircle,
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight,
    ArrowUp
  } from 'lucide-svelte';

  let releases = $state<NyaaRelease[]>([]);
  let loading = $state<boolean>(true);
  let error = $state<string | null>(null);

  // Scroll & Pagination UX
  let scrollY = $state<number>(0);
  let tableTopRef = $state<HTMLElement | null>(null);
  let jumpPageInput = $state<string>('');
  let isPageChanging = $state<boolean>(false);

  // Filters
  let category = $state<string>('1_2');
  let searchQuery = $state<string>('');
  let userQuery = $state<string>('');
  let page = $state<number>(1);
  let sortOrder = $state<'desc' | 'asc'>('desc');

  // Modal State for Scraped Release
  let selectedRelease = $state<NyaaRelease | null>(null);
  let releaseDetail = $state<NyaaReleaseDetail | null>(null);
  let detailLoading = $state<boolean>(false);
  let detailError = $state<string | null>(null);
  let activeTab = $state<'nyaa' | 'metadata'>('nyaa');
  let copiedMagnetId = $state<number | null>(null);
  let downloadingIds = $state<Set<number>>(new Set());
  let downloadedSuccessIds = $state<Set<number>>(new Set());

  // Modal-specific interactive states
  let manualSyncTitle = $state<string>('');
  let isSyncingMetadata = $state<boolean>(false);
  let syncError = $state<string | null>(null);
  let syncSuccess = $state<string | null>(null);
  let detailCopiedMagnet = $state<boolean>(false);
  let detailCopiedTitle = $state<boolean>(false);
  let detailCopiedHash = $state<boolean>(false);
  let descViewMode = $state<'markdown' | 'raw'>('markdown');

  function renderMarkdown(content?: string | null): string {
    if (!content) return '';
    try {
      const rawHtml = marked.parse(content, { async: false, breaks: true, gfm: true }) as string;
      if (typeof window !== 'undefined' && typeof (DOMPurify as any).sanitize === 'function') {
        return (DOMPurify as any).sanitize(rawHtml);
      }
      return rawHtml;
    } catch {
      return content;
    }
  }

  let ws: WebSocket | null = null;

  async function downloadRelease(rel: NyaaRelease) {
    if (!rel.magnet_uri) return;
    if (!auth.isAuthenticated) {
      goto('/login?redirect=/explore');
      return;
    }
    downloadingIds = new Set(downloadingIds).add(rel.id);
    try {
      await api.downloader.add({ magnet_uri: rel.magnet_uri, nyaa_id: rel.id });
      downloadedSuccessIds = new Set(downloadedSuccessIds).add(rel.id);
      setTimeout(() => {
        downloadedSuccessIds.delete(rel.id);
        downloadedSuccessIds = new Set(downloadedSuccessIds);
      }, 4000);
    } catch (err: any) {
      alert(`Failed to start download: ${err.message}`);
    } finally {
      downloadingIds.delete(rel.id);
      downloadingIds = new Set(downloadingIds);
    }
  }

  async function loadReleases() {
    loading = true;
    error = null;
    try {
      const q = searchQuery.trim() || undefined;
      if (userQuery.trim().length > 0) {
        releases = await api.explore.getUserReleases(userQuery.trim(), {
          page,
          query: q,
          sort: 'id',
          order: sortOrder
        });
      } else {
        releases = await api.explore.getLatest({
          category,
          page,
          query: q,
          sort: 'id',
          order: sortOrder
        });
      }
    } catch (err: any) {
      error = err.message || 'Failed to load releases from Nyaa.si';
    } finally {
      loading = false;
    }
  }

  async function goToPage(targetPage: number) {
    if (targetPage < 1 || loading) return;
    page = targetPage;
    isPageChanging = true;

    // Smoothly scroll to the top of the releases table
    if (tableTopRef) {
      const yOffset = -75; // Account for the sticky floating header
      const y = tableTopRef.getBoundingClientRect().top + window.pageYOffset + yOffset;
      window.scrollTo({ top: Math.max(0, y), behavior: 'smooth' });
    }

    await loadReleases();
    isPageChanging = false;
  }

  function handleJumpPageSubmit(e: Event) {
    e.preventDefault();
    const p = parseInt(jumpPageInput.trim(), 10);
    if (!isNaN(p) && p >= 1) {
      jumpPageInput = '';
      goToPage(p);
    }
  }

  function scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function handleFilterSubmit(e: Event) {
    e.preventDefault();
    goToPage(1);
  }

  function toggleDateSort() {
    sortOrder = sortOrder === 'desc' ? 'asc' : 'desc';
    goToPage(1);
  }

  function filterByUploader(uploaderName?: string) {
    if (!uploaderName) return;
    userQuery = uploaderName;
    goToPage(1);
  }

  function formatDate(dateStr: string): string {
    if (!dateStr) return 'N/A';
    try {
      const d = new Date(dateStr);
      if (isNaN(d.getTime())) return dateStr;
      const now = new Date();
      const diffMs = now.getTime() - d.getTime();
      const diffSec = Math.floor(diffMs / 1000);
      const diffMin = Math.floor(diffSec / 60);
      const diffHour = Math.floor(diffMin / 60);
      const diffDay = Math.floor(diffHour / 24);

      if (diffSec < 60) return 'Just now';
      if (diffMin < 60) return `${diffMin}m ago`;
      if (diffHour < 24) return `${diffHour}h ago`;
      if (diffDay < 7) return `${diffDay}d ago`;
      return d.toISOString().split('T')[0];
    } catch {
      return dateStr;
    }
  }

  function formatFullDate(dateStr: string): string {
    try {
      const d = new Date(dateStr);
      return isNaN(d.getTime()) ? dateStr : d.toLocaleString();
    } catch {
      return dateStr;
    }
  }

  function clearUserFilter() {
    userQuery = '';
    goToPage(1);
  }

  function copyMagnet(release: NyaaRelease) {
    navigator.clipboard.writeText(release.magnet_uri);
    copiedMagnetId = release.id;
    setTimeout(() => {
      copiedMagnetId = null;
    }, 2000);
  }

  async function fetchReleaseDetail(id: number) {
    detailLoading = true;
    detailError = null;
    try {
      const detail = await api.explore.getView(id);
      releaseDetail = detail;
      if (selectedRelease && detail.anilist_metadata && !selectedRelease.anilist_metadata) {
        selectedRelease.anilist_metadata = detail.anilist_metadata;
      }
    } catch (err: any) {
      detailError = err.message || 'Failed to load details from nyaa.si';
    } finally {
      detailLoading = false;
    }
  }

  function openDetailModal(release: NyaaRelease, tab: 'nyaa' | 'metadata' = 'nyaa') {
    selectedRelease = release;
    activeTab = tab;
    manualSyncTitle = release.parsed.anime_title || release.title;
    syncError = null;
    syncSuccess = null;
    detailCopiedMagnet = false;
    detailCopiedTitle = false;
    detailCopiedHash = false;
    releaseDetail = null;
    fetchReleaseDetail(release.id);
  }

  function closeModal() {
    selectedRelease = null;
    releaseDetail = null;
    detailLoading = false;
    detailError = null;
    descViewMode = 'markdown';
    syncError = null;
    syncSuccess = null;
  }

  function copyDetailMagnet() {
    const magnet = releaseDetail?.magnet_uri || selectedRelease?.magnet_uri;
    if (!magnet) return;
    navigator.clipboard.writeText(magnet);
    detailCopiedMagnet = true;
    setTimeout(() => {
      detailCopiedMagnet = false;
    }, 2000);
  }

  function copyDetailTitle() {
    const title = releaseDetail?.title || selectedRelease?.title;
    if (!title) return;
    navigator.clipboard.writeText(title);
    detailCopiedTitle = true;
    setTimeout(() => {
      detailCopiedTitle = false;
    }, 2000);
  }

  function copyDetailHash() {
    if (!releaseDetail?.info_hash) return;
    navigator.clipboard.writeText(releaseDetail.info_hash);
    detailCopiedHash = true;
    setTimeout(() => {
      detailCopiedHash = false;
    }, 2000);
  }

  async function handleManualSync() {
    if (!selectedRelease || !manualSyncTitle.trim()) return;
    if (!auth.isAuthenticated) {
      syncError = 'Authentication required. Please sign in to sync metadata.';
      return;
    }
    isSyncingMetadata = true;
    syncError = null;
    syncSuccess = null;
    try {
      const meta = await api.explore.syncMetadata(manualSyncTitle.trim(), selectedRelease.id);
      if (meta) {
        selectedRelease.anilist_metadata = meta;
        const currentTitle = (selectedRelease.parsed.anime_title || '').toLowerCase();
        for (let i = 0; i < releases.length; i++) {
          if (
            releases[i].id === selectedRelease.id ||
            (currentTitle && releases[i].parsed?.anime_title?.toLowerCase() === currentTitle)
          ) {
            releases[i].anilist_metadata = meta;
          }
        }
        syncSuccess = 'AniList metadata synced successfully!';
      } else {
        syncError = 'No matching anime found on AniList for this title query.';
      }
    } catch (err: any) {
      syncError = err.message || 'Failed to sync metadata with AniList.';
    } finally {
      isSyncingMetadata = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && selectedRelease) {
      closeModal();
      return;
    }

    // Ignore keyboard shortcuts if modal is open or user is typing in an input
    if (selectedRelease) return;
    const activeEl = document.activeElement;
    if (
      activeEl &&
      (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA' || activeEl.tagName === 'SELECT')
    ) {
      return;
    }

    if (e.key === 'ArrowRight' && !loading && releases.length > 0) {
      goToPage(page + 1);
    } else if (e.key === 'ArrowLeft' && !loading && page > 1) {
      goToPage(page - 1);
    } else if (e.key === 'Home') {
      scrollToTop();
    }
  }

  onMount(() => {
    loadReleases();

    if (typeof window !== 'undefined') {
      try {
        const wsUrl = getWebSocketUrl('/api/ws');
        ws = new WebSocket(wsUrl);
        ws.onmessage = (event) => {
          try {
            const msg = JSON.parse(event.data);
            if (msg.event === 'anilist_metadata_synced' && msg.data?.title && msg.data?.metadata) {
              const syncedTitle = msg.data.title.toLowerCase();
              for (let i = 0; i < releases.length; i++) {
                if (releases[i].parsed?.anime_title && releases[i].parsed.anime_title.toLowerCase() === syncedTitle) {
                  releases[i].anilist_metadata = msg.data.metadata;
                }
              }
              if (
                selectedRelease &&
                selectedRelease.parsed?.anime_title &&
                selectedRelease.parsed.anime_title.toLowerCase() === syncedTitle
              ) {
                selectedRelease.anilist_metadata = msg.data.metadata;
              }
            }
          } catch (_) {}
        };
      } catch (_) {}
    }

    return () => {
      ws?.close();
    };
  });
</script>

<svelte:window onkeydown={handleKeydown} bind:scrollY={scrollY} />

<svelte:head>
  <title>Nyaa Explorer & AniList Sync - Nyaalist</title>
</svelte:head>

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl w-full mx-auto space-y-6">
  <!-- Top Bar & Expressive Filter Deck -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl space-y-5 border border-slate-200/80 dark:border-white/10">
    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-slate-200/80 dark:border-white/5 pb-4">
      <div>
        <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-3">
          <div class="relative flex items-center justify-center">
            <span class="w-3 h-3 rounded-full bg-cyan-500 dark:bg-cyan-400 animate-pulse"></span>
            <span class="absolute w-5 h-5 rounded-full bg-cyan-500/30 dark:bg-cyan-400/30 animate-ping"></span>
          </div>
          Nyaa Torrent Explorer
        </h1>
        <p class="text-xs text-slate-500 dark:text-slate-400 mt-1 font-mono">
          Live stream from nyaa.si with automatic title heuristics & AniList GraphQL synchronization.
        </p>
      </div>

      <div class="flex items-center gap-2 flex-wrap">
        {#if userQuery.trim().length > 0}
          <div class="glass-pill px-3 py-1.5 rounded-full text-xs font-mono flex items-center gap-2 border-cyan-400/40 text-cyan-600 dark:text-cyan-300">
            <span class="text-slate-500 dark:text-slate-400">Uploader:</span>
            <span class="font-bold">@{userQuery}</span>
            <button
              onclick={clearUserFilter}
              class="text-slate-400 hover:text-rose-500 dark:hover:text-rose-400 cursor-pointer font-bold ml-0.5"
              title="Clear uploader filter"
            >
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {/if}

        {#if searchQuery.trim().length > 0}
          <div class="glass-pill px-3 py-1.5 rounded-full text-xs font-mono flex items-center gap-2 border-slate-200/80 dark:border-white/20 text-slate-700 dark:text-white">
            <span class="text-slate-500 dark:text-slate-400">Query:</span>
            <span class="font-bold">"{searchQuery.trim()}"</span>
            <button
              onclick={() => { searchQuery = ''; page = 1; loadReleases(); }}
              class="text-slate-400 hover:text-rose-500 dark:hover:text-rose-400 cursor-pointer font-bold ml-0.5"
              title="Clear search query"
            >
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {/if}

        <button
          onclick={() => loadReleases()}
          disabled={loading}
          class="btn-expressive glass-pill px-4 py-2 rounded-2xl text-xs font-mono text-slate-700 dark:text-white transition-all cursor-pointer flex items-center gap-2"
        >
          <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin text-cyan-500 dark:text-cyan-400' : ''}" />
          <span>Refresh</span>
        </button>
      </div>
    </div>

    <!-- Filters Form -->
    <form onsubmit={handleFilterSubmit} class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-12 gap-3.5 text-xs font-mono">
      <div class="md:col-span-3 space-y-1.5">
        <label for="category" class="text-[10px] uppercase font-bold tracking-wider text-slate-600 dark:text-slate-400">Category</label>
        <select
          id="category"
          bind:value={category}
          disabled={userQuery.trim().length > 0}
          class="glass-input w-full rounded-2xl px-3 py-2.5 disabled:opacity-40"
        >
          <option value="1_2" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - English-translated (1_2)</option>
          <option value="1_0" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">All Anime (1_0)</option>
          <option value="1_4" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - Raw (1_4)</option>
          <option value="1_3" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Anime - Non-English (1_3)</option>
        </select>
      </div>

      <div class="md:col-span-3 space-y-1.5">
        <label for="user" class="text-[10px] uppercase font-bold tracking-wider text-slate-600 dark:text-slate-400">Uploader</label>
        <input
          id="user"
          type="text"
          bind:value={userQuery}
          placeholder="Erai-raws / SubsPlease"
          class="glass-input w-full rounded-2xl px-3.5 py-2.5"
        />
      </div>

      <div class="md:col-span-4 space-y-1.5">
        <label for="search" class="text-[10px] uppercase font-bold tracking-wider text-slate-600 dark:text-slate-400">Title Search Query</label>
        <div class="relative">
          <input
            id="search"
            type="text"
            bind:value={searchQuery}
            placeholder="e.g. Frieren, Solo Leveling..."
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>
      </div>

      <div class="md:col-span-2 flex items-end">
        <button
          type="submit"
          disabled={loading}
          class="btn-expressive w-full bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 font-bold py-2.5 px-4 rounded-2xl transition-all cursor-pointer disabled:opacity-40 shadow-[0_0_20px_rgba(0,229,255,0.25)] flex items-center justify-center gap-2"
        >
          <Filter class="w-3.5 h-3.5 stroke-[2.5]" />
          <span>Filter</span>
        </button>
      </div>
    </form>
  </div>

  <!-- Status / Alert Banners -->
  {#if error}
    <div class="glass-panel border-l-4 border-rose-500 p-4 rounded-2xl text-xs font-mono text-rose-300 bg-rose-950/20 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <AlertCircle class="w-4 h-4 text-rose-400" />
        <span>Error loading releases: {error}</span>
      </div>
      <button onclick={() => (error = null)} class="text-rose-500 hover:text-rose-700 dark:text-rose-400 dark:hover:text-white cursor-pointer font-bold">✕</button>
    </div>
  {/if}

  <!-- Releases Data Table -->
  <div bind:this={tableTopRef} class="glass-panel rounded-3xl overflow-hidden shadow-2xl border border-slate-200/80 dark:border-white/10">
    <!-- Active Loading Shimmer Bar -->
    {#if loading}
      <div class="h-0.5 w-full bg-gradient-to-r from-cyan-400 via-sky-400 to-indigo-500 animate-pulse"></div>
    {/if}

    <div class="overflow-x-auto">
      <table class="w-full text-left text-sm border-collapse">
        <thead class="bg-slate-100/90 dark:bg-slate-950/70 border-b border-slate-200/80 dark:border-white/10 text-[11px] font-mono text-slate-600 dark:text-slate-300 uppercase tracking-wider backdrop-blur-md">
          <tr>
            <th class="py-3.5 px-4 w-14 font-semibold">Cover</th>
            <th class="py-3.5 px-4 font-semibold">Release Title & Technical Tags</th>
            <th class="py-3.5 px-4 w-48 font-semibold">AniList Match</th>
            <th
              class="py-3.5 px-4 w-32 cursor-pointer select-none hover:text-slate-900 dark:hover:text-white transition-colors font-semibold"
              onclick={toggleDateSort}
              title="Click to sort by date ({sortOrder === 'desc' ? 'Newest first' : 'Oldest first'})"
            >
              <div class="flex items-center gap-1.5">
                <span>Date</span>
                <span class="text-cyan-500 dark:text-cyan-400 font-bold text-xs">{sortOrder === 'desc' ? '↓' : '↑'}</span>
              </div>
            </th>
            <th class="py-3.5 px-4 w-24 font-semibold">Size</th>
            <th class="py-3.5 px-4 w-24 font-semibold">S / L</th>
            <th class="py-3.5 px-4 w-44 text-right font-semibold">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200/60 dark:divide-white/5 transition-opacity duration-200 {loading && isPageChanging ? 'opacity-40 pointer-events-none' : 'opacity-100'}">
          {#if loading && releases.length === 0}
            <tr>
              <td colspan="7" class="py-16 text-center text-slate-500 dark:text-slate-400 font-mono text-xs">
                <div class="inline-block w-6 h-6 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin mb-3"></div>
                <div>Scraping Nyaa.si and querying AniList GraphQL API...</div>
              </td>
            </tr>
          {:else if releases.length === 0}
            <tr>
              <td colspan="7" class="py-16 text-center text-slate-500 dark:text-slate-400 font-mono text-xs">
                <Compass class="w-10 h-10 text-slate-400 dark:text-slate-500 mx-auto mb-2" />
                No releases found matching your search criteria.
              </td>
            </tr>
          {:else}
            {#each releases as release (release.id)}
              <tr
                class="hover:bg-white/[0.04] transition-colors group cursor-pointer"
                onclick={() => openDetailModal(release, 'nyaa')}
              >
                <!-- Cover Image Thumbnail -->
                <td class="py-3 px-4">
                  <button
                    type="button"
                    onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                    class="block w-10 h-14 rounded-xl overflow-hidden border border-slate-200 dark:border-white/10 group-hover:border-cyan-400/40 group-hover:shadow-[0_0_12px_rgba(0,229,255,0.25)] transition-all cursor-pointer bg-slate-100 dark:bg-slate-900/60"
                    title="View details for {release.parsed.anime_title}"
                  >
                    {#if release.anilist_metadata?.cover_image_local_path}
                      <img
                        src={release.anilist_metadata.cover_image_local_path}
                        alt="Anime Poster"
                        class="w-full h-full object-cover"
                        loading="lazy"
                      />
                    {:else if release.anilist_metadata?.cover_image_url}
                      <img
                        src={release.anilist_metadata.cover_image_url}
                        alt="Anime Poster"
                        class="w-full h-full object-cover"
                        loading="lazy"
                      />
                    {:else}
                      <div class="w-full h-full flex items-center justify-center text-[10px] text-slate-500 dark:text-slate-400 font-mono">
                        N/A
                      </div>
                    {/if}
                  </button>
                </td>

                <!-- Release Title & Badges -->
                <td class="py-3 px-4 space-y-1.5">
                  <div class="flex items-center gap-2 flex-wrap">
                    {#if release.parsed.uploader}
                      <button
                        type="button"
                        onclick={(e) => { e.stopPropagation(); filterByUploader(release.parsed.uploader); }}
                        class="text-xs font-mono font-semibold px-2 py-0.5 rounded-full bg-cyan-500/10 text-cyan-300 hover:bg-cyan-500/20 border border-cyan-400/25 transition-all cursor-pointer"
                        title="Filter by uploader @{release.parsed.uploader}"
                      >
                        [{release.parsed.uploader}]
                      </button>
                    {/if}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                      class="text-sm font-semibold text-slate-900 dark:text-white group-hover:text-cyan-600 dark:group-hover:text-cyan-300 transition-colors text-left cursor-pointer"
                    >
                      {release.parsed.anime_title || release.title}
                    </button>
                    {#if release.parsed.episode}
                      <span class="text-xs font-mono font-bold px-2 py-0.2 rounded-full bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                        EP {release.parsed.episode}
                      </span>
                    {/if}
                  </div>

                  <div class="text-xs text-slate-500 dark:text-slate-400 line-clamp-1 font-mono">
                    {release.title}
                  </div>

                  <!-- Technical Badges -->
                  <div class="flex items-center gap-1.5 flex-wrap text-[10px] font-mono">
                    {#if release.parsed.resolution}
                      <span class="px-2 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 border border-slate-300 dark:border-white/10 text-slate-700 dark:text-slate-300">
                        {release.parsed.resolution}
                      </span>
                    {/if}
                    {#if release.parsed.codec}
                      <span class="px-2 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 border border-slate-300 dark:border-white/10 text-slate-700 dark:text-slate-300">
                        {release.parsed.codec}
                      </span>
                    {/if}
                    {#if release.parsed.audio}
                      <span class="px-2 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 border border-slate-300 dark:border-white/10 text-slate-700 dark:text-slate-300">
                        {release.parsed.audio}
                      </span>
                    {/if}
                    {#if release.parsed.crc32}
                      <span class="px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-400/25 text-cyan-300">
                        [{release.parsed.crc32}]
                      </span>
                    {/if}
                  </div>
                </td>

                <!-- AniList Match -->
                <td class="py-3 px-4">
                  {#if release.anilist_metadata}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'metadata'); }}
                      class="text-left group/meta cursor-pointer"
                      title="View AniList metadata"
                    >
                      <div class="text-xs font-semibold text-slate-900 dark:text-white group-hover/meta:text-cyan-600 dark:group-hover/meta:text-cyan-300 line-clamp-1 transition-colors">
                        {release.anilist_metadata.title_english || release.anilist_metadata.title_romaji}
                      </div>
                      <div class="flex items-center gap-2 text-[10px] font-mono text-slate-500 dark:text-slate-400 mt-1">
                        {#if release.anilist_metadata.average_score}
                          <span class="text-emerald-300 font-bold flex items-center gap-0.5">
                            ★ {release.anilist_metadata.average_score}%
                          </span>
                        {/if}
                        <span class="px-1.5 py-0.2 rounded bg-slate-200/70 dark:bg-white/5 text-slate-700 dark:text-slate-300">
                          {release.anilist_metadata.format || 'TV'}
                        </span>
                        {#if release.anilist_metadata.episodes}
                          <span>({release.anilist_metadata.episodes} eps)</span>
                        {/if}
                      </div>
                    </button>
                  {:else}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'metadata'); }}
                      class="text-xs font-mono text-slate-500 dark:text-slate-400 hover:text-cyan-600 dark:hover:text-cyan-300 italic transition-colors cursor-pointer text-left"
                      title="Click to view or sync metadata"
                    >
                      No match (Sync)
                    </button>
                  {/if}
                </td>

                <!-- Date -->
                <td class="py-3 px-4 text-xs font-mono text-slate-600 dark:text-slate-300 whitespace-nowrap" title={formatFullDate(release.nyaa_created_at)}>
                  {formatDate(release.nyaa_created_at)}
                </td>

                <!-- File Size -->
                <td class="py-3 px-4 text-xs font-mono text-slate-600 dark:text-slate-300 whitespace-nowrap">
                  {release.file_size_formatted}
                </td>

                <!-- Seeds / Peers -->
                <td class="py-3 px-4 text-xs font-mono whitespace-nowrap">
                  <span class="text-emerald-400 font-bold">{release.seeders}</span>
                  <span class="text-slate-400 dark:text-slate-500"> / </span>
                  <span class="text-rose-400 font-bold">{release.leechers}</span>
                </td>

                <!-- Action Buttons -->
                <td class="py-3 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                      class="btn-expressive px-2.5 py-1 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-slate-200 hover:text-cyan-600 dark:hover:text-cyan-300 transition-all cursor-pointer"
                      title="View release detail modal"
                    >
                      Detail
                    </button>

                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); downloadRelease(release); }}
                      disabled={downloadingIds.has(release.id)}
                      class="btn-expressive px-3 py-1 rounded-xl text-xs font-mono font-semibold transition-all cursor-pointer flex items-center gap-1.5 {
                        downloadedSuccessIds.has(release.id)
                          ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-400/40 shadow-[0_0_12px_rgba(50,215,75,0.3)]'
                          : 'bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-600 dark:text-cyan-300 border border-cyan-400/40 shadow-[0_0_12px_rgba(0,229,255,0.2)]'
                      }"
                      title="Download torrent with librqbit engine"
                    >
                      {#if downloadingIds.has(release.id)}
                        <span class="inline-block animate-spin w-3 h-3 border border-cyan-400 border-t-transparent rounded-full"></span>
                        Starting...
                      {:else if downloadedSuccessIds.has(release.id)}
                        <Check class="w-3 h-3" />
                        <span>Added</span>
                      {:else}
                        <DownloadCloud class="w-3 h-3" />
                        <span>Download</span>
                      {/if}
                    </button>

                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); copyMagnet(release); }}
                      class="btn-expressive px-2.5 py-1 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white transition-all cursor-pointer"
                      title="Copy Magnet Link"
                    >
                      {copiedMagnetId === release.id ? '✓ Copied' : 'Magnet'}
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>

    <!-- Enhanced Bottom Pagination -->
    <div class="bg-slate-100/90 dark:bg-slate-950/70 border-t border-slate-200/80 dark:border-white/5 px-5 py-4 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs font-mono backdrop-blur-md">
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-slate-600 dark:text-slate-400">
          Page <strong class="text-cyan-600 dark:text-cyan-300 font-bold">{page}</strong>
          {#if releases.length > 0}
            <span class="text-slate-400 dark:text-slate-500 font-normal">({releases.length} results)</span>
          {/if}
        </span>

        <!-- Quick Jump to Page Form -->
        <form onsubmit={handleJumpPageSubmit} class="flex items-center gap-1.5">
          <label for="jump-page-bottom" class="text-slate-500 dark:text-slate-400 text-[11px] hidden md:inline">Go to:</label>
          <input
            id="jump-page-bottom"
            type="number"
            min="1"
            placeholder="#"
            bind:value={jumpPageInput}
            class="glass-input w-14 rounded-xl px-2 py-1 text-center text-xs font-mono"
          />
          <button
            type="submit"
            disabled={!jumpPageInput || loading}
            class="btn-expressive px-2.5 py-1 rounded-xl glass-pill text-[11px] text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white disabled:opacity-30 cursor-pointer"
          >
            Go
          </button>
        </form>
      </div>

      <!-- Navigation Action Buttons -->
      <div class="flex items-center gap-1.5 sm:gap-2">
        <button
          type="button"
          onclick={() => goToPage(1)}
          disabled={page <= 1 || loading}
          class="btn-expressive px-3 py-1.5 glass-pill rounded-xl text-slate-700 dark:text-white disabled:opacity-40 cursor-pointer flex items-center gap-1"
          title="First Page"
        >
          <ChevronsLeft class="w-3.5 h-3.5" />
          <span class="hidden md:inline">First</span>
        </button>

        <button
          type="button"
          onclick={() => goToPage(page - 1)}
          disabled={page <= 1 || loading}
          class="btn-expressive px-3.5 py-1.5 glass-pill rounded-xl text-slate-700 dark:text-white disabled:opacity-40 cursor-pointer flex items-center gap-1.5"
          title="Previous page (Left Arrow)"
        >
          <ChevronLeft class="w-3.5 h-3.5" />
          <span>Previous</span>
        </button>

        <!-- Current Page Pill -->
        <span class="px-3.5 py-1.5 rounded-xl bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-400/30 font-bold shadow-[0_0_12px_rgba(0,229,255,0.2)]">
          {page}
        </span>

        <button
          type="button"
          onclick={() => goToPage(page + 1)}
          disabled={loading || releases.length === 0}
          class="btn-expressive px-3.5 py-1.5 glass-pill rounded-xl text-slate-700 dark:text-white disabled:opacity-40 cursor-pointer flex items-center gap-1.5"
          title="Next page (Right Arrow)"
        >
          {#if loading && isPageChanging}
            <span class="inline-block animate-spin w-3 h-3 border border-cyan-400 border-t-transparent rounded-full"></span>
            <span>Loading...</span>
          {:else}
            <span>Next</span>
            <ChevronRight class="w-3.5 h-3.5" />
          {/if}
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Floating "Back to Top" Action Button -->
{#if scrollY > 400}
  <div class="fixed bottom-6 right-6 z-40 animate-fade-in">
    <button
      type="button"
      onclick={scrollToTop}
      class="glass-panel-elevated px-3.5 py-2.5 rounded-full text-slate-700 dark:text-slate-200 hover:text-cyan-600 dark:hover:text-cyan-300 hover:border-cyan-400/40 shadow-2xl hover:shadow-[0_0_20px_rgba(0,229,255,0.3)] transition-all cursor-pointer flex items-center gap-2 btn-expressive group border border-slate-200/90 dark:border-white/15"
      title="Scroll back to top (Home key)"
      aria-label="Back to top"
    >
      <ArrowUp class="w-4 h-4 group-hover:-translate-y-0.5 transition-transform text-cyan-600 dark:text-cyan-400" />
      <span class="text-xs font-mono font-semibold hidden sm:inline">Top</span>
      <span class="text-[10px] font-mono px-1.5 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-400/25 font-bold">
        P.{page}
      </span>
    </button>
  </div>
{/if}

<!-- Scraped Item Detail Modal (Material 3 Expressive Glass Modal) -->
{#if selectedRelease}
  <div
    class="fixed inset-0 bg-slate-950/80 backdrop-blur-xl z-50 flex items-center justify-center p-3 md:p-6 overflow-y-auto animate-fade-in"
    tabindex="-1"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') closeModal(); }}
  >
    <div
      class="glass-panel-elevated rounded-3xl max-w-3xl w-full my-auto shadow-2xl overflow-hidden flex flex-col max-h-[90vh] border border-white/15 animate-scale-in"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-release-title"
      tabindex="-1"
    >
      <!-- Modal Header -->
      <div class="p-5 sm:p-6 border-b border-white/10 bg-slate-950/40 flex items-start justify-between gap-4">
        <div class="space-y-1.5 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="px-2.5 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-400/25 text-[10px] font-mono font-semibold text-cyan-300">
              Nyaa #{selectedRelease.id}
            </span>
            <span class="px-2.5 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 border border-slate-300/60 dark:border-white/10 text-[10px] font-mono text-slate-700 dark:text-slate-300">
              {selectedRelease.category}
            </span>
            {#if selectedRelease.parsed.uploader || selectedRelease.uploader}
              <span class="px-2.5 py-0.5 rounded-full bg-purple-500/10 border border-purple-500/25 text-[10px] font-mono text-purple-300">
                @{selectedRelease.parsed.uploader || selectedRelease.uploader}
              </span>
            {/if}
            {#if selectedRelease.parsed.episode}
              <span class="px-2.5 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-[10px] font-mono font-bold text-emerald-300">
                EP {selectedRelease.parsed.episode}
              </span>
            {/if}
          </div>
          <h2 id="modal-release-title" class="text-lg sm:text-xl font-bold text-slate-900 dark:text-white tracking-tight truncate">
            {selectedRelease.parsed.anime_title || selectedRelease.title}
          </h2>
          <p class="text-xs text-slate-500 dark:text-slate-400 font-mono line-clamp-1" title={selectedRelease.title}>
            {selectedRelease.title}
          </p>
        </div>

        <button
          onclick={closeModal}
          class="text-slate-400 hover:text-slate-700 dark:hover:text-white p-2 rounded-full glass-pill transition-colors font-mono cursor-pointer shrink-0 btn-expressive"
          title="Close modal (Esc)"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Tab Navigation (M3 Expressive Pills) -->
      <div class="flex border-b border-slate-200/80 dark:border-white/5 bg-slate-100/50 dark:bg-slate-950/50 px-5 gap-2 pt-2 text-xs font-mono">
        <button
          type="button"
          onclick={() => activeTab = 'nyaa'}
          class="py-2.5 px-4 font-semibold rounded-t-2xl border-b-2 transition-all cursor-pointer flex items-center gap-2 {
            activeTab === 'nyaa'
              ? 'border-cyan-500 dark:border-cyan-400 text-cyan-600 dark:text-cyan-300 bg-cyan-500/10'
              : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
          }"
        >
          <span>Detail from Nyaa</span>
          <span class="text-[10px] px-2 py-0.2 rounded-full bg-slate-200 dark:bg-slate-900 border border-slate-300/60 dark:border-white/10 text-slate-700 dark:text-slate-300">
            {selectedRelease.file_size_formatted}
          </span>
        </button>

        <button
          type="button"
          onclick={() => activeTab = 'metadata'}
          class="py-2.5 px-4 font-semibold rounded-t-2xl border-b-2 transition-all cursor-pointer flex items-center gap-2 {
            activeTab === 'metadata'
              ? 'border-cyan-500 dark:border-cyan-400 text-cyan-600 dark:text-cyan-300 bg-cyan-500/10'
              : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
          }"
        >
          <span>AniList Metadata</span>
          {#if selectedRelease.anilist_metadata}
            <span class="text-[10px] px-2 py-0.2 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-bold">
              ✓ Synced
            </span>
          {:else}
            <span class="text-[10px] px-2 py-0.2 rounded-full bg-slate-200/70 dark:bg-white/5 text-slate-600 dark:text-slate-400 border border-slate-300/60 dark:border-white/10">
              Unsynced
            </span>
          {/if}
        </button>
      </div>

      <!-- Tab Content Area -->
      <div class="p-6 overflow-y-auto space-y-5 flex-1">
        {#if activeTab === 'nyaa'}
          <!-- TAB 1: DETAIL FROM NYAA -->
          {#snippet renderFileItem(item: NyaaFileItem, depth: number)}
            <div
              style="padding-left: {depth * 18}px"
              class="py-1.5 flex items-center justify-between text-xs font-mono border-b border-white/5 last:border-0 hover:bg-white/[0.04] rounded-xl px-2.5 transition-colors"
            >
              <div class="flex items-center gap-2 min-w-0">
                {#if item.is_folder}
                  <Folder class="w-3.5 h-3.5 text-cyan-600 dark:text-cyan-400 shrink-0" />
                  <span class="font-bold text-slate-900 dark:text-white truncate">{item.name}</span>
                {:else}
                  <FileText class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 shrink-0" />
                  <span class="text-slate-700 dark:text-slate-300 truncate">{item.name}</span>
                {/if}
              </div>
              {#if item.size_formatted}
                <span class="text-[10px] text-slate-500 dark:text-slate-400 bg-slate-200/60 dark:bg-slate-900/60 px-2 py-0.5 rounded-full border border-slate-300/40 dark:border-white/5 shrink-0 ml-2">
                  {item.size_formatted}
                </span>
              {/if}
            </div>
            {#if item.children && item.children.length > 0}
              {#each item.children as child}
                {@render renderFileItem(child, depth + 1)}
              {/each}
            {/if}
          {/snippet}

          <div class="space-y-5">
            <!-- Scrape Status Banner -->
            {#if detailLoading}
              <div class="glass-panel border border-cyan-400/40 rounded-2xl p-4 flex items-center justify-between text-xs font-mono text-cyan-600 dark:text-cyan-300 shadow-[0_0_20px_rgba(0,229,255,0.15)]">
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 border-2 border-cyan-500 dark:border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
                  <span>Scraping detailed release data from <strong class="text-slate-800 dark:text-white">nyaa.si/view/{selectedRelease.id}</strong>...</span>
                </div>
                <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-400/10 text-cyan-600 dark:text-cyan-300 border border-cyan-400/30">Live Scrape</span>
              </div>
            {:else if detailError}
              <div class="glass-panel border border-rose-500/40 rounded-2xl p-4 flex items-center justify-between text-xs font-mono text-rose-600 dark:text-rose-300 bg-rose-500/10 dark:bg-rose-950/20">
                <span>⚠️ Failed to scrap detail: {detailError}</span>
                <button
                  type="button"
                  onclick={() => selectedRelease && fetchReleaseDetail(selectedRelease.id)}
                  class="btn-expressive px-3 py-1 rounded-xl glass-pill text-slate-700 dark:text-white transition-colors cursor-pointer text-xs"
                >
                  Retry
                </button>
              </div>
            {/if}

            <!-- Full Title Card -->
            <div class="glass-card-interactive rounded-2xl p-4 space-y-2">
              <div class="flex items-center justify-between text-[11px] font-mono text-slate-500 dark:text-slate-400 uppercase">
                <span>Original Release Title</span>
                <button
                  type="button"
                  onclick={copyDetailTitle}
                  class="text-cyan-600 dark:text-cyan-400 hover:text-cyan-700 dark:hover:text-cyan-300 cursor-pointer flex items-center gap-1 font-mono text-[10px]"
                >
                  <Copy class="w-3 h-3" />
                  <span>{detailCopiedTitle ? 'Copied' : 'Copy Full Title'}</span>
                </button>
              </div>
              <div class="font-mono text-xs text-slate-800 dark:text-slate-200 select-all break-all bg-slate-100 dark:bg-slate-950/60 p-3 rounded-xl border border-slate-200/60 dark:border-white/5">
                {releaseDetail?.title || selectedRelease.title}
              </div>
            </div>

            <!-- Swarm & Metrics Cards -->
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs font-mono">
              <div class="glass-panel rounded-2xl p-3.5 text-center border border-slate-200/60 dark:border-white/5">
                <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">File Size</span>
                <span class="text-base font-bold text-slate-900 dark:text-white mt-1 block">
                  {releaseDetail?.file_size_formatted || selectedRelease.file_size_formatted}
                </span>
                <span class="text-[10px] text-slate-500 dark:text-slate-400 block truncate">
                  {(releaseDetail?.file_size_bytes || selectedRelease.file_size_bytes).toLocaleString()} B
                </span>
              </div>

              <div class="glass-panel rounded-2xl p-3.5 text-center border border-slate-200/60 dark:border-white/5">
                <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Seeders</span>
                <span class="text-base font-bold text-emerald-400 mt-1 block">
                  ▲ {releaseDetail?.seeders ?? selectedRelease.seeders}
                </span>
                <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Active uploaders</span>
              </div>

              <div class="glass-panel rounded-2xl p-3.5 text-center border border-slate-200/60 dark:border-white/5">
                <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Leechers</span>
                <span class="text-base font-bold text-rose-400 mt-1 block">
                  ▼ {releaseDetail?.leechers ?? selectedRelease.leechers}
                </span>
                <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Downloading</span>
              </div>

              <div class="glass-panel rounded-2xl p-3.5 text-center border border-slate-200/60 dark:border-white/5">
                <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Completed</span>
                <span class="text-base font-bold text-cyan-400 mt-1 block">
                  ✓ {releaseDetail?.completed_count ?? selectedRelease.completed_count}
                </span>
                <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Total snatches</span>
              </div>
            </div>

            <!-- Info Hash & Submitter Info -->
            {#if releaseDetail?.info_hash || releaseDetail?.information}
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs font-mono">
                {#if releaseDetail?.info_hash}
                  <div class="glass-panel rounded-2xl p-4 space-y-2 border border-white/5">
                    <div class="flex items-center justify-between text-[10px] uppercase text-slate-500 dark:text-slate-400">
                      <span>Torrent Info Hash (SHA-1)</span>
                      <button
                        type="button"
                        onclick={copyDetailHash}
                        class="text-cyan-400 hover:text-cyan-300 cursor-pointer flex items-center gap-1 text-[10px]"
                      >
                        <Copy class="w-3 h-3" />
                        <span>{detailCopiedHash ? 'Copied' : 'Copy Hash'}</span>
                      </button>
                    </div>
                    <div class="bg-slate-950/60 p-2.5 rounded-xl border border-white/5 text-cyan-300 font-mono text-xs select-all break-all tracking-wider">
                      {releaseDetail.info_hash}
                    </div>
                  </div>
                {/if}

                {#if releaseDetail?.information}
                  <div class="glass-panel rounded-2xl p-4 space-y-2 border border-white/5">
                    <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Submitter Information</span>
                    <div class="bg-slate-100 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5 text-slate-800 dark:text-slate-200 font-mono text-xs truncate">
                      {#if releaseDetail.information.startsWith('http')}
                        <a
                          href={releaseDetail.information}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="text-cyan-400 hover:underline flex items-center gap-1 truncate"
                        >
                          <span>{releaseDetail.information}</span>
                          <ExternalLink class="w-3.5 h-3.5" />
                        </a>
                      {:else}
                        {releaseDetail.information}
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Technical Specifications Grid -->
            <div class="glass-panel rounded-2xl p-5 space-y-3 border border-white/5">
              <h3 class="text-xs font-mono uppercase text-slate-500 dark:text-slate-400 tracking-wider border-b border-slate-200/60 dark:border-white/5 pb-2">
                Parsed Technical Specifications
              </h3>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-2.5 text-xs font-mono">
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Anime Title:</span>
                  <span class="text-slate-900 dark:text-white font-semibold truncate max-w-[200px]" title={releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title}>
                    {releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Episode:</span>
                  <span class="text-emerald-600 dark:text-emerald-400 font-bold">
                    {(releaseDetail?.parsed?.episode || selectedRelease.parsed.episode) ? `EP ${releaseDetail?.parsed?.episode || selectedRelease.parsed.episode}` : 'N/A (Batch/Movie)'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Resolution:</span>
                  <span class="text-slate-800 dark:text-slate-200">
                    {releaseDetail?.parsed?.resolution || selectedRelease.parsed.resolution || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Video Codec:</span>
                  <span class="text-slate-800 dark:text-slate-200">
                    {releaseDetail?.parsed?.codec || selectedRelease.parsed.codec || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Audio Codec:</span>
                  <span class="text-slate-800 dark:text-slate-200">
                    {releaseDetail?.parsed?.audio || selectedRelease.parsed.audio || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-1.5">
                  <span class="text-slate-500 dark:text-slate-400">Checksum (CRC32):</span>
                  <span class="text-cyan-600 dark:text-cyan-300">
                    {(releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32) ? `[${releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32}]` : 'N/A'}
                  </span>
                </div>
              </div>
            </div>

            <!-- Scraped Torrent File List -->
            {#if releaseDetail?.file_list && releaseDetail.file_list.length > 0}
              <div class="glass-panel rounded-2xl p-5 space-y-3 border border-white/5">
                <div class="flex items-center justify-between border-b border-white/5 pb-2 text-xs font-mono">
                  <div class="flex items-center gap-2">
                    <span class="uppercase text-slate-700 dark:text-slate-300 tracking-wider font-bold">Torrent Files</span>
                    <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/10 text-cyan-300 border border-cyan-400/25">
                      {releaseDetail.file_list.length} {releaseDetail.file_list.length === 1 ? 'item' : 'items'}
                    </span>
                  </div>
                  <span class="text-[10px] text-slate-500 dark:text-slate-400">nyaa.si/view/{selectedRelease.id}</span>
                </div>
                <div class="max-h-60 overflow-y-auto space-y-0.5 pr-1 bg-slate-950/60 p-2.5 rounded-xl border border-white/5">
                  {#each releaseDetail.file_list as item}
                    {@render renderFileItem(item, 0)}
                  {/each}
                </div>
              </div>
            {/if}

            <!-- Scraped Description & Notes (Markdown Formatted) -->
            {#if releaseDetail?.description_markdown}
              <div class="glass-panel rounded-2xl p-5 space-y-3 border border-white/5">
                <div class="flex items-center justify-between border-b border-white/5 pb-2 text-xs font-mono">
                  <div class="flex items-center gap-2">
                    <span class="uppercase text-slate-700 dark:text-slate-300 tracking-wider font-bold">Release Notes</span>
                    <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/10 text-cyan-300 border border-cyan-400/25">
                      Markdown
                    </span>
                  </div>
                  <div class="flex items-center gap-1.5 bg-slate-200/60 dark:bg-slate-950/60 border border-slate-300/40 dark:border-white/5 rounded-xl p-0.5">
                    <button
                      type="button"
                      onclick={() => descViewMode = 'markdown'}
                      class="px-2.5 py-1 rounded-lg text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'markdown' ? 'bg-cyan-500 dark:bg-cyan-400 text-white dark:text-slate-950 font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
                    >
                      Formatted
                    </button>
                    <button
                      type="button"
                      onclick={() => descViewMode = 'raw'}
                      class="px-2.5 py-1 rounded-lg text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'raw' ? 'bg-cyan-500 dark:bg-cyan-400 text-white dark:text-slate-950 font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
                    >
                      Raw
                    </button>
                  </div>
                </div>

                {#if descViewMode === 'markdown'}
                  <div class="max-h-96 overflow-y-auto pr-3 bg-slate-50/80 dark:bg-slate-950/60 p-4 rounded-xl border border-slate-200/80 dark:border-white/5 prose-nyaa break-words">
                    {@html renderMarkdown(releaseDetail.description_markdown)}
                  </div>
                {:else}
                  <div class="max-h-96 overflow-y-auto pr-2 bg-slate-50/80 dark:bg-slate-950/60 p-3 rounded-xl border border-slate-200/80 dark:border-white/5 text-xs font-mono text-slate-700 dark:text-slate-300 leading-relaxed whitespace-pre-wrap break-words select-all">
                    {releaseDetail.description_markdown}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Magnet URI Details -->
            <div class="glass-panel rounded-2xl p-4 space-y-2 border border-white/5">
              <div class="flex items-center justify-between text-[11px] font-mono text-slate-500 dark:text-slate-400 uppercase">
                <span>Magnet URI</span>
                <button
                  type="button"
                  onclick={copyDetailMagnet}
                  class="text-cyan-600 dark:text-cyan-400 hover:text-cyan-700 dark:hover:text-cyan-300 cursor-pointer flex items-center gap-1 font-mono text-[10px]"
                >
                  <Copy class="w-3 h-3" />
                  <span>{detailCopiedMagnet ? 'Copied' : 'Copy Magnet URI'}</span>
                </button>
              </div>
              <div class="font-mono text-[11px] text-slate-700 dark:text-slate-300 select-all break-all bg-slate-100 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5 max-h-20 overflow-y-auto">
                {releaseDetail?.magnet_uri || selectedRelease.magnet_uri}
              </div>
            </div>

            <!-- Action Buttons Footer inside Nyaa Tab -->
            <div class="flex items-center justify-between gap-3 pt-2 flex-wrap">
              <div class="flex items-center gap-2">
                <a
                  href="https://nyaa.si/view/{selectedRelease.id}"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="btn-expressive px-4 py-2 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-white hover:text-cyan-600 dark:hover:text-cyan-300 flex items-center gap-1.5"
                >
                  <span>Nyaa.si</span>
                  <ExternalLink class="w-3.5 h-3.5" />
                </a>
              </div>

              <div class="flex items-center gap-2.5">
                <button
                  type="button"
                  onclick={() => copyDetailMagnet()}
                  class="btn-expressive px-4 py-2 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-white cursor-pointer"
                >
                  {detailCopiedMagnet ? '✓ Magnet Copied' : 'Copy Magnet'}
                </button>

                <button
                  type="button"
                  onclick={() => {
                    const target = releaseDetail
                      ? {
                          ...selectedRelease!,
                          magnet_uri: releaseDetail.magnet_uri,
                          title: releaseDetail.title,
                          file_size_bytes: releaseDetail.file_size_bytes,
                          file_size_formatted: releaseDetail.file_size_formatted,
                        }
                      : selectedRelease;
                    if (target) downloadRelease(target);
                  }}
                  disabled={downloadingIds.has(selectedRelease.id)}
                  class="btn-expressive px-5 py-2 rounded-xl text-xs font-mono font-bold transition-all cursor-pointer flex items-center gap-2 {
                    downloadedSuccessIds.has(selectedRelease.id)
                      ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                      : 'bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 shadow-[0_0_20px_rgba(0,229,255,0.25)]'
                  }"
                >
                  {#if downloadingIds.has(selectedRelease.id)}
                    <span class="inline-block animate-spin w-3.5 h-3.5 border-2 border-slate-950 border-t-transparent rounded-full"></span>
                    <span>Starting...</span>
                  {:else if downloadedSuccessIds.has(selectedRelease.id)}
                    <Check class="w-4 h-4 stroke-[2.5]" />
                    <span>Added to Downloader</span>
                  {:else}
                    <DownloadCloud class="w-4 h-4 stroke-[2.5]" />
                    <span>Start Download</span>
                  {/if}
                </button>
              </div>
            </div>
          </div>
        {:else}
          <!-- TAB 2: METADATA -->
          <div class="space-y-5">
            {#if selectedRelease.anilist_metadata}
              <div class="space-y-4">
                {#if selectedRelease.anilist_metadata.banner_image_url}
                  <div class="w-full h-36 rounded-2xl overflow-hidden border border-white/10 relative bg-slate-900 shadow-lg">
                    <img
                      src={selectedRelease.anilist_metadata.banner_image_url}
                      alt="Banner"
                      class="w-full h-full object-cover opacity-60"
                    />
                    <div class="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/40 to-transparent"></div>
                  </div>
                {/if}

                <div class="flex flex-col sm:flex-row gap-5 items-start">
                  {#if selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                    <img
                      src={selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                      alt={selectedRelease.anilist_metadata.title_romaji}
                      class="w-32 h-44 object-cover rounded-2xl border border-white/10 shrink-0 bg-slate-900 shadow-2xl"
                    />
                  {/if}

                  <div class="space-y-2 flex-1 min-w-0">
                    <h3 class="text-xl font-bold text-slate-900 dark:text-white tracking-tight">
                      {selectedRelease.anilist_metadata.title_english || selectedRelease.anilist_metadata.title_romaji}
                    </h3>
                    <div class="text-xs font-mono text-cyan-300">
                      {selectedRelease.anilist_metadata.title_romaji}
                    </div>
                    {#if selectedRelease.anilist_metadata.title_native}
                      <div class="text-xs text-slate-500 dark:text-slate-400">
                        {selectedRelease.anilist_metadata.title_native}
                      </div>
                    {/if}

                    <div class="flex items-center gap-2 flex-wrap text-xs font-mono pt-1">
                      {#if selectedRelease.anilist_metadata.average_score}
                        <span class="px-2.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 font-bold">
                          ★ {selectedRelease.anilist_metadata.average_score}%
                        </span>
                      {/if}
                      <span class="px-2.5 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/10">
                        {selectedRelease.anilist_metadata.format || 'TV'}
                      </span>
                      <span class="px-2.5 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/10">
                        {selectedRelease.anilist_metadata.status || 'RELEASING'}
                      </span>
                      {#if selectedRelease.anilist_metadata.episodes}
                        <span class="px-2.5 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/10">
                          {selectedRelease.anilist_metadata.episodes} Episodes
                        </span>
                      {/if}
                    </div>

                    <!-- External Links -->
                    <div class="flex items-center gap-3 pt-1 text-xs font-mono">
                      <a
                        href="https://anilist.co/anime/{selectedRelease.anilist_metadata.id}"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-cyan-400 hover:text-cyan-300 flex items-center gap-1 font-semibold"
                      >
                        <span>AniList #{selectedRelease.anilist_metadata.id}</span>
                        <ExternalLink class="w-3.5 h-3.5" />
                      </a>
                      {#if selectedRelease.anilist_metadata.mal_id}
                        <span class="text-slate-400 dark:text-slate-600">•</span>
                        <a
                          href="https://myanimelist.net/anime/{selectedRelease.anilist_metadata.mal_id}"
                          target="_blank"
                          rel="noopener noreferrer"
                          class="text-indigo-400 hover:text-indigo-300 flex items-center gap-1 font-semibold"
                        >
                          <span>MAL #{selectedRelease.anilist_metadata.mal_id}</span>
                          <ExternalLink class="w-3.5 h-3.5" />
                        </a>
                      {/if}
                    </div>

                    {#if selectedRelease.anilist_metadata.genres && selectedRelease.anilist_metadata.genres.length > 0}
                      <div class="flex items-center gap-1.5 flex-wrap pt-1">
                        {#each selectedRelease.anilist_metadata.genres as genre}
                          <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-slate-200/80 dark:bg-slate-900/60 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/5">
                            {genre}
                          </span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>

                {#if selectedRelease.anilist_metadata.description}
                  <div class="glass-panel rounded-2xl p-5 space-y-2 border border-white/5">
                    <h4 class="text-xs font-mono uppercase text-slate-500 dark:text-slate-400 font-bold">Synopsis</h4>
                    <div class="text-xs text-slate-700 dark:text-slate-300 leading-relaxed max-h-48 overflow-y-auto pr-2 space-y-1">
                      {@html selectedRelease.anilist_metadata.description}
                    </div>
                  </div>
                {/if}
              </div>
            {:else}
              <div class="glass-panel rounded-2xl p-8 text-center space-y-3 border border-slate-200/60 dark:border-white/5">
                <Compass class="w-10 h-10 text-slate-400 dark:text-slate-600 mx-auto" />
                <h3 class="text-sm font-bold text-slate-900 dark:text-white font-mono">No AniList Metadata Match</h3>
                <p class="text-xs text-slate-500 dark:text-slate-400 max-w-md mx-auto">
                  No metadata has been synchronized yet for title
                  <span class="text-cyan-600 dark:text-cyan-300 font-semibold">"{selectedRelease.parsed.anime_title || selectedRelease.title}"</span>.
                </p>
              </div>
            {/if}

            <!-- Sync / Re-sync Query Box -->
            <div class="glass-panel rounded-2xl p-5 space-y-3 border border-white/5">
              <div class="flex items-center justify-between text-xs font-mono">
                <span class="text-slate-600 dark:text-slate-400 uppercase text-[10px] font-bold">
                  {selectedRelease.anilist_metadata ? 'Re-sync with Different Title' : 'Sync Metadata with AniList'}
                </span>
                {#if selectedRelease.anilist_metadata?.last_synced_at}
                  <span class="text-[10px] text-slate-500 dark:text-slate-400">
                    Last synced: {formatDate(selectedRelease.anilist_metadata.last_synced_at)}
                  </span>
                {/if}
              </div>

              {#if syncError}
                <div class="p-3 rounded-xl bg-rose-950/30 border border-rose-500/40 text-xs font-mono text-rose-300">
                  {syncError}
                </div>
              {/if}

              {#if syncSuccess}
                <div class="p-3 rounded-xl bg-emerald-950/30 border border-emerald-500/40 text-xs font-mono text-emerald-300">
                  {syncSuccess}
                </div>
              {/if}

              {#if !auth.isAuthenticated}
                <div class="p-3.5 rounded-2xl glass-pill flex items-center justify-between text-xs font-mono">
                  <span class="text-slate-600 dark:text-slate-400">Sign in to manually search & sync AniList metadata for this title.</span>
                  <a href="/login?redirect=/explore" class="text-cyan-600 dark:text-cyan-400 hover:text-cyan-700 dark:hover:text-cyan-300 font-bold">Sign In &rarr;</a>
                </div>
              {:else}
                <div class="flex gap-2">
                  <input
                    type="text"
                    bind:value={manualSyncTitle}
                    placeholder="Enter anime title (e.g. Frieren: Beyond Journey's End)"
                    class="glass-input flex-1 rounded-2xl px-3.5 py-2.5 text-xs font-mono"
                  />
                  <button
                    type="button"
                    onclick={handleManualSync}
                    disabled={isSyncingMetadata || !manualSyncTitle.trim()}
                    class="btn-expressive bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 font-bold text-xs font-mono px-4 py-2.5 rounded-2xl transition-all cursor-pointer disabled:opacity-40 flex items-center gap-1.5 shrink-0 shadow-[0_0_15px_rgba(0,229,255,0.2)]"
                  >
                    {#if isSyncingMetadata}
                      <span class="inline-block animate-spin w-3 h-3 border-2 border-slate-950 border-t-transparent rounded-full"></span>
                      <span>Syncing...</span>
                    {:else}
                      <Sparkles class="w-3.5 h-3.5 stroke-[2.5]" />
                      <span>Sync AniList</span>
                    {/if}
                  </button>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
