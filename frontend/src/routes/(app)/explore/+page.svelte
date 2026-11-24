<script lang="ts">
  import { api, getWebSocketUrl } from '#lib/api/client';
  import type { NyaaRelease, NyaaReleaseDetail, NyaaFileItem, AnimeMetadata } from '#lib/api/types';
  import { onMount } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';

  let releases = $state<NyaaRelease[]>([]);
  let loading = $state<boolean>(true);
  let error = $state<string | null>(null);

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

  function handleFilterSubmit(e: Event) {
    e.preventDefault();
    page = 1;
    loadReleases();
  }

  function toggleDateSort() {
    sortOrder = sortOrder === 'desc' ? 'asc' : 'desc';
    page = 1;
    loadReleases();
  }

  function filterByUploader(uploaderName?: string) {
    if (!uploaderName) return;
    userQuery = uploaderName;
    page = 1;
    loadReleases();
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
    page = 1;
    loadReleases();
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

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>Nyaa Explorer & AniList Sync - Nyaalist</title>
</svelte:head>

<div class="flex-1 p-6 max-w-7xl w-full mx-auto space-y-6">
  <!-- Top Bar & Control Panel -->
  <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-5 shadow-xl space-y-4">
    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-[#2C2C2E] pb-4">
      <div>
        <h1 class="text-xl font-bold font-mono tracking-tight text-white flex items-center gap-2.5">
          <span class="w-3 h-3 rounded-full bg-[#00E5FF] animate-pulse"></span>
          Nyaa Torrent Explorer
        </h1>
        <p class="text-xs text-[#98989D] mt-0.5">
          Live feed from nyaa.si with automatic title heuristics and AniList metadata synchronization.
        </p>
      </div>

      <div class="flex items-center gap-2">
        {#if userQuery.trim().length > 0}
          <div class="flex items-center gap-2 bg-[#121212] border border-[#00E5FF]/40 rounded-xl px-3 py-1.5 text-xs font-mono">
            <span class="text-[#98989D]">Uploader:</span>
            <span class="text-[#00E5FF] font-bold">@{userQuery}</span>
            <button
              onclick={clearUserFilter}
              class="text-gray-400 hover:text-[#FF453A] ml-1 cursor-pointer font-bold"
              title="Clear uploader filter"
            >
              ✕
            </button>
          </div>
        {/if}
        {#if searchQuery.trim().length > 0}
          <div class="flex items-center gap-2 bg-[#121212] border border-[#2C2C2E] rounded-xl px-3 py-1.5 text-xs font-mono">
            <span class="text-[#98989D]">Query:</span>
            <span class="text-white font-bold">"{searchQuery.trim()}"</span>
            <button
              onclick={() => { searchQuery = ''; page = 1; loadReleases(); }}
              class="text-gray-400 hover:text-[#FF453A] ml-1 cursor-pointer font-bold"
              title="Clear search query"
            >
              ✕
            </button>
          </div>
        {/if}

        <button
          onclick={() => loadReleases()}
          disabled={loading}
          class="bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white px-3.5 py-1.5 rounded-xl text-xs font-mono transition-colors cursor-pointer flex items-center gap-1.5"
        >
          <span class={loading ? 'animate-spin' : ''}>↻</span>
          Refresh
        </button>
      </div>
    </div>

    <!-- Filters Form -->
    <form onsubmit={handleFilterSubmit} class="grid grid-cols-1 md:grid-cols-12 gap-3 text-xs font-mono">
      <div class="md:col-span-3 space-y-1">
        <label for="category" class="text-[10px] uppercase text-[#98989D]">Category</label>
        <select
          id="category"
          bind:value={category}
          disabled={userQuery.trim().length > 0}
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-3 py-2 text-white focus:outline-none focus:border-[#00E5FF] disabled:opacity-40"
        >
          <option value="1_2">Anime - English-translated (1_2)</option>
          <option value="1_0">All Anime (1_0)</option>
          <option value="1_4">Anime - Raw (1_4)</option>
          <option value="1_3">Anime - Non-English (1_3)</option>
        </select>
      </div>

      <div class="md:col-span-3 space-y-1">
        <label for="user" class="text-[10px] uppercase text-[#98989D]">Uploader (e.g. Erai-raws)</label>
        <input
          id="user"
          type="text"
          bind:value={userQuery}
          placeholder="Erai-raws / SubsPlease"
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-3 py-2 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF]"
        />
      </div>

      <div class="md:col-span-4 space-y-1">
        <label for="search" class="text-[10px] uppercase text-[#98989D]">Title Search Query</label>
        <input
          id="search"
          type="text"
          bind:value={searchQuery}
          placeholder="e.g. Dragon Ball Super, Kozame..."
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-3 py-2 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF]"
        />
      </div>

      <div class="md:col-span-2 flex items-end">
        <button
          type="submit"
          disabled={loading}
          class="w-full bg-[#00E5FF] hover:bg-[#00c8e0] text-[#121212] font-bold py-2 px-4 rounded-xl shadow transition-all cursor-pointer disabled:opacity-50"
        >
          Filter
        </button>
      </div>
    </form>
  </div>

  <!-- Status / Alert Banners -->
  {#if error}
    <div class="bg-[#3A1C1C] border-l-4 border-[#FF453A] p-4 rounded-r-xl text-sm font-mono text-[#FF453A]">
      Error loading releases: {error}
    </div>
  {/if}

  <!-- Releases Data Table -->
  <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl overflow-hidden shadow-2xl">
    <div class="overflow-x-auto">
      <table class="w-full text-left text-sm">
        <thead class="bg-[#121212] border-b border-[#2C2C2E] text-[11px] font-mono text-[#98989D] uppercase tracking-wider">
          <tr>
            <th class="py-3 px-4 w-14">Cover</th>
            <th class="py-3 px-4">Release Title & Technical Tags</th>
            <th class="py-3 px-4 w-44">AniList Match</th>
            <th
              class="py-3 px-4 w-32 cursor-pointer select-none hover:text-white transition-colors"
              onclick={toggleDateSort}
              title="Click to sort by date ({sortOrder === 'desc' ? 'Newest first' : 'Oldest first'})"
            >
              <div class="flex items-center gap-1.5">
                <span>Date</span>
                {#if sortOrder === 'desc'}
                  <span class="text-[#00E5FF] font-bold text-xs" title="Newest first">↓</span>
                {:else}
                  <span class="text-[#00E5FF] font-bold text-xs" title="Oldest first">↑</span>
                {/if}
              </div>
            </th>
            <th class="py-3 px-4 w-24">Size</th>
            <th class="py-3 px-4 w-24">S / L</th>
            <th class="py-3 px-4 w-40 text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#2C2C2E]">
          {#if loading && releases.length === 0}
            <tr>
              <td colspan="7" class="py-16 text-center text-[#98989D] font-mono text-xs">
                <div class="inline-block w-5 h-5 border-2 border-[#00E5FF] border-t-transparent rounded-full animate-spin mb-2"></div>
                <div>Scraping Nyaa.si and querying AniList GraphQL API...</div>
              </td>
            </tr>
          {:else if releases.length === 0}
            <tr>
              <td colspan="7" class="py-12 text-center text-[#98989D] font-mono text-xs">
                No releases found matching your search criteria.
              </td>
            </tr>
          {:else}
            {#each releases as release (release.id)}
              <tr
                class="hover:bg-[#252525] transition-colors group cursor-pointer"
                onclick={() => openDetailModal(release, 'nyaa')}
              >
                <!-- Cover Image Thumbnail -->
                <td class="py-3 px-4">
                  <button
                    type="button"
                    onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                    class="block w-10 h-14 rounded overflow-hidden border border-[#2C2C2E] group-hover:border-[#00E5FF] transition-colors cursor-pointer bg-[#121212]"
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
                      <div class="w-full h-full flex items-center justify-center text-[10px] text-gray-500 font-mono">
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
                        class="text-xs font-mono font-semibold px-2 py-0.5 rounded bg-[#121212] text-[#00E5FF] hover:bg-[#00E5FF]/20 border border-[#2C2C2E] transition-colors cursor-pointer"
                        title="Filter by uploader @{release.parsed.uploader}"
                      >
                        [{release.parsed.uploader}]
                      </button>
                    {/if}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                      class="text-sm font-semibold text-white group-hover:text-[#00E5FF] transition-colors text-left cursor-pointer"
                    >
                      {release.parsed.anime_title}
                    </button>
                    {#if release.parsed.episode}
                      <span class="text-xs font-mono font-bold px-1.5 py-0.2 rounded bg-[#121212] text-[#32D74B] border border-[#2C2C2E]">
                        EP {release.parsed.episode}
                      </span>
                    {/if}
                  </div>

                  <div class="text-xs text-[#98989D] line-clamp-1 font-mono">
                    {release.title}
                  </div>

                  <!-- Technical Badges -->
                  <div class="flex items-center gap-1.5 flex-wrap text-[10px] font-mono">
                    {#if release.parsed.resolution}
                      <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-gray-300">
                        {release.parsed.resolution}
                      </span>
                    {/if}
                    {#if release.parsed.codec}
                      <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-gray-300">
                        {release.parsed.codec}
                      </span>
                    {/if}
                    {#if release.parsed.audio}
                      <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-gray-300">
                        {release.parsed.audio}
                      </span>
                    {/if}
                    {#if release.parsed.crc32}
                      <span class="px-1.5 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[#00E5FF]">
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
                      <div class="text-xs font-semibold text-white group-hover/meta:text-[#00E5FF] line-clamp-1 transition-colors">
                        {release.anilist_metadata.title_english || release.anilist_metadata.title_romaji}
                      </div>
                      <div class="flex items-center gap-2 text-[10px] font-mono text-[#98989D] mt-0.5">
                        {#if release.anilist_metadata.average_score}
                          <span class="text-[#32D74B] font-bold">{release.anilist_metadata.average_score}%</span>
                        {/if}
                        <span>{release.anilist_metadata.format || 'TV'}</span>
                        {#if release.anilist_metadata.episodes}
                          <span>({release.anilist_metadata.episodes} eps)</span>
                        {/if}
                      </div>
                    </button>
                  {:else}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'metadata'); }}
                      class="text-xs font-mono text-gray-500 hover:text-[#00E5FF] italic transition-colors cursor-pointer text-left"
                      title="Click to view or sync metadata"
                    >
                      No match (Sync)
                    </button>
                  {/if}
                </td>

                <!-- Date -->
                <td class="py-3 px-4 text-xs font-mono text-gray-300 whitespace-nowrap" title={formatFullDate(release.nyaa_created_at)}>
                  {formatDate(release.nyaa_created_at)}
                </td>

                <!-- File Size -->
                <td class="py-3 px-4 text-xs font-mono text-gray-300 whitespace-nowrap">
                  {release.file_size_formatted}
                </td>

                <!-- Seeds / Peers -->
                <td class="py-3 px-4 text-xs font-mono whitespace-nowrap">
                  <span class="text-[#32D74B] font-bold">{release.seeders}</span>
                  <span class="text-gray-500"> / </span>
                  <span class="text-[#FF453A] font-bold">{release.leechers}</span>
                </td>

                <!-- Action Buttons -->
                <td class="py-3 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); openDetailModal(release, 'nyaa'); }}
                      class="px-2 py-1 rounded-lg text-xs font-mono bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white hover:text-[#00E5FF] transition-colors cursor-pointer"
                      title="View release detail modal"
                    >
                      Detail
                    </button>
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); downloadRelease(release); }}
                      disabled={downloadingIds.has(release.id)}
                      class="px-2.5 py-1 rounded-lg text-xs font-mono transition-all cursor-pointer flex items-center gap-1.5 {
                        downloadedSuccessIds.has(release.id)
                          ? 'bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/40 shadow-[0_0_8px_rgba(50,215,75,0.2)]'
                          : 'bg-[#00E5FF]/20 hover:bg-[#00E5FF]/30 text-[#00E5FF] border border-[#00E5FF]/40 shadow-[0_0_8px_rgba(0,229,255,0.2)]'
                      }"
                      title="Download torrent with librqbit engine"
                    >
                      {#if downloadingIds.has(release.id)}
                        <span class="inline-block animate-spin w-2.5 h-2.5 border border-[#00E5FF] border-t-transparent rounded-full"></span>
                        Starting...
                      {:else if downloadedSuccessIds.has(release.id)}
                        ✓ Added
                      {:else}
                        ↓ Download
                      {/if}
                    </button>
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); copyMagnet(release); }}
                      class="px-2.5 py-1 rounded-lg text-xs font-mono bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white transition-colors cursor-pointer"
                      title="Copy Magnet Link"
                    >
                      {copiedMagnetId === release.id ? 'Copied!' : 'Magnet'}
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div class="bg-[#121212] border-t border-[#2C2C2E] px-4 py-3 flex items-center justify-between text-xs font-mono">
      <span class="text-[#98989D]">
        Page <span class="text-white font-bold">{page}</span>
      </span>
      <div class="flex items-center gap-2">
        <button
          onclick={() => { if (page > 1) { page -= 1; loadReleases(); } }}
          disabled={page <= 1 || loading}
          class="px-3 py-1 bg-[#1E1E1E] border border-[#2C2C2E] rounded text-white disabled:opacity-40 cursor-pointer"
        >
          Prev
        </button>
        <button
          onclick={() => { page += 1; loadReleases(); }}
          disabled={loading || releases.length === 0}
          class="px-3 py-1 bg-[#1E1E1E] border border-[#2C2C2E] rounded text-white disabled:opacity-40 cursor-pointer"
        >
          Next
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Scraped Item Detail Modal (2 Tabs: Detail from Nyaa & Metadata) -->
{#if selectedRelease}
  <div
    class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-3 md:p-6 overflow-y-auto"
    tabindex="-1"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') closeModal(); }}
  >
    <div
      class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl max-w-3xl w-full my-auto shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-release-title"
      tabindex="-1"
    >
      <!-- Modal Header -->
      <div class="p-5 border-b border-[#2C2C2E] bg-[#161616] flex items-start justify-between gap-4">
        <div class="space-y-1 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="px-2 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-[#00E5FF]">
              Nyaa #{selectedRelease.id}
            </span>
            <span class="px-2 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-[#98989D]">
              {selectedRelease.category}
            </span>
            {#if selectedRelease.parsed.uploader || selectedRelease.uploader}
              <span class="px-2 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono text-purple-400">
                @{selectedRelease.parsed.uploader || selectedRelease.uploader}
              </span>
            {/if}
            {#if selectedRelease.parsed.episode}
              <span class="px-2 py-0.5 rounded bg-[#121212] border border-[#2C2C2E] text-[10px] font-mono font-bold text-[#32D74B]">
                EP {selectedRelease.parsed.episode}
              </span>
            {/if}
          </div>
          <h2 id="modal-release-title" class="text-lg font-bold text-white tracking-tight truncate">
            {selectedRelease.parsed.anime_title || selectedRelease.title}
          </h2>
          <p class="text-xs text-[#98989D] font-mono line-clamp-1" title={selectedRelease.title}>
            {selectedRelease.title}
          </p>
        </div>

        <button
          onclick={closeModal}
          class="text-gray-400 hover:text-white p-1.5 rounded-lg bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] transition-colors font-mono text-sm cursor-pointer shrink-0"
          title="Close modal (Esc)"
        >
          ✕
        </button>
      </div>

      <!-- Tab Navigation (2 Tabs: Detail from Nyaa & Metadata) -->
      <div class="flex border-b border-[#2C2C2E] bg-[#121212] px-5 text-xs font-mono">
        <button
          type="button"
          onclick={() => activeTab = 'nyaa'}
          class="py-3 px-4 font-semibold border-b-2 transition-all cursor-pointer flex items-center gap-2 {
            activeTab === 'nyaa'
              ? 'border-[#00E5FF] text-[#00E5FF] bg-[#1E1E1E]/60'
              : 'border-transparent text-[#98989D] hover:text-white hover:bg-[#1E1E1E]/30'
          }"
        >
          <span>Detail from Nyaa</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded bg-[#1E1E1E] text-gray-400 border border-[#2C2C2E]">
            {selectedRelease.file_size_formatted}
          </span>
        </button>

        <button
          type="button"
          onclick={() => activeTab = 'metadata'}
          class="py-3 px-4 font-semibold border-b-2 transition-all cursor-pointer flex items-center gap-2 {
            activeTab === 'metadata'
              ? 'border-[#00E5FF] text-[#00E5FF] bg-[#1E1E1E]/60'
              : 'border-transparent text-[#98989D] hover:text-white hover:bg-[#1E1E1E]/30'
          }"
        >
          <span>Metadata</span>
          {#if selectedRelease.anilist_metadata}
            <span class="text-[10px] px-1.5 py-0.2 rounded bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/30 font-bold">
              ✓ Synced
            </span>
          {:else}
            <span class="text-[10px] px-1.5 py-0.2 rounded bg-gray-800 text-gray-400 border border-[#2C2C2E]">
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
              style="padding-left: {depth * 16}px"
              class="py-1.5 flex items-center justify-between text-xs font-mono border-b border-[#2C2C2E]/30 last:border-0 hover:bg-[#1A1A1A] rounded px-2 transition-colors"
            >
              <div class="flex items-center gap-2 min-w-0">
                {#if item.is_folder}
                  <span class="text-[#00E5FF] text-sm shrink-0">📁</span>
                  <span class="font-bold text-white truncate">{item.name}</span>
                {:else}
                  <span class="text-gray-400 text-sm shrink-0">📄</span>
                  <span class="text-gray-200 truncate">{item.name}</span>
                {/if}
              </div>
              {#if item.size_formatted}
                <span class="text-[10px] text-gray-400 bg-[#121212] px-2 py-0.5 rounded border border-[#2C2C2E] shrink-0 ml-2">
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
              <div class="bg-[#121212] border border-[#00E5FF]/40 rounded-xl p-3.5 flex items-center justify-between text-xs font-mono text-[#00E5FF] shadow-[0_0_15px_rgba(0,229,255,0.1)]">
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 border-2 border-[#00E5FF] border-t-transparent rounded-full animate-spin"></div>
                  <span>Scraping detailed release data from <strong class="text-white">nyaa.si/view/{selectedRelease.id}</strong>...</span>
                </div>
                <span class="text-[10px] px-2 py-0.5 rounded bg-[#00E5FF]/10 text-[#00E5FF] border border-[#00E5FF]/30">Live Scrape</span>
              </div>
            {:else if detailError}
              <div class="bg-[#3A1C1C] border border-[#FF453A]/40 rounded-xl p-3.5 flex items-center justify-between text-xs font-mono text-[#FF453A]">
                <div class="flex items-center gap-2">
                  <span>⚠️ Failed to scrap detail: {detailError}</span>
                </div>
                <button
                  type="button"
                  onclick={() => selectedRelease && fetchReleaseDetail(selectedRelease.id)}
                  class="px-2.5 py-1 rounded bg-[#1E1E1E] hover:bg-[#2A2A2E] text-white border border-[#2C2C2E] transition-colors cursor-pointer text-[10px]"
                >
                  ↻ Retry Scrape
                </button>
              </div>
            {/if}

            <!-- Full Title Card -->
            <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-2">
              <div class="flex items-center justify-between text-[11px] font-mono text-[#98989D] uppercase">
                <span>Original Release Title</span>
                <button
                  type="button"
                  onclick={copyDetailTitle}
                  class="text-[#00E5FF] hover:underline cursor-pointer flex items-center gap-1 font-mono text-[10px]"
                >
                  {detailCopiedTitle ? '✓ Copied' : 'Copy Full Title'}
                </button>
              </div>
              <div class="font-mono text-xs text-gray-200 select-all break-all bg-[#1E1E1E] p-2.5 rounded-lg border border-[#2C2C2E]">
                {releaseDetail?.title || selectedRelease.title}
              </div>
            </div>

            <!-- Swarm & Metrics Cards -->
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs font-mono">
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3 text-center">
                <span class="text-[10px] uppercase text-[#98989D] block">File Size</span>
                <span class="text-sm font-bold text-white mt-1 block">
                  {releaseDetail?.file_size_formatted || selectedRelease.file_size_formatted}
                </span>
                <span class="text-[10px] text-gray-500 block truncate" title="{(releaseDetail?.file_size_bytes || selectedRelease.file_size_bytes)} bytes">
                  {(releaseDetail?.file_size_bytes || selectedRelease.file_size_bytes).toLocaleString()} B
                </span>
              </div>

              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3 text-center">
                <span class="text-[10px] uppercase text-[#98989D] block">Seeders</span>
                <span class="text-sm font-bold text-[#32D74B] mt-1 block">
                  ▲ {releaseDetail?.seeders ?? selectedRelease.seeders}
                </span>
                <span class="text-[10px] text-gray-500 block">Active uploaders</span>
              </div>

              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3 text-center">
                <span class="text-[10px] uppercase text-[#98989D] block">Leechers</span>
                <span class="text-sm font-bold text-[#FF453A] mt-1 block">
                  ▼ {releaseDetail?.leechers ?? selectedRelease.leechers}
                </span>
                <span class="text-[10px] text-gray-500 block">Downloading</span>
              </div>

              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3 text-center">
                <span class="text-[10px] uppercase text-[#98989D] block">Completed</span>
                <span class="text-sm font-bold text-[#00E5FF] mt-1 block">
                  ✓ {releaseDetail?.completed_count ?? selectedRelease.completed_count}
                </span>
                <span class="text-[10px] text-gray-500 block">Total snatches</span>
              </div>
            </div>

            <!-- Info Hash & Submitter Info (from scraped view) -->
            {#if releaseDetail?.info_hash || releaseDetail?.information}
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs font-mono">
                {#if releaseDetail?.info_hash}
                  <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3.5 space-y-2">
                    <div class="flex items-center justify-between text-[10px] uppercase text-[#98989D]">
                      <span>Torrent Info Hash (SHA-1)</span>
                      <button
                        type="button"
                        onclick={copyDetailHash}
                        class="text-[#00E5FF] hover:underline cursor-pointer flex items-center gap-1 text-[10px]"
                      >
                        {detailCopiedHash ? '✓ Copied' : 'Copy Hash'}
                      </button>
                    </div>
                    <div class="bg-[#1E1E1E] p-2 rounded-lg border border-[#2C2C2E] text-[#00E5FF] font-mono text-xs select-all break-all tracking-wider">
                      {releaseDetail.info_hash}
                    </div>
                  </div>
                {/if}

                {#if releaseDetail?.information}
                  <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-3.5 space-y-2">
                    <span class="text-[10px] uppercase text-[#98989D] block">Submitter Information</span>
                    <div class="bg-[#1E1E1E] p-2 rounded-lg border border-[#2C2C2E] text-gray-200 font-mono text-xs truncate">
                      {#if releaseDetail.information.startsWith('http')}
                        <a
                          href={releaseDetail.information}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="text-[#00E5FF] hover:underline flex items-center gap-1 truncate"
                        >
                          {releaseDetail.information} ↗
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
            <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-3">
              <h3 class="text-xs font-mono uppercase text-[#98989D] tracking-wider border-b border-[#2C2C2E] pb-2">
                Parsed Technical Specifications
              </h3>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-2.5 text-xs font-mono">
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Anime Title:</span>
                  <span class="text-white font-semibold truncate max-w-[200px]" title={releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title}>
                    {releaseDetail?.parsed?.anime_title || selectedRelease.parsed.anime_title || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Episode:</span>
                  <span class="text-[#32D74B] font-bold">
                    {(releaseDetail?.parsed?.episode || selectedRelease.parsed.episode) ? `EP ${releaseDetail?.parsed?.episode || selectedRelease.parsed.episode}` : 'N/A (Batch/Movie)'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Resolution:</span>
                  <span class="text-gray-200">
                    {releaseDetail?.parsed?.resolution || selectedRelease.parsed.resolution || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Video Codec:</span>
                  <span class="text-gray-200">
                    {releaseDetail?.parsed?.codec || selectedRelease.parsed.codec || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Audio Codec:</span>
                  <span class="text-gray-200">
                    {releaseDetail?.parsed?.audio || selectedRelease.parsed.audio || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Source:</span>
                  <span class="text-gray-200">
                    {releaseDetail?.parsed?.source || selectedRelease.parsed.source || 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Checksum (CRC32):</span>
                  <span class="text-[#00E5FF]">
                    {(releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32) ? `[${releaseDetail?.parsed?.crc32 || selectedRelease.parsed.crc32}]` : 'N/A'}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Uploader:</span>
                  {#if releaseDetail?.uploader_url}
                    <a
                      href="https://nyaa.si{releaseDetail.uploader_url}"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="text-purple-400 hover:text-[#00E5FF] transition-colors flex items-center gap-1 font-semibold"
                      title="View Nyaa user profile"
                    >
                      @{releaseDetail.uploader} ↗
                    </a>
                  {:else}
                    <span class="text-purple-400">
                      {releaseDetail?.uploader || selectedRelease.parsed.uploader || selectedRelease.uploader || 'Anonymous'}
                    </span>
                  {/if}
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Date Uploaded:</span>
                  <span class="text-gray-300" title={formatFullDate(releaseDetail?.nyaa_created_at || selectedRelease.nyaa_created_at)}>
                    {formatDate(releaseDetail?.nyaa_created_at || selectedRelease.nyaa_created_at)}
                  </span>
                </div>
                <div class="flex items-center justify-between border-b border-[#2C2C2E]/40 pb-1.5">
                  <span class="text-[#98989D]">Category:</span>
                  <span class="text-gray-300">
                    {releaseDetail?.category || selectedRelease.category}
                  </span>
                </div>
              </div>
            </div>

            <!-- Scraped Torrent File List -->
            {#if releaseDetail?.file_list && releaseDetail.file_list.length > 0}
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-3">
                <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-2 text-xs font-mono">
                  <div class="flex items-center gap-2">
                    <span class="uppercase text-[#98989D] tracking-wider font-bold">Torrent Files</span>
                    <span class="text-[10px] px-2 py-0.5 rounded-full bg-[#1E1E1E] text-[#00E5FF] border border-[#2C2C2E]">
                      {releaseDetail.file_list.length} {releaseDetail.file_list.length === 1 ? 'item' : 'items'}
                    </span>
                  </div>
                  <span class="text-[10px] text-gray-500">Scraped from nyaa.si/view/{selectedRelease.id}</span>
                </div>
                <div class="max-h-60 overflow-y-auto space-y-0.5 pr-1 bg-[#161616] p-2 rounded-lg border border-[#2C2C2E]">
                  {#each releaseDetail.file_list as item}
                    {@render renderFileItem(item, 0)}
                  {/each}
                </div>
              </div>
            {:else if detailLoading}
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 text-xs font-mono text-gray-500 flex items-center gap-2">
                <div class="w-3.5 h-3.5 border-2 border-[#00E5FF] border-t-transparent rounded-full animate-spin"></div>
                <span>Loading torrent file structure...</span>
              </div>
            {/if}

            <!-- Scraped Description & Release Notes (Markdown Formatted) -->
            {#if releaseDetail?.description_markdown}
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-3">
                <div class="flex items-center justify-between border-b border-[#2C2C2E] pb-2 text-xs font-mono">
                  <div class="flex items-center gap-2">
                    <span class="uppercase text-[#98989D] tracking-wider font-bold">Release Notes & Description</span>
                    <span class="text-[10px] px-2 py-0.5 rounded-full bg-[#1E1E1E] text-[#00E5FF] border border-[#2C2C2E]">
                      Markdown
                    </span>
                  </div>
                  <div class="flex items-center gap-1.5 bg-[#1E1E1E] border border-[#2C2C2E] rounded-lg p-0.5">
                    <button
                      type="button"
                      onclick={() => descViewMode = 'markdown'}
                      class="px-2.5 py-1 rounded text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'markdown' ? 'bg-[#00E5FF] text-[#121212] font-bold' : 'text-gray-400 hover:text-white'}"
                    >
                      Formatted
                    </button>
                    <button
                      type="button"
                      onclick={() => descViewMode = 'raw'}
                      class="px-2.5 py-1 rounded text-[10px] font-mono transition-colors cursor-pointer {descViewMode === 'raw' ? 'bg-[#00E5FF] text-[#121212] font-bold' : 'text-gray-400 hover:text-white'}"
                    >
                      Raw
                    </button>
                  </div>
                </div>

                {#if descViewMode === 'markdown'}
                  <div class="max-h-96 overflow-y-auto pr-3 bg-[#161616] p-4 rounded-lg border border-[#2C2C2E] prose-nyaa break-words">
                    {@html renderMarkdown(releaseDetail.description_markdown)}
                  </div>
                {:else}
                  <div class="max-h-96 overflow-y-auto pr-2 bg-[#161616] p-3 rounded-lg border border-[#2C2C2E] text-xs font-mono text-gray-300 leading-relaxed whitespace-pre-wrap break-words select-all">
                    {releaseDetail.description_markdown}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Magnet URI Details -->
            <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-2">
              <div class="flex items-center justify-between text-[11px] font-mono text-[#98989D] uppercase">
                <span>Magnet URI</span>
                <button
                  type="button"
                  onclick={copyDetailMagnet}
                  class="text-[#00E5FF] hover:underline cursor-pointer flex items-center gap-1 font-mono text-[10px]"
                >
                  {detailCopiedMagnet ? '✓ Copied' : 'Copy Magnet URI'}
                </button>
              </div>
              <div class="font-mono text-[11px] text-gray-400 select-all break-all bg-[#1E1E1E] p-2.5 rounded-lg border border-[#2C2C2E] max-h-20 overflow-y-auto">
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
                  class="px-3.5 py-2 rounded-xl text-xs font-mono bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white hover:text-[#00E5FF] transition-colors cursor-pointer flex items-center gap-1.5"
                >
                  <span>Nyaa.si</span>
                  <span class="text-[10px]">↗</span>
                </a>
                {#if releaseDetail?.torrent_file_url || selectedRelease.torrent_file_url}
                  <a
                    href="https://nyaa.si{releaseDetail?.torrent_file_url || selectedRelease.torrent_file_url}"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="px-3.5 py-2 rounded-xl text-xs font-mono bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white hover:text-[#00E5FF] transition-colors cursor-pointer flex items-center gap-1.5"
                  >
                    <span>.torrent</span>
                    <span class="text-[10px]">↓</span>
                  </a>
                {/if}
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onclick={() => copyDetailMagnet()}
                  class="px-3.5 py-2 rounded-xl text-xs font-mono bg-[#121212] hover:bg-[#252525] border border-[#2C2C2E] text-white transition-colors cursor-pointer"
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
                  class="px-4 py-2 rounded-xl text-xs font-mono font-bold transition-all cursor-pointer flex items-center gap-1.5 {
                    downloadedSuccessIds.has(selectedRelease.id)
                      ? 'bg-[#32D74B]/20 text-[#32D74B] border border-[#32D74B]/40'
                      : 'bg-[#00E5FF] hover:bg-[#00c8e0] text-[#121212]'
                  }"
                >
                  {#if downloadingIds.has(selectedRelease.id)}
                    <span class="inline-block animate-spin w-3 h-3 border-2 border-[#121212] border-t-transparent rounded-full"></span>
                    Starting Download...
                  {:else if downloadedSuccessIds.has(selectedRelease.id)}
                    ✓ Added to Downloader
                  {:else}
                    ↓ Start Download
                  {/if}
                </button>
              </div>
            </div>
          </div>
        {:else}
          <!-- TAB 2: METADATA -->
          <div class="space-y-5">
            {#if selectedRelease.anilist_metadata}
              <!-- Hero Section with Poster and Info -->
              <div class="space-y-4">
                {#if selectedRelease.anilist_metadata.banner_image_url}
                  <div class="w-full h-32 rounded-xl overflow-hidden border border-[#2C2C2E] relative bg-[#121212]">
                    <img
                      src={selectedRelease.anilist_metadata.banner_image_url}
                      alt="Banner"
                      class="w-full h-full object-cover opacity-70"
                    />
                    <div class="absolute inset-0 bg-gradient-to-t from-[#1E1E1E] to-transparent"></div>
                  </div>
                {/if}

                <div class="flex flex-col sm:flex-row gap-5 items-start">
                  {#if selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                    <img
                      src={selectedRelease.anilist_metadata.cover_image_local_path || selectedRelease.anilist_metadata.cover_image_url}
                      alt={selectedRelease.anilist_metadata.title_romaji}
                      class="w-32 h-44 object-cover rounded-xl border border-[#2C2C2E] shrink-0 bg-[#121212] shadow-lg"
                    />
                  {/if}

                  <div class="space-y-2 flex-1 min-w-0">
                    <h3 class="text-xl font-bold text-white tracking-tight">
                      {selectedRelease.anilist_metadata.title_english || selectedRelease.anilist_metadata.title_romaji}
                    </h3>
                    <div class="text-xs font-mono text-[#00E5FF]">
                      {selectedRelease.anilist_metadata.title_romaji}
                    </div>
                    {#if selectedRelease.anilist_metadata.title_native}
                      <div class="text-xs text-[#98989D]">
                        {selectedRelease.anilist_metadata.title_native}
                      </div>
                    {/if}

                    <div class="flex items-center gap-2 flex-wrap text-xs font-mono pt-1">
                      {#if selectedRelease.anilist_metadata.average_score}
                        <span class="px-2 py-0.5 rounded bg-[#121212] text-[#32D74B] border border-[#2C2C2E] font-bold">
                          ★ {selectedRelease.anilist_metadata.average_score}%
                        </span>
                      {/if}
                      <span class="px-2 py-0.5 rounded bg-[#121212] text-gray-300 border border-[#2C2C2E]">
                        {selectedRelease.anilist_metadata.format || 'TV'}
                      </span>
                      <span class="px-2 py-0.5 rounded bg-[#121212] text-gray-300 border border-[#2C2C2E]">
                        {selectedRelease.anilist_metadata.status || 'RELEASING'}
                      </span>
                      {#if selectedRelease.anilist_metadata.episodes}
                        <span class="px-2 py-0.5 rounded bg-[#121212] text-[#98989D] border border-[#2C2C2E]">
                          {selectedRelease.anilist_metadata.episodes} Episodes
                        </span>
                      {/if}
                      {#if selectedRelease.anilist_metadata.season || selectedRelease.anilist_metadata.season_year}
                        <span class="px-2 py-0.5 rounded bg-[#121212] text-gray-400 border border-[#2C2C2E]">
                          {selectedRelease.anilist_metadata.season || ''} {selectedRelease.anilist_metadata.season_year || ''}
                        </span>
                      {/if}
                    </div>

                    <!-- External Links -->
                    <div class="flex items-center gap-2 pt-1 text-xs font-mono">
                      <a
                        href="https://anilist.co/anime/{selectedRelease.anilist_metadata.id}"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-[#00E5FF] hover:underline flex items-center gap-1"
                      >
                        AniList #{selectedRelease.anilist_metadata.id} ↗
                      </a>
                      {#if selectedRelease.anilist_metadata.mal_id}
                        <span class="text-gray-600">•</span>
                        <a
                          href="https://myanimelist.net/anime/{selectedRelease.anilist_metadata.mal_id}"
                          target="_blank"
                          rel="noopener noreferrer"
                          class="text-[#2E51A2] hover:underline flex items-center gap-1 font-semibold"
                        >
                          MyAnimeList #{selectedRelease.anilist_metadata.mal_id} ↗
                        </a>
                      {/if}
                    </div>

                    {#if selectedRelease.anilist_metadata.genres && selectedRelease.anilist_metadata.genres.length > 0}
                      <div class="flex items-center gap-1.5 flex-wrap pt-1">
                        {#each selectedRelease.anilist_metadata.genres as genre}
                          <span class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-[#121212] text-[#98989D] border border-[#2C2C2E]">
                            {genre}
                          </span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>

                {#if selectedRelease.anilist_metadata.description}
                  <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-2">
                    <h4 class="text-xs font-mono uppercase text-[#98989D]">Synopsis</h4>
                    <div class="text-xs text-gray-300 leading-relaxed max-h-48 overflow-y-auto pr-2 space-y-1">
                      {@html selectedRelease.anilist_metadata.description}
                    </div>
                  </div>
                {/if}
              </div>
            {:else}
              <!-- Unsynced State -->
              <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-8 text-center space-y-4">
                <div class="w-12 h-12 mx-auto rounded-full bg-[#1E1E1E] border border-[#2C2C2E] flex items-center justify-center text-xl text-[#00E5FF]">
                  ?
                </div>
                <div>
                  <h3 class="text-sm font-bold text-white font-mono">No AniList Metadata Match</h3>
                  <p class="text-xs text-[#98989D] max-w-md mx-auto mt-1">
                    No metadata has been synchronized yet for title
                    <span class="text-[#00E5FF] font-semibold">"{selectedRelease.parsed.anime_title || selectedRelease.title}"</span>.
                  </p>
                </div>
              </div>
            {/if}

            <!-- Sync / Re-sync Query Box -->
            <div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4 space-y-3">
              <div class="flex items-center justify-between text-xs font-mono">
                <span class="text-[#98989D] uppercase text-[10px]">
                  {selectedRelease.anilist_metadata ? 'Re-sync / Match with Different Title' : 'Sync Metadata with AniList'}
                </span>
                {#if selectedRelease.anilist_metadata?.last_synced_at}
                  <span class="text-[10px] text-gray-500">
                    Last synced: {formatDate(selectedRelease.anilist_metadata.last_synced_at)}
                  </span>
                {/if}
              </div>

              {#if syncError}
                <div class="p-2.5 rounded-lg bg-[#3A1C1C] border border-[#FF453A]/40 text-xs font-mono text-[#FF453A]">
                  {syncError}
                </div>
              {/if}

              {#if syncSuccess}
                <div class="p-2.5 rounded-lg bg-[#1C3A24] border border-[#32D74B]/40 text-xs font-mono text-[#32D74B]">
                  {syncSuccess}
                </div>
              {/if}

              <div class="flex gap-2">
                <input
                  type="text"
                  bind:value={manualSyncTitle}
                  placeholder="Enter anime title (e.g. Frieren: Beyond Journey's End)"
                  class="flex-1 bg-[#1E1E1E] border border-[#2C2C2E] rounded-xl px-3 py-2 text-xs font-mono text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF]"
                />
                <button
                  type="button"
                  onclick={handleManualSync}
                  disabled={isSyncingMetadata || !manualSyncTitle.trim()}
                  class="bg-[#00E5FF] hover:bg-[#00c8e0] text-[#121212] font-bold text-xs font-mono px-4 py-2 rounded-xl transition-all cursor-pointer disabled:opacity-40 flex items-center gap-1.5 shrink-0"
                >
                  {#if isSyncingMetadata}
                    <span class="inline-block animate-spin w-3 h-3 border-2 border-[#121212] border-t-transparent rounded-full"></span>
                    Syncing...
                  {:else}
                    Sync AniList
                  {/if}
                </button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
