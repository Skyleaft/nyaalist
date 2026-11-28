<script lang="ts">
  import { api, getWebSocketUrl } from '#lib/api/client';
  import { auth } from '#lib/stores/auth.svelte';
  import { goto } from '$app/navigation';
  import type { NyaaRelease, NyaaReleaseDetail, NyaaFileItem } from '#lib/api/types';
  import { onMount } from 'svelte';
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
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
    ArrowUp,
    LayoutGrid,
    List,
    Magnet,
    Info,
    HardDrive,
    Clock,
    RotateCcw
  } from 'lucide-svelte';

  const QUICK_UPLOADERS = ['SubsPlease', 'Erai-raws', 'ASW', 'EMBER', 'Judas'];

  let releases = $state<NyaaRelease[]>([]);
  let loading = $state<boolean>(true);
  let error = $state<string | null>(null);

  // Scroll & Pagination UX
  let scrollY = $state<number>(0);
  let resultsTopRef = $state<HTMLElement | null>(null);
  let jumpPageInput = $state<string>('');
  let isPageChanging = $state<boolean>(false);

  // Server Filters
  let category = $state<string>('1_2');
  let searchQuery = $state<string>('');
  let userQuery = $state<string>('');
  let page = $state<number>(1);
  let sortOrder = $state<'desc' | 'asc'>('desc');

  // Client View & Instant Page Refinement
  let viewMode = $state<'grid' | 'table'>('grid');
  let pageSort = $state<'nyaa' | 'seeders' | 'size'>('nyaa');
  let quickFilter = $state<'all' | 'matched' | '1080p'>('all');

  // Non-blocking toast
  let toast = $state<{ message: string; tone: 'success' | 'error' | 'info' } | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

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

  let ws: WebSocket | null = null;

  const displayedReleases = $derived.by(() => {
    let list = releases.filter((r) => {
      if (quickFilter === 'matched' && !r.anilist_metadata) return false;
      if (quickFilter === '1080p' && !r.parsed.resolution?.toLowerCase().includes('1080')) {
        return false;
      }
      return true;
    });

    if (pageSort === 'seeders') {
      list = [...list].sort((a, b) => b.seeders - a.seeders);
    } else if (pageSort === 'size') {
      list = [...list].sort((a, b) => (b.file_size_bytes || 0) - (a.file_size_bytes || 0));
    }
    return list;
  });

  const matchedCount = $derived(releases.filter((r) => Boolean(r.anilist_metadata)).length);
  const hasActiveFilters = $derived(
    category !== '1_2' ||
      userQuery.trim().length > 0 ||
      searchQuery.trim().length > 0 ||
      quickFilter !== 'all' ||
      pageSort !== 'nyaa'
  );

  function showToast(message: string, tone: 'success' | 'error' | 'info' = 'info') {
    toast = { message, tone };
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast = null;
    }, 4000);
  }

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

  function getPosterUrl(rel: NyaaRelease): string | undefined {
    return rel.anilist_metadata?.cover_image_local_path || rel.anilist_metadata?.cover_image_url || undefined;
  }

  function getBackdropUrl(rel: NyaaRelease): string | undefined {
    return (
      rel.anilist_metadata?.banner_image_url ||
      rel.anilist_metadata?.cover_image_local_path ||
      rel.anilist_metadata?.cover_image_url ||
      undefined
    );
  }

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
      showToast(`Added "${rel.parsed.anime_title || rel.title}" to downloader`, 'success');
      setTimeout(() => {
        const next = new Set(downloadedSuccessIds);
        next.delete(rel.id);
        downloadedSuccessIds = next;
      }, 4000);
    } catch (err: any) {
      showToast(`Failed to start download: ${err.message}`, 'error');
    } finally {
      const next = new Set(downloadingIds);
      next.delete(rel.id);
      downloadingIds = next;
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

    if (resultsTopRef) {
      const yOffset = -84;
      const y = resultsTopRef.getBoundingClientRect().top + window.pageYOffset + yOffset;
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
    userQuery = userQuery.trim().toLowerCase() === uploaderName.toLowerCase() ? '' : uploaderName;
    goToPage(1);
  }

  function resetAllFilters() {
    category = '1_2';
    searchQuery = '';
    userQuery = '';
    sortOrder = 'desc';
    quickFilter = 'all';
    pageSort = 'nyaa';
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

  async function copyMagnet(release: NyaaRelease) {
    try {
      await navigator.clipboard.writeText(release.magnet_uri);
      copiedMagnetId = release.id;
      showToast('Magnet URI copied to clipboard', 'info');
      setTimeout(() => {
        if (copiedMagnetId === release.id) copiedMagnetId = null;
      }, 2000);
    } catch {
      showToast('Failed to copy magnet URI', 'error');
    }
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

  const selectedIndex = $derived(
    selectedRelease ? displayedReleases.findIndex((r) => r.id === selectedRelease!.id) : -1
  );

  function openAdjacentRelease(offset: -1 | 1) {
    if (selectedIndex < 0) return;
    const next = displayedReleases[selectedIndex + offset];
    if (next) {
      openDetailModal(next, activeTab);
    }
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

{#snippet releaseCard(release: NyaaRelease)}
  {@const posterUrl = getPosterUrl(release)}
  {@const backdropUrl = getBackdropUrl(release)}
  {@const isDownloading = downloadingIds.has(release.id)}
  {@const isDownloaded = downloadedSuccessIds.has(release.id)}
  {@const isCopied = copiedMagnetId === release.id}
  <article
    class="glass-card-interactive rounded-3xl overflow-hidden flex flex-col group relative"
  >
    <!-- Visual Cover / Backdrop Header -->
    <div
      role="button"
      tabindex="0"
      onclick={() => openDetailModal(release, 'nyaa')}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          openDetailModal(release, 'nyaa');
        }
      }}
      class="relative h-48 bg-slate-900 overflow-hidden cursor-pointer text-left shrink-0 focus:outline-none focus:ring-2 focus:ring-cyan-400"
    >
      {#if backdropUrl}
        <img
          src={backdropUrl}
          alt=""
          aria-hidden="true"
          class="absolute inset-0 w-full h-full object-cover scale-110 blur-xl opacity-50 transition-transform duration-500 group-hover:scale-125"
          loading="lazy"
        />
        <div class="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/45 to-slate-950/35"></div>
      {:else}
        <div class="absolute inset-0 bg-gradient-to-br from-slate-800 via-slate-900 to-slate-950"></div>
      {/if}

      <!-- Top Overlay: Uploader & AniList Score / Match Status -->
      <div class="absolute top-2.5 left-2.5 right-2.5 z-20 flex items-center justify-between gap-1.5">
        {#if release.parsed.uploader}
          <button
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              filterByUploader(release.parsed.uploader);
            }}
            class="px-2.5 py-0.5 rounded-full bg-slate-950/70 hover:bg-cyan-500/30 backdrop-blur-md border border-white/15 hover:border-cyan-400/50 text-[10px] font-mono font-semibold text-cyan-300 transition-all cursor-pointer truncate max-w-[55%]"
            title="Filter by uploader @{release.parsed.uploader}"
          >
            @{release.parsed.uploader}
          </button>
        {:else}
          <span></span>
        {/if}

        <div class="flex items-center gap-1.5 shrink-0">
          {#if release.anilist_metadata?.average_score}
            <button
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                openDetailModal(release, 'metadata');
              }}
              class="px-2 py-0.5 rounded-full bg-emerald-500/85 text-white border border-emerald-300/40 backdrop-blur-md text-[10px] font-mono font-bold flex items-center gap-0.5 cursor-pointer shadow"
              title="View AniList metadata"
            >
              <Star class="w-2.5 h-2.5 fill-current" />
              <span>{release.anilist_metadata.average_score}%</span>
            </button>
          {:else if release.anilist_metadata}
            <button
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                openDetailModal(release, 'metadata');
              }}
              class="px-2 py-0.5 rounded-full bg-cyan-500/80 text-slate-950 border border-cyan-300/40 backdrop-blur-md text-[10px] font-mono font-bold cursor-pointer shadow"
            >
              AniList
            </button>
          {:else}
            <button
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                openDetailModal(release, 'metadata');
              }}
              class="px-2 py-0.5 rounded-full bg-slate-950/70 hover:bg-cyan-500/25 text-slate-300 hover:text-cyan-200 border border-white/15 backdrop-blur-md text-[10px] font-mono cursor-pointer transition-colors"
              title="Sync AniList metadata"
            >
              + Sync
            </button>
          {/if}
        </div>
      </div>

      <!-- Bottom Poster & Title Overlay -->
      <div class="relative z-10 h-full p-3.5 pt-10 flex items-end gap-3">
        {#if posterUrl}
          <div class="w-20 h-28 rounded-2xl overflow-hidden border border-white/25 shadow-xl shrink-0 bg-slate-900 transition-transform duration-300 group-hover:scale-[1.03]">
            <img
              src={posterUrl}
              alt={release.parsed.anime_title || release.title}
              class="w-full h-full object-cover"
              loading="lazy"
            />
          </div>
        {:else}
          <div class="w-16 h-24 rounded-2xl border border-white/15 bg-white/5 backdrop-blur-md flex flex-col items-center justify-center shrink-0 text-slate-400">
            <Compass class="w-6 h-6 opacity-70" />
            <span class="text-[9px] font-mono mt-1 opacity-75">NYAA</span>
          </div>
        {/if}

        <div class="min-w-0 flex-1 pb-0.5 space-y-1.5">
          <div class="flex items-center gap-1.5 flex-wrap">
            {#if release.parsed.episode}
              <span class="px-2 py-0.5 rounded-full bg-emerald-500/25 border border-emerald-400/40 text-[10px] font-mono font-bold text-emerald-200 backdrop-blur-md">
                EP {release.parsed.episode}
              </span>
            {/if}
            {#if release.parsed.resolution}
              <span class="px-2 py-0.5 rounded-full bg-cyan-500/25 border border-cyan-400/40 text-[10px] font-mono font-bold text-cyan-200 backdrop-blur-md">
                {release.parsed.resolution}
              </span>
            {/if}
            {#if release.anilist_metadata?.format}
              <span class="px-2 py-0.5 rounded-full bg-black/45 border border-white/15 text-[10px] font-mono text-slate-200 backdrop-blur-md">
                {release.anilist_metadata.format}
              </span>
            {/if}
          </div>

          <h2
            class="text-sm font-bold text-white leading-snug line-clamp-2 group-hover:text-cyan-300 transition-colors drop-shadow"
            title={release.anilist_metadata?.title_english || release.parsed.anime_title || release.title}
          >
            {release.anilist_metadata?.title_english || release.parsed.anime_title || release.title}
          </h2>
        </div>
      </div>
    </div>

    <!-- Card Body -->
    <div class="p-4 flex-1 flex flex-col justify-between gap-3">
      <div class="space-y-2.5">
        <!-- Raw Release Filename -->
        <p
          class="text-[11px] font-mono text-slate-600 dark:text-slate-300 line-clamp-2 leading-relaxed"
          title={release.title}
        >
          {release.title}
        </p>

        <!-- Technical Tags -->
        {#if release.parsed.codec || release.parsed.audio || release.parsed.crc32 || release.parsed.source}
          <div class="flex items-center gap-1.5 flex-wrap text-[10px] font-mono">
            {#if release.parsed.source}
              <span class="px-2 py-0.5 rounded-full bg-slate-200/80 dark:bg-white/6 border border-slate-300/70 dark:border-white/10 text-slate-700 dark:text-slate-300">
                {release.parsed.source}
              </span>
            {/if}
            {#if release.parsed.codec}
              <span class="px-2 py-0.5 rounded-full bg-slate-200/80 dark:bg-white/6 border border-slate-300/70 dark:border-white/10 text-slate-700 dark:text-slate-300">
                {release.parsed.codec}
              </span>
            {/if}
            {#if release.parsed.audio}
              <span class="px-2 py-0.5 rounded-full bg-slate-200/80 dark:bg-white/6 border border-slate-300/70 dark:border-white/10 text-slate-700 dark:text-slate-300">
                {release.parsed.audio}
              </span>
            {/if}
            {#if release.parsed.crc32}
              <span class="px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-500/25 text-cyan-700 dark:text-cyan-300">
                [{release.parsed.crc32}]
              </span>
            {/if}
          </div>
        {/if}
      </div>

      <div class="space-y-3">
        <!-- Metrics Strip: Size, Date, Seeders/Leechers -->
        <div class="rounded-2xl bg-slate-100/85 dark:bg-slate-950/60 border border-slate-200/80 dark:border-white/8 px-3 py-2 flex items-center justify-between gap-2 text-[11px] font-mono">
          <div class="flex items-center gap-3 min-w-0">
            <span class="inline-flex items-center gap-1 font-bold text-slate-800 dark:text-slate-200 whitespace-nowrap">
              <HardDrive class="w-3 h-3 text-cyan-600 dark:text-cyan-400" />
              {release.file_size_formatted}
            </span>
            <span
              class="inline-flex items-center gap-1 text-slate-500 dark:text-slate-400 whitespace-nowrap"
              title={formatFullDate(release.nyaa_created_at)}
            >
              <Clock class="w-3 h-3" />
              {formatDate(release.nyaa_created_at)}
            </span>
          </div>

          <div class="flex items-center gap-1.5 whitespace-nowrap shrink-0" title="Seeders / Leechers">
            <span class="text-emerald-600 dark:text-emerald-400 font-bold">▲ {release.seeders}</span>
            <span class="text-slate-400 dark:text-slate-600">/</span>
            <span class="text-rose-600 dark:text-rose-400 font-bold">▼ {release.leechers}</span>
          </div>
        </div>

        <!-- Card Actions -->
        <div class="flex items-center gap-1.5 font-mono">
          <button
            type="button"
            onclick={() => downloadRelease(release)}
            disabled={isDownloading}
            class="btn-expressive flex-1 px-3 py-2 rounded-2xl text-xs font-bold transition-all cursor-pointer flex items-center justify-center gap-1.5 {isDownloaded
              ? 'bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border border-emerald-500/40'
              : 'bg-gradient-to-r from-cyan-500 to-sky-500 hover:from-cyan-400 hover:to-sky-400 text-white shadow-[0_0_15px_rgba(0,229,255,0.22)]'}"
            title="Download torrent with librqbit engine"
          >
            {#if isDownloading}
              <span class="inline-block animate-spin w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Starting...</span>
            {:else if isDownloaded}
              <Check class="w-3.5 h-3.5" />
              <span>Added</span>
            {:else}
              <DownloadCloud class="w-3.5 h-3.5" />
              <span>Download</span>
            {/if}
          </button>

          <button
            type="button"
            onclick={() => copyMagnet(release)}
            class="btn-expressive p-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 hover:text-cyan-600 dark:hover:text-cyan-300 cursor-pointer"
            title="Copy Magnet URI"
            aria-label="Copy Magnet URI"
          >
            {#if isCopied}
              <Check class="w-4 h-4 text-emerald-500" />
            {:else}
              <Magnet class="w-4 h-4" />
            {/if}
          </button>

          <button
            type="button"
            onclick={() => openDetailModal(release, 'nyaa')}
            class="btn-expressive p-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 hover:text-cyan-600 dark:hover:text-cyan-300 cursor-pointer"
            title="Inspect release files & AniList details"
            aria-label="Inspect release details"
          >
            <Info class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  </article>
{/snippet}

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl w-full mx-auto space-y-5">
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
        <button
          type="button"
          onclick={toggleDateSort}
          disabled={loading}
          class="btn-expressive glass-pill px-3.5 py-2 rounded-2xl text-xs font-mono text-slate-700 dark:text-slate-200 cursor-pointer flex items-center gap-1.5"
          title="Server sort order by release date"
        >
          <ArrowUpDown class="w-3.5 h-3.5 text-cyan-600 dark:text-cyan-400" />
          <span>{sortOrder === 'desc' ? 'Newest First' : 'Oldest First'}</span>
        </button>

        <button
          type="button"
          onclick={() => loadReleases()}
          disabled={loading}
          class="btn-expressive glass-pill px-4 py-2 rounded-2xl text-xs font-mono text-slate-700 dark:text-white transition-all cursor-pointer flex items-center gap-2"
        >
          <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin text-cyan-500 dark:text-cyan-400' : ''}" />
          <span>Refresh</span>
        </button>
      </div>
    </div>

    <!-- Search & Filter Form -->
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
          <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            id="search"
            type="text"
            bind:value={searchQuery}
            placeholder="e.g. Frieren, Solo Leveling..."
            class="glass-input w-full rounded-2xl pl-9 pr-8 py-2.5"
          />
          {#if searchQuery}
            <button
              type="button"
              onclick={() => {
                searchQuery = '';
                goToPage(1);
              }}
              class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
              aria-label="Clear search"
            >
              ✕
            </button>
          {/if}
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

    <!-- Quick Release Group Chips & Active Filter Pills -->
    <div class="flex flex-wrap items-center justify-between gap-2.5 pt-1 border-t border-slate-200/70 dark:border-white/5 text-[11px] font-mono">
      <div class="flex items-center gap-1.5 flex-wrap">
        <span class="text-slate-500 dark:text-slate-400 mr-1">Groups:</span>
        {#each QUICK_UPLOADERS as grp}
          {@const isActiveGroup = userQuery.trim().toLowerCase() === grp.toLowerCase()}
          <button
            type="button"
            onclick={() => filterByUploader(grp)}
            class="px-2.5 py-1 rounded-full border transition-all cursor-pointer {isActiveGroup
              ? 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border-cyan-500/45 font-bold'
              : 'glass-pill text-slate-600 dark:text-slate-300 hover:text-cyan-700 dark:hover:text-cyan-300'}"
          >
            @{grp}
          </button>
        {/each}
      </div>

      {#if hasActiveFilters}
        <button
          type="button"
          onclick={resetAllFilters}
          class="inline-flex items-center gap-1 text-cyan-700 dark:text-cyan-300 hover:underline cursor-pointer"
        >
          <RotateCcw class="w-3 h-3" />
          <span>Reset all</span>
        </button>
      {/if}
    </div>
  </div>

  <!-- Toast Banner -->
  {#if toast}
    <div
      transition:fly={{ y: -8, duration: 180, easing: cubicOut }}
      class="glass-panel rounded-2xl px-4 py-3 flex items-center justify-between gap-3 text-xs font-mono border {toast.tone === 'success'
        ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
        : toast.tone === 'error'
          ? 'border-rose-500/40 bg-rose-500/10 text-rose-700 dark:text-rose-300'
          : 'border-cyan-500/40 bg-cyan-500/10 text-cyan-700 dark:text-cyan-300'}"
    >
      <div class="flex items-center gap-2">
        {#if toast.tone === 'success'}
          <CheckCircle2 class="w-4 h-4 shrink-0" />
        {:else if toast.tone === 'error'}
          <AlertCircle class="w-4 h-4 shrink-0" />
        {:else}
          <Info class="w-4 h-4 shrink-0" />
        {/if}
        <span>{toast.message}</span>
      </div>
      <button
        type="button"
        onclick={() => (toast = null)}
        class="p-1 rounded-lg hover:bg-black/5 dark:hover:bg-white/10 cursor-pointer"
        aria-label="Dismiss notification"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  {/if}

  <!-- Error Banner -->
  {#if error}
    <div class="glass-panel border-l-4 border-rose-500 p-4 rounded-2xl text-xs font-mono text-rose-700 dark:text-rose-300 bg-rose-500/10 dark:bg-rose-950/20 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <AlertCircle class="w-4 h-4 text-rose-500 dark:text-rose-400" />
        <span>Error loading releases: {error}</span>
      </div>
      <button onclick={() => (error = null)} class="text-rose-500 hover:text-rose-700 dark:text-rose-400 dark:hover:text-white cursor-pointer font-bold">✕</button>
    </div>
  {/if}

  <!-- Results Toolbar (Refinement, Sort, Top Pagination & View Switcher) -->
  <div
    bind:this={resultsTopRef}
    class="glass-panel rounded-3xl p-3.5 border border-slate-200/80 dark:border-white/10 flex flex-col lg:flex-row items-stretch lg:items-center justify-between gap-3"
  >
    <div class="flex flex-wrap items-center gap-2 text-xs font-mono">
      <!-- Quick In-Page Refinement -->
      {#each [
        { id: 'all', label: `All (${releases.length})` },
        { id: 'matched', label: `AniList Matched (${matchedCount})` },
        { id: '1080p', label: '1080p' }
      ] as pill (pill.id)}
        <button
          type="button"
          onclick={() => (quickFilter = pill.id as typeof quickFilter)}
          class="btn-expressive px-3 py-1.5 rounded-full text-xs font-semibold cursor-pointer transition-all {quickFilter === pill.id
            ? 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40'
            : 'text-slate-600 dark:text-slate-400 hover:bg-black/5 dark:hover:bg-white/5 border border-transparent'}"
        >
          {pill.label}
        </button>
      {/each}

      {#if userQuery.trim()}
        <span class="px-2.5 py-1 rounded-full bg-cyan-500/15 border border-cyan-500/35 text-cyan-700 dark:text-cyan-300 flex items-center gap-1.5">
          <span>@{userQuery.trim()}</span>
          <button type="button" onclick={clearUserFilter} class="hover:text-rose-500 cursor-pointer" aria-label="Clear uploader">
            <X class="w-3 h-3" />
          </button>
        </span>
      {/if}
    </div>

    <div class="flex flex-wrap items-center justify-between lg:justify-end gap-2 text-xs font-mono">
      <!-- In-Page Sort -->
      <select
        bind:value={pageSort}
        aria-label="Sort visible results"
        class="glass-input rounded-2xl px-3 py-1.5 text-xs font-mono cursor-pointer"
      >
        <option value="nyaa" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Sort: Nyaa Order</option>
        <option value="seeders" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Sort: Most Seeders</option>
        <option value="size" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Sort: Largest Size</option>
      </select>

      <!-- Compact Top Pagination -->
      <div class="flex items-center gap-1">
        <button
          type="button"
          onclick={() => goToPage(page - 1)}
          disabled={page <= 1 || loading}
          class="p-1.5 rounded-xl glass-pill disabled:opacity-40 cursor-pointer"
          title="Previous page"
          aria-label="Previous page"
        >
          <ChevronLeft class="w-4 h-4" />
        </button>
        <span class="px-2.5 py-1 rounded-xl bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30 font-bold">
          P.{page}
        </span>
        <button
          type="button"
          onclick={() => goToPage(page + 1)}
          disabled={loading || releases.length === 0}
          class="p-1.5 rounded-xl glass-pill disabled:opacity-40 cursor-pointer"
          title="Next page"
          aria-label="Next page"
        >
          <ChevronRight class="w-4 h-4" />
        </button>
      </div>

      <!-- Grid / Table Switcher -->
      <div class="flex items-center p-1 rounded-2xl bg-slate-200/70 dark:bg-slate-950/60 border border-slate-300/70 dark:border-white/10">
        <button
          type="button"
          onclick={() => (viewMode = 'grid')}
          class="p-1.5 rounded-xl transition-all cursor-pointer {viewMode === 'grid'
            ? 'bg-white dark:bg-slate-800 text-cyan-600 dark:text-cyan-300 shadow-sm'
            : 'text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          title="Card Grid View"
          aria-label="Card Grid View"
        >
          <LayoutGrid class="w-4 h-4" />
        </button>
        <button
          type="button"
          onclick={() => (viewMode = 'table')}
          class="p-1.5 rounded-xl transition-all cursor-pointer {viewMode === 'table'
            ? 'bg-white dark:bg-slate-800 text-cyan-600 dark:text-cyan-300 shadow-sm'
            : 'text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          title="Compact Table View"
          aria-label="Compact Table View"
        >
          <List class="w-4 h-4" />
        </button>
      </div>
    </div>
  </div>

  <!-- Loading Shimmer Bar on Page Change -->
  {#if loading && releases.length > 0}
    <div class="h-1 w-full rounded-full bg-gradient-to-r from-cyan-400 via-sky-400 to-indigo-500 animate-pulse"></div>
  {/if}

  <!-- Releases Content: Card Grid or Table -->
  {#if loading && releases.length === 0}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {#each Array(8) as _, idx (idx)}
        <div class="glass-panel rounded-3xl overflow-hidden border border-slate-200/80 dark:border-white/10 animate-pulse">
          <div class="h-48 bg-slate-200/80 dark:bg-slate-800/60"></div>
          <div class="p-4 space-y-3">
            <div class="h-3 bg-slate-200 dark:bg-slate-800 rounded-full w-4/5"></div>
            <div class="h-3 bg-slate-200 dark:bg-slate-800 rounded-full w-2/3"></div>
            <div class="h-9 bg-slate-200/70 dark:bg-slate-800/50 rounded-2xl"></div>
            <div class="h-9 bg-slate-200 dark:bg-slate-800 rounded-2xl"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if displayedReleases.length === 0}
    <div class="glass-panel rounded-3xl py-20 px-6 text-center border border-slate-200/80 dark:border-white/10 space-y-3">
      <Compass class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">
        No releases found
      </h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-md mx-auto font-mono">
        {releases.length === 0
          ? 'Try adjusting your Nyaa category, uploader, or search query.'
          : 'No items on this page match the selected quick filter.'}
      </p>
      <div class="pt-2">
        <button
          type="button"
          onclick={resetAllFilters}
          class="btn-expressive px-5 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-400 to-sky-400 text-slate-950 font-bold text-xs font-mono cursor-pointer"
        >
          Reset Filters
        </button>
      </div>
    </div>
  {:else if viewMode === 'grid'}
    <div
      class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 transition-opacity duration-200 {loading && isPageChanging
        ? 'opacity-45 pointer-events-none'
        : 'opacity-100'}"
    >
      {#each displayedReleases as release (release.id)}
        {@render releaseCard(release)}
      {/each}
    </div>
  {:else}
    <!-- Compact Table View -->
    <div class="glass-panel rounded-3xl overflow-hidden shadow-2xl border border-slate-200/80 dark:border-white/10">
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
            {#each displayedReleases as release (release.id)}
              {@const posterUrl = getPosterUrl(release)}
              <tr
                class="hover:bg-black/[0.02] dark:hover:bg-white/[0.04] transition-colors group cursor-pointer"
                onclick={() => openDetailModal(release, 'nyaa')}
              >
                <td class="py-3 px-4">
                  <button
                    type="button"
                    onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                    class="block w-10 h-14 rounded-xl overflow-hidden border border-slate-200 dark:border-white/10 group-hover:border-cyan-400/40 transition-all cursor-pointer bg-slate-100 dark:bg-slate-900/60"
                    title="View details for {release.parsed.anime_title}"
                  >
                    {#if posterUrl}
                      <img src={posterUrl} alt="Anime Poster" class="w-full h-full object-cover" loading="lazy" />
                    {:else}
                      <div class="w-full h-full flex items-center justify-center text-[10px] text-slate-500 dark:text-slate-400 font-mono">
                        N/A
                      </div>
                    {/if}
                  </button>
                </td>

                <td class="py-3 px-4 space-y-1.5">
                  <div class="flex items-center gap-2 flex-wrap">
                    {#if release.parsed.uploader}
                      <button
                        type="button"
                        onclick={(e) => { e.stopPropagation(); filterByUploader(release.parsed.uploader); }}
                        class="text-xs font-mono font-semibold px-2 py-0.5 rounded-full bg-cyan-500/10 text-cyan-700 dark:text-cyan-300 hover:bg-cyan-500/20 border border-cyan-500/25 transition-all cursor-pointer"
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
                      <span class="text-xs font-mono font-bold px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border border-emerald-500/30">
                        EP {release.parsed.episode}
                      </span>
                    {/if}
                  </div>

                  <div class="text-xs text-slate-500 dark:text-slate-400 line-clamp-1 font-mono">
                    {release.title}
                  </div>

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
                      <span class="px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-500/25 text-cyan-700 dark:text-cyan-300">
                        [{release.parsed.crc32}]
                      </span>
                    {/if}
                  </div>
                </td>

                <td class="py-3 px-4">
                  {#if release.anilist_metadata}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'metadata'); }}
                      class="text-left group/meta cursor-pointer"
                    >
                      <div class="text-xs font-semibold text-slate-900 dark:text-white group-hover/meta:text-cyan-600 dark:group-hover/meta:text-cyan-300 line-clamp-1 transition-colors">
                        {release.anilist_metadata.title_english || release.anilist_metadata.title_romaji}
                      </div>
                      <div class="flex items-center gap-2 text-[10px] font-mono text-slate-500 dark:text-slate-400 mt-1">
                        {#if release.anilist_metadata.average_score}
                          <span class="text-emerald-600 dark:text-emerald-300 font-bold">★ {release.anilist_metadata.average_score}%</span>
                        {/if}
                        <span class="px-1.5 py-0.5 rounded bg-slate-200/70 dark:bg-white/5 text-slate-700 dark:text-slate-300">
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
                    >
                      No match (Sync)
                    </button>
                  {/if}
                </td>

                <td class="py-3 px-4 text-xs font-mono text-slate-600 dark:text-slate-300 whitespace-nowrap" title={formatFullDate(release.nyaa_created_at)}>
                  {formatDate(release.nyaa_created_at)}
                </td>

                <td class="py-3 px-4 text-xs font-mono text-slate-600 dark:text-slate-300 whitespace-nowrap">
                  {release.file_size_formatted}
                </td>

                <td class="py-3 px-4 text-xs font-mono whitespace-nowrap">
                  <span class="text-emerald-600 dark:text-emerald-400 font-bold">{release.seeders}</span>
                  <span class="text-slate-400 dark:text-slate-500"> / </span>
                  <span class="text-rose-600 dark:text-rose-400 font-bold">{release.leechers}</span>
                </td>

                <td class="py-3 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                      class="btn-expressive px-2.5 py-1 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-slate-200 hover:text-cyan-600 dark:hover:text-cyan-300 cursor-pointer"
                    >
                      Detail
                    </button>
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); downloadRelease(release); }}
                      disabled={downloadingIds.has(release.id)}
                      class="btn-expressive px-3 py-1 rounded-xl text-xs font-mono font-semibold cursor-pointer flex items-center gap-1.5 {downloadedSuccessIds.has(release.id)
                        ? 'bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border border-emerald-500/40'
                        : 'bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40'}"
                    >
                      {#if downloadingIds.has(release.id)}
                        <span class="inline-block animate-spin w-3 h-3 border border-cyan-400 border-t-transparent rounded-full"></span>
                        <span>Starting...</span>
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
                      class="btn-expressive px-2.5 py-1 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-slate-300 cursor-pointer"
                    >
                      {copiedMagnetId === release.id ? '✓ Copied' : 'Magnet'}
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

  <!-- Bottom Pagination Deck -->
  <div class="glass-panel rounded-3xl px-5 py-4 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs font-mono border border-slate-200/80 dark:border-white/10">
    <div class="flex items-center gap-3 flex-wrap">
      <span class="text-slate-600 dark:text-slate-400">
        Page <strong class="text-cyan-600 dark:text-cyan-300 font-bold">{page}</strong>
        {#if displayedReleases.length > 0}
          <span class="text-slate-400 dark:text-slate-500 font-normal">
            ({displayedReleases.length}{displayedReleases.length !== releases.length ? ` of ${releases.length}` : ''} results)
          </span>
        {/if}
      </span>

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

      <span class="px-3.5 py-1.5 rounded-xl bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/35 font-bold shadow-[0_0_12px_rgba(0,229,255,0.2)]">
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

<!-- Floating "Back to Top" Action Button -->
{#if scrollY > 400}
  <div class="fixed bottom-6 right-6 z-40">
    <button
      type="button"
      onclick={scrollToTop}
      class="glass-panel-elevated px-3.5 py-2.5 rounded-full text-slate-700 dark:text-slate-200 hover:text-cyan-600 dark:hover:text-cyan-300 hover:border-cyan-400/40 shadow-2xl hover:shadow-[0_0_20px_rgba(0,229,255,0.3)] transition-all cursor-pointer flex items-center gap-2 btn-expressive group border border-slate-200/90 dark:border-white/15"
      title="Scroll back to top (Home key)"
      aria-label="Back to top"
    >
      <ArrowUp class="w-4 h-4 group-hover:-translate-y-0.5 transition-transform text-cyan-600 dark:text-cyan-400" />
      <span class="text-xs font-mono font-semibold hidden sm:inline">Top</span>
      <span class="text-[10px] font-mono px-1.5 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/25 font-bold">
        P.{page}
      </span>
    </button>
  </div>
{/if}

<!-- Scraped Item Detail Modal (Wide Split-Pane Glass Modal) -->
{#if selectedRelease}
  {@const modalBackdrop = getBackdropUrl(selectedRelease)}
  {@const modalPoster = getPosterUrl(selectedRelease)}
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    class="fixed inset-0 bg-slate-950/80 backdrop-blur-xl z-[60] flex items-center justify-center p-2.5 sm:p-4 lg:p-6 overflow-y-auto"
    tabindex="-1"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') closeModal(); }}
  >
    <div
      transition:scale={{ duration: 240, start: 0.95, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl max-w-6xl w-full my-auto shadow-2xl overflow-hidden flex flex-col max-h-[92vh] border border-slate-200/80 dark:border-white/15"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-release-title"
      tabindex="-1"
    >
      <!-- Ambient Hero Header -->
      <div class="relative overflow-hidden border-b border-slate-200/80 dark:border-white/10 bg-slate-100/80 dark:bg-slate-950/60 shrink-0">
        {#if modalBackdrop}
          <img
            src={modalBackdrop}
            alt=""
            aria-hidden="true"
            class="absolute inset-0 w-full h-full object-cover opacity-20 dark:opacity-25 blur-2xl scale-110 pointer-events-none"
          />
          <div class="absolute inset-0 bg-gradient-to-r from-white/80 via-white/60 to-white/80 dark:from-slate-950/90 dark:via-slate-950/75 dark:to-slate-950/90 pointer-events-none"></div>
        {/if}

        <div class="relative z-10 p-4 sm:p-5 lg:px-6 flex items-start justify-between gap-4">
          <div class="flex items-start gap-3.5 sm:gap-4 min-w-0 flex-1">
            {#if modalPoster}
              <img
                src={modalPoster}
                alt={selectedRelease.parsed.anime_title || selectedRelease.title}
                class="hidden sm:block w-14 h-20 object-cover rounded-2xl border border-slate-300/60 dark:border-white/20 shrink-0 shadow-lg bg-slate-900"
              />
            {/if}

            <div class="space-y-1.5 min-w-0 flex-1">
              <div class="flex items-center gap-1.5 flex-wrap">
                <span class="px-2.5 py-0.5 rounded-full bg-cyan-500/15 border border-cyan-500/30 text-[10px] font-mono font-semibold text-cyan-700 dark:text-cyan-300">
                  Nyaa #{selectedRelease.id}
                </span>
                <span class="px-2.5 py-0.5 rounded-full bg-slate-200/80 dark:bg-white/8 border border-slate-300/60 dark:border-white/10 text-[10px] font-mono text-slate-700 dark:text-slate-300">
                  {selectedRelease.category}
                </span>
                {#if selectedRelease.parsed.uploader || selectedRelease.uploader}
                  <button
                    type="button"
                    onclick={() => {
                      const up = selectedRelease!.parsed.uploader || selectedRelease!.uploader;
                      closeModal();
                      filterByUploader(up);
                    }}
                    class="px-2.5 py-0.5 rounded-full bg-purple-500/15 hover:bg-purple-500/25 border border-purple-500/30 text-[10px] font-mono text-purple-700 dark:text-purple-300 cursor-pointer transition-colors"
                    title="Filter releases by this uploader"
                  >
                    @{selectedRelease.parsed.uploader || selectedRelease.uploader}
                  </button>
                {/if}
                {#if selectedRelease.parsed.episode}
                  <span class="px-2.5 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/30 text-[10px] font-mono font-bold text-emerald-700 dark:text-emerald-300">
                    EP {selectedRelease.parsed.episode}
                  </span>
                {/if}
                {#if selectedRelease.parsed.resolution}
                  <span class="px-2.5 py-0.5 rounded-full bg-sky-500/15 border border-sky-500/30 text-[10px] font-mono font-bold text-sky-700 dark:text-sky-300">
                    {selectedRelease.parsed.resolution}
                  </span>
                {/if}
              </div>

              <h2 id="modal-release-title" class="text-base sm:text-xl font-bold text-slate-900 dark:text-white tracking-tight line-clamp-1">
                {selectedRelease.anilist_metadata?.title_english || selectedRelease.parsed.anime_title || selectedRelease.title}
              </h2>

              <p class="text-[11px] sm:text-xs text-slate-600 dark:text-slate-400 font-mono line-clamp-1" title={selectedRelease.title}>
                {selectedRelease.title}
              </p>
            </div>
          </div>

          <!-- Header Prev/Next & Close Controls -->
          <div class="flex items-center gap-1.5 shrink-0">
            {#if selectedIndex >= 0 && displayedReleases.length > 1}
              <div class="hidden sm:flex items-center gap-1 mr-1 px-1.5 py-1 rounded-full glass-pill text-[11px] font-mono">
                <button
                  type="button"
                  onclick={() => openAdjacentRelease(-1)}
                  disabled={selectedIndex <= 0}
                  class="p-1 rounded-full hover:bg-black/5 dark:hover:bg-white/10 disabled:opacity-30 cursor-pointer"
                  title="Previous item in results"
                  aria-label="Previous release"
                >
                  <ChevronLeft class="w-3.5 h-3.5" />
                </button>
                <span class="px-1 text-slate-600 dark:text-slate-300">
                  {selectedIndex + 1}/{displayedReleases.length}
                </span>
                <button
                  type="button"
                  onclick={() => openAdjacentRelease(1)}
                  disabled={selectedIndex >= displayedReleases.length - 1}
                  class="p-1 rounded-full hover:bg-black/5 dark:hover:bg-white/10 disabled:opacity-30 cursor-pointer"
                  title="Next item in results"
                  aria-label="Next release"
                >
                  <ChevronRight class="w-3.5 h-3.5" />
                </button>
              </div>
            {/if}

            <button
              type="button"
              onclick={closeModal}
              class="text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white p-2 rounded-full glass-pill transition-colors font-mono cursor-pointer btn-expressive"
              title="Close modal (Esc)"
              aria-label="Close modal"
            >
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Tab Navigation -->
        <div class="relative z-10 flex items-center justify-between border-t border-slate-200/70 dark:border-white/5 bg-slate-100/60 dark:bg-slate-950/50 px-4 sm:px-6 pt-2 text-xs font-mono">
          <div class="flex gap-2">
            <button
              type="button"
              onclick={() => (activeTab = 'nyaa')}
              class="py-2.5 px-4 font-semibold rounded-t-2xl border-b-2 transition-all cursor-pointer flex items-center gap-2 {activeTab === 'nyaa'
                ? 'border-cyan-500 dark:border-cyan-400 text-cyan-700 dark:text-cyan-300 bg-cyan-500/10'
                : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
            >
              <span>Release Details</span>
              <span class="text-[10px] px-2 py-0.5 rounded-full bg-slate-200 dark:bg-slate-900 border border-slate-300/60 dark:border-white/10 text-slate-700 dark:text-slate-300">
                {releaseDetail?.file_size_formatted || selectedRelease.file_size_formatted}
              </span>
            </button>

            <button
              type="button"
              onclick={() => (activeTab = 'metadata')}
              class="py-2.5 px-4 font-semibold rounded-t-2xl border-b-2 transition-all cursor-pointer flex items-center gap-2 {activeTab === 'metadata'
                ? 'border-cyan-500 dark:border-cyan-400 text-cyan-700 dark:text-cyan-300 bg-cyan-500/10'
                : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
            >
              <span>AniList Metadata</span>
              {#if selectedRelease.anilist_metadata}
                <span class="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border border-emerald-500/30 font-bold">
                  ✓ Synced
                </span>
              {:else}
                <span class="text-[10px] px-2 py-0.5 rounded-full bg-slate-200/70 dark:bg-white/5 text-slate-600 dark:text-slate-400 border border-slate-300/60 dark:border-white/10">
                  Unsynced
                </span>
              {/if}
            </button>
          </div>

          <span class="hidden md:inline text-[11px] text-slate-500 dark:text-slate-400 pb-2">
            {formatFullDate(selectedRelease.nyaa_created_at)}
          </span>
        </div>
      </div>

      <!-- Tab Content Area -->
      <div class="p-4 sm:p-6 overflow-y-auto flex-1 space-y-5">
        {#if activeTab === 'nyaa'}
          {#snippet renderFileItem(item: NyaaFileItem, depth: number)}
            <div
              style="padding-left: {depth * 18}px"
              class="py-1.5 flex items-center justify-between text-xs font-mono border-b border-slate-200/60 dark:border-white/5 last:border-0 hover:bg-black/[0.02] dark:hover:bg-white/[0.04] rounded-xl px-2.5 transition-colors"
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

          <!-- Scrape Status Banner -->
          {#if detailLoading}
            <div class="glass-panel border border-cyan-500/40 rounded-2xl px-4 py-3 flex items-center justify-between text-xs font-mono text-cyan-700 dark:text-cyan-300 shadow-[0_0_20px_rgba(0,229,255,0.12)]">
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 border-2 border-cyan-500 dark:border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
                <span>Scraping live release data from <strong class="text-slate-900 dark:text-white">nyaa.si/view/{selectedRelease.id}</strong>...</span>
              </div>
              <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">Live Scrape</span>
            </div>
          {:else if detailError}
            <div class="glass-panel border border-rose-500/40 rounded-2xl px-4 py-3 flex items-center justify-between text-xs font-mono text-rose-700 dark:text-rose-300 bg-rose-500/10 dark:bg-rose-950/20">
              <span>⚠️ Failed to scrape detail: {detailError}</span>
              <button
                type="button"
                onclick={() => selectedRelease && fetchReleaseDetail(selectedRelease.id)}
                class="btn-expressive px-3 py-1 rounded-xl glass-pill text-slate-700 dark:text-white transition-colors cursor-pointer text-xs"
              >
                Retry
              </button>
            </div>
          {/if}

          <!-- Full Release Title Bar -->
          <div class="glass-panel rounded-2xl p-3.5 border border-slate-200/70 dark:border-white/8 flex flex-col sm:flex-row sm:items-center justify-between gap-2.5 text-xs font-mono">
            <div class="min-w-0 flex-1">
              <span class="text-[10px] uppercase font-bold text-slate-500 dark:text-slate-400 block mb-1">Original Release Title</span>
              <div class="text-slate-900 dark:text-slate-100 select-all break-all">
                {releaseDetail?.title || selectedRelease.title}
              </div>
            </div>
            <button
              type="button"
              onclick={copyDetailTitle}
              class="btn-expressive px-3 py-1.5 rounded-xl glass-pill text-cyan-700 dark:text-cyan-300 text-[11px] flex items-center gap-1.5 shrink-0 self-start sm:self-center cursor-pointer"
            >
              {#if detailCopiedTitle}
                <Check class="w-3.5 h-3.5 text-emerald-500" />
                <span>Copied</span>
              {:else}
                <Copy class="w-3.5 h-3.5" />
                <span>Copy Title</span>
              {/if}
            </button>
          </div>

          <!-- Wide 2-Column Split Pane -->
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-5 items-start">
            <!-- Left Column: Swarm, Technical Specs, Hash & Magnet -->
            <div class="lg:col-span-5 space-y-4">
              <!-- Swarm & Metrics 2x2 Grid -->
              <div class="grid grid-cols-2 gap-3 text-xs font-mono">
                <div class="glass-panel rounded-2xl p-3.5 border border-slate-200/60 dark:border-white/5">
                  <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">File Size</span>
                  <span class="text-base font-bold text-slate-900 dark:text-white mt-1 block">
                    {releaseDetail?.file_size_formatted || selectedRelease.file_size_formatted}
                  </span>
                  <span class="text-[10px] text-slate-500 dark:text-slate-400 block truncate">
                    {(releaseDetail?.file_size_bytes || selectedRelease.file_size_bytes).toLocaleString()} B
                  </span>
                </div>

                <div class="glass-panel rounded-2xl p-3.5 border border-slate-200/60 dark:border-white/5">
                  <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Completed</span>
                  <span class="text-base font-bold text-cyan-600 dark:text-cyan-400 mt-1 block">
                    ✓ {releaseDetail?.completed_count ?? selectedRelease.completed_count}
                  </span>
                  <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Total snatches</span>
                </div>

                <div class="glass-panel rounded-2xl p-3.5 border border-slate-200/60 dark:border-white/5">
                  <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Seeders</span>
                  <span class="text-base font-bold text-emerald-600 dark:text-emerald-400 mt-1 block">
                    ▲ {releaseDetail?.seeders ?? selectedRelease.seeders}
                  </span>
                  <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Active uploaders</span>
                </div>

                <div class="glass-panel rounded-2xl p-3.5 border border-slate-200/60 dark:border-white/5">
                  <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Leechers</span>
                  <span class="text-base font-bold text-rose-600 dark:text-rose-400 mt-1 block">
                    ▼ {releaseDetail?.leechers ?? selectedRelease.leechers}
                  </span>
                  <span class="text-[10px] text-slate-500 dark:text-slate-400 block">Downloading</span>
                </div>
              </div>

              <!-- Technical Specifications -->
              <div class="glass-panel rounded-2xl p-4 space-y-2.5 border border-slate-200/60 dark:border-white/5 text-xs font-mono">
                <h3 class="text-[11px] uppercase text-slate-500 dark:text-slate-400 font-bold tracking-wider border-b border-slate-200/60 dark:border-white/5 pb-2">
                  Parsed Technical Specs
                </h3>
                <div class="space-y-2">
                  <div class="flex items-center justify-between gap-2 border-b border-slate-200/50 dark:border-white/5 pb-1.5">
                    <span class="text-slate-500 dark:text-slate-400 shrink-0">Anime Title</span>
                    <span class="text-slate-900 dark:text-white font-semibold truncate text-right" title={releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title}>
                      {releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title || 'N/A'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2 border-b border-slate-200/50 dark:border-white/5 pb-1.5">
                    <span class="text-slate-500 dark:text-slate-400">Episode</span>
                    <span class="text-emerald-600 dark:text-emerald-400 font-bold">
                      {(releaseDetail?.parsed?.episode || selectedRelease.parsed.episode) ? `EP ${releaseDetail?.parsed?.episode || selectedRelease.parsed.episode}` : 'Batch / Movie'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2 border-b border-slate-200/50 dark:border-white/5 pb-1.5">
                    <span class="text-slate-500 dark:text-slate-400">Resolution</span>
                    <span class="text-slate-800 dark:text-slate-200">
                      {releaseDetail?.parsed?.resolution || selectedRelease.parsed.resolution || 'N/A'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2 border-b border-slate-200/50 dark:border-white/5 pb-1.5">
                    <span class="text-slate-500 dark:text-slate-400">Video / Audio</span>
                    <span class="text-slate-800 dark:text-slate-200 text-right">
                      {releaseDetail?.parsed?.codec || selectedRelease.parsed.codec || 'N/A'} · {releaseDetail?.parsed?.audio || selectedRelease.parsed.audio || 'N/A'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-slate-500 dark:text-slate-400">Checksum (CRC32)</span>
                    <span class="text-cyan-700 dark:text-cyan-300">
                      {(releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32) ? `[${releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32}]` : 'N/A'}
                    </span>
                  </div>
                </div>
              </div>

              <!-- Info Hash & Submitter Information -->
              {#if releaseDetail?.info_hash}
                <div class="glass-panel rounded-2xl p-4 space-y-2 border border-slate-200/60 dark:border-white/5 text-xs font-mono">
                  <div class="flex items-center justify-between text-[10px] uppercase text-slate-500 dark:text-slate-400">
                    <span>Torrent Info Hash (SHA-1)</span>
                    <button
                      type="button"
                      onclick={copyDetailHash}
                      class="text-cyan-600 dark:text-cyan-400 hover:underline cursor-pointer flex items-center gap-1"
                    >
                      <Copy class="w-3 h-3" />
                      <span>{detailCopiedHash ? 'Copied' : 'Copy'}</span>
                    </button>
                  </div>
                  <div class="bg-slate-100 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5 text-cyan-700 dark:text-cyan-300 select-all break-all tracking-wider">
                    {releaseDetail.info_hash}
                  </div>
                </div>
              {/if}

              {#if releaseDetail?.information}
                <div class="glass-panel rounded-2xl p-4 space-y-2 border border-slate-200/60 dark:border-white/5 text-xs font-mono">
                  <span class="text-[10px] uppercase text-slate-500 dark:text-slate-400 block">Submitter Information</span>
                  <div class="bg-slate-100 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5 text-slate-800 dark:text-slate-200 truncate">
                    {#if releaseDetail.information.startsWith('http')}
                      <a
                        href={releaseDetail.information}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-cyan-600 dark:text-cyan-400 hover:underline flex items-center gap-1.5 truncate"
                      >
                        <span class="truncate">{releaseDetail.information}</span>
                        <ExternalLink class="w-3.5 h-3.5 shrink-0" />
                      </a>
                    {:else}
                      {releaseDetail.information}
                    {/if}
                  </div>
                </div>
              {/if}

              <!-- Magnet URI -->
              <div class="glass-panel rounded-2xl p-4 space-y-2 border border-slate-200/60 dark:border-white/5 text-xs font-mono">
                <div class="flex items-center justify-between text-[10px] uppercase text-slate-500 dark:text-slate-400">
                  <span>Magnet URI</span>
                  <button
                    type="button"
                    onclick={copyDetailMagnet}
                    class="text-cyan-600 dark:text-cyan-400 hover:underline cursor-pointer flex items-center gap-1"
                  >
                    <Copy class="w-3 h-3" />
                    <span>{detailCopiedMagnet ? 'Copied' : 'Copy'}</span>
                  </button>
                </div>
                <div class="text-[11px] text-slate-700 dark:text-slate-300 select-all break-all bg-slate-100 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5 max-h-20 overflow-y-auto">
                  {releaseDetail?.magnet_uri || selectedRelease.magnet_uri}
                </div>
              </div>
            </div>

            <!-- Right Column: Torrent File List & Release Notes -->
            <div class="lg:col-span-7 space-y-4">
              {#if detailLoading}
                <div class="glass-panel rounded-2xl p-5 space-y-3 border border-slate-200/60 dark:border-white/5 animate-pulse">
                  <div class="h-3 w-32 bg-slate-200 dark:bg-slate-800 rounded-full"></div>
                  <div class="space-y-2">
                    <div class="h-8 bg-slate-200/70 dark:bg-slate-800/50 rounded-xl"></div>
                    <div class="h-8 bg-slate-200/70 dark:bg-slate-800/50 rounded-xl"></div>
                    <div class="h-8 bg-slate-200/70 dark:bg-slate-800/50 rounded-xl"></div>
                  </div>
                </div>
              {/if}

              {#if releaseDetail?.file_list && releaseDetail.file_list.length > 0}
                <div class="glass-panel rounded-2xl p-4 sm:p-5 space-y-3 border border-slate-200/60 dark:border-white/5">
                  <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-2 text-xs font-mono">
                    <div class="flex items-center gap-2">
                      <span class="uppercase text-slate-700 dark:text-slate-300 tracking-wider font-bold">Torrent Files</span>
                      <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">
                        {releaseDetail.file_list.length} {releaseDetail.file_list.length === 1 ? 'item' : 'items'}
                      </span>
                    </div>
                    <span class="text-[10px] text-slate-500 dark:text-slate-400">nyaa.si/view/{selectedRelease.id}</span>
                  </div>
                  <div class="max-h-72 overflow-y-auto space-y-0.5 pr-1 bg-slate-100/80 dark:bg-slate-950/60 p-2.5 rounded-xl border border-slate-200/60 dark:border-white/5">
                    {#each releaseDetail.file_list as item}
                      {@render renderFileItem(item, 0)}
                    {/each}
                  </div>
                </div>
              {/if}

              {#if releaseDetail?.description_markdown}
                <div class="glass-panel rounded-2xl p-4 sm:p-5 space-y-3 border border-slate-200/60 dark:border-white/5">
                  <div class="flex items-center justify-between border-b border-slate-200/60 dark:border-white/5 pb-2 text-xs font-mono">
                    <div class="flex items-center gap-2">
                      <span class="uppercase text-slate-700 dark:text-slate-300 tracking-wider font-bold">Release Notes</span>
                      <span class="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border border-cyan-500/30">
                        Markdown
                      </span>
                    </div>
                    <div class="flex items-center gap-1 bg-slate-200/60 dark:bg-slate-950/60 border border-slate-300/40 dark:border-white/5 rounded-xl p-0.5">
                      <button
                        type="button"
                        onclick={() => (descViewMode = 'markdown')}
                        class="px-2.5 py-1 rounded-lg text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'markdown' ? 'bg-cyan-500 dark:bg-cyan-400 text-white dark:text-slate-950 font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
                      >
                        Formatted
                      </button>
                      <button
                        type="button"
                        onclick={() => (descViewMode = 'raw')}
                        class="px-2.5 py-1 rounded-lg text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'raw' ? 'bg-cyan-500 dark:bg-cyan-400 text-white dark:text-slate-950 font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
                      >
                        Raw
                      </button>
                    </div>
                  </div>

                  {#if descViewMode === 'markdown'}
                    <div class="max-h-[26rem] overflow-y-auto pr-3 bg-slate-50/80 dark:bg-slate-950/60 p-4 rounded-xl border border-slate-200/80 dark:border-white/5 prose-nyaa break-words">
                      {@html renderMarkdown(releaseDetail.description_markdown)}
                    </div>
                  {:else}
                    <div class="max-h-[26rem] overflow-y-auto pr-2 bg-slate-50/80 dark:bg-slate-950/60 p-3 rounded-xl border border-slate-200/80 dark:border-white/5 text-xs font-mono text-slate-700 dark:text-slate-300 leading-relaxed whitespace-pre-wrap break-words select-all">
                      {releaseDetail.description_markdown}
                    </div>
                  {/if}
                </div>
              {:else if !detailLoading && (!releaseDetail?.file_list || releaseDetail.file_list.length === 0)}
                <div class="glass-panel rounded-2xl p-8 text-center border border-slate-200/60 dark:border-white/5 text-xs font-mono text-slate-500 dark:text-slate-400">
                  No additional release notes or file tree reported for this torrent.
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <!-- TAB 2: ANILIST METADATA (Wide 2-Column Layout) -->
          <div class="space-y-5">
            {#if selectedRelease.anilist_metadata}
              {#if selectedRelease.anilist_metadata.banner_image_url}
                <div class="w-full h-40 sm:h-48 rounded-2xl overflow-hidden border border-slate-200/60 dark:border-white/10 relative bg-slate-900 shadow-lg">
                  <img
                    src={selectedRelease.anilist_metadata.banner_image_url}
                    alt="Banner"
                    class="w-full h-full object-cover opacity-70"
                  />
                  <div class="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/35 to-transparent"></div>
                </div>
              {/if}

              <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
                <!-- Left Poster & Quick Stats -->
                <div class="lg:col-span-4 flex flex-row lg:flex-col gap-4 items-start">
                  {#if selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                    <img
                      src={selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                      alt={selectedRelease.anilist_metadata.title_romaji}
                      class="w-28 sm:w-36 lg:w-full max-w-[220px] aspect-[2/3] object-cover rounded-2xl border border-slate-200/80 dark:border-white/15 shrink-0 bg-slate-900 shadow-2xl"
                    />
                  {/if}

                  <div class="space-y-3 flex-1 min-w-0 w-full">
                    <div class="flex items-center gap-2 flex-wrap text-xs font-mono">
                      {#if selectedRelease.anilist_metadata.average_score}
                        <span class="px-2.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-500/30 font-bold">
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
                          {selectedRelease.anilist_metadata.episodes} Eps
                        </span>
                      {/if}
                    </div>

                    <div class="flex items-center gap-3 text-xs font-mono flex-wrap">
                      <a
                        href="https://anilist.co/anime/{selectedRelease.anilist_metadata.id}"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-cyan-600 dark:text-cyan-400 hover:underline flex items-center gap-1 font-semibold"
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
                          class="text-indigo-600 dark:text-indigo-400 hover:underline flex items-center gap-1 font-semibold"
                        >
                          <span>MAL #{selectedRelease.anilist_metadata.mal_id}</span>
                          <ExternalLink class="w-3.5 h-3.5" />
                        </a>
                      {/if}
                    </div>
                  </div>
                </div>

                <!-- Right Titles, Genres, Synopsis & Sync -->
                <div class="lg:col-span-8 space-y-4 min-w-0">
                  <div class="space-y-1.5">
                    <h3 class="text-xl sm:text-2xl font-bold text-slate-900 dark:text-white tracking-tight">
                      {selectedRelease.anilist_metadata.title_english || selectedRelease.anilist_metadata.title_romaji}
                    </h3>
                    <div class="text-xs font-mono text-cyan-700 dark:text-cyan-300">
                      {selectedRelease.anilist_metadata.title_romaji}
                    </div>
                    {#if selectedRelease.anilist_metadata.title_native}
                      <div class="text-xs text-slate-500 dark:text-slate-400">
                        {selectedRelease.anilist_metadata.title_native}
                      </div>
                    {/if}

                    {#if selectedRelease.anilist_metadata.genres && selectedRelease.anilist_metadata.genres.length > 0}
                      <div class="flex items-center gap-1.5 flex-wrap pt-2">
                        {#each selectedRelease.anilist_metadata.genres as genre}
                          <span class="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-slate-200/80 dark:bg-slate-900/60 text-slate-700 dark:text-slate-300 border border-slate-300/60 dark:border-white/5">
                            {genre}
                          </span>
                        {/each}
                      </div>
                    {/if}
                  </div>

                  {#if selectedRelease.anilist_metadata.description}
                    <div class="glass-panel rounded-2xl p-5 space-y-2 border border-slate-200/60 dark:border-white/5">
                      <h4 class="text-xs font-mono uppercase text-slate-500 dark:text-slate-400 font-bold">Synopsis</h4>
                      <div class="text-xs sm:text-sm text-slate-700 dark:text-slate-300 leading-relaxed max-h-60 overflow-y-auto pr-2 space-y-1">
                        {@html selectedRelease.anilist_metadata.description}
                      </div>
                    </div>
                  {/if}
                </div>
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
            <div class="glass-panel rounded-2xl p-5 space-y-3 border border-slate-200/60 dark:border-white/5">
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
                <div class="p-3 rounded-xl bg-rose-500/10 border border-rose-500/40 text-xs font-mono text-rose-700 dark:text-rose-300">
                  {syncError}
                </div>
              {/if}

              {#if syncSuccess}
                <div class="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/40 text-xs font-mono text-emerald-700 dark:text-emerald-300">
                  {syncSuccess}
                </div>
              {/if}

              {#if !auth.isAuthenticated}
                <div class="p-3.5 rounded-2xl glass-pill flex items-center justify-between text-xs font-mono">
                  <span class="text-slate-600 dark:text-slate-400">Sign in to manually search & sync AniList metadata for this title.</span>
                  <a href="/login?redirect=/explore" class="text-cyan-600 dark:text-cyan-400 hover:text-cyan-700 dark:hover:text-cyan-300 font-bold">Sign In &rarr;</a>
                </div>
              {:else}
                <form
                  onsubmit={(e) => {
                    e.preventDefault();
                    handleManualSync();
                  }}
                  class="flex flex-col sm:flex-row gap-2"
                >
                  <input
                    type="text"
                    bind:value={manualSyncTitle}
                    placeholder="Enter anime title (e.g. Frieren: Beyond Journey's End)"
                    class="glass-input flex-1 rounded-2xl px-3.5 py-2.5 text-xs font-mono"
                  />
                  <button
                    type="submit"
                    disabled={isSyncingMetadata || !manualSyncTitle.trim()}
                    class="btn-expressive bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 font-bold text-xs font-mono px-4 py-2.5 rounded-2xl transition-all cursor-pointer disabled:opacity-40 flex items-center justify-center gap-1.5 shrink-0 shadow-[0_0_15px_rgba(0,229,255,0.2)]"
                  >
                    {#if isSyncingMetadata}
                      <span class="inline-block animate-spin w-3 h-3 border-2 border-slate-950 border-t-transparent rounded-full"></span>
                      <span>Syncing...</span>
                    {:else}
                      <Sparkles class="w-3.5 h-3.5 stroke-[2.5]" />
                      <span>Sync AniList</span>
                    {/if}
                  </button>
                </form>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- Sticky Modal Action Footer (Always accessible across both tabs) -->
      <div class="px-4 sm:px-6 py-3.5 border-t border-slate-200/80 dark:border-white/10 bg-slate-100/85 dark:bg-slate-950/75 backdrop-blur-md flex items-center justify-between gap-3 flex-wrap shrink-0">
        <div class="flex items-center gap-2">
          <a
            href="https://nyaa.si/view/{selectedRelease.id}"
            target="_blank"
            rel="noopener noreferrer"
            class="btn-expressive px-3.5 py-2 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-white hover:text-cyan-600 dark:hover:text-cyan-300 flex items-center gap-1.5"
          >
            <span>Open Nyaa.si</span>
            <ExternalLink class="w-3.5 h-3.5" />
          </a>

          {#if selectedIndex >= 0 && displayedReleases.length > 1}
            <div class="flex sm:hidden items-center gap-1 px-1.5 py-1 rounded-xl glass-pill text-[11px] font-mono">
              <button
                type="button"
                onclick={() => openAdjacentRelease(-1)}
                disabled={selectedIndex <= 0}
                class="p-1 disabled:opacity-30 cursor-pointer"
                aria-label="Previous release"
              >
                <ChevronLeft class="w-3.5 h-3.5" />
              </button>
              <span>{selectedIndex + 1}/{displayedReleases.length}</span>
              <button
                type="button"
                onclick={() => openAdjacentRelease(1)}
                disabled={selectedIndex >= displayedReleases.length - 1}
                class="p-1 disabled:opacity-30 cursor-pointer"
                aria-label="Next release"
              >
                <ChevronRight class="w-3.5 h-3.5" />
              </button>
            </div>
          {/if}
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            onclick={() => copyDetailMagnet()}
            class="btn-expressive px-4 py-2 rounded-xl text-xs font-mono glass-pill text-slate-700 dark:text-white cursor-pointer flex items-center gap-1.5"
          >
            {#if detailCopiedMagnet}
              <Check class="w-3.5 h-3.5 text-emerald-500" />
              <span>Magnet Copied</span>
            {:else}
              <Magnet class="w-3.5 h-3.5" />
              <span>Copy Magnet</span>
            {/if}
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
                    file_size_formatted: releaseDetail.file_size_formatted
                  }
                : selectedRelease;
              if (target) downloadRelease(target);
            }}
            disabled={downloadingIds.has(selectedRelease.id)}
            class="btn-expressive px-5 py-2 rounded-xl text-xs font-mono font-bold transition-all cursor-pointer flex items-center gap-2 {downloadedSuccessIds.has(selectedRelease.id)
              ? 'bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border border-emerald-500/40'
              : 'bg-gradient-to-r from-cyan-400 to-sky-400 hover:from-cyan-300 hover:to-sky-300 text-slate-950 shadow-[0_0_20px_rgba(0,229,255,0.25)]'}"
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
  </div>
{/if}
