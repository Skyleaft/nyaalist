<script lang="ts">
  import { api } from '#lib/api/client';
  import type { NyaaRelease, AnimeMetadata } from '#lib/api/types';
  import { onMount } from 'svelte';

  let releases = $state<NyaaRelease[]>([]);
  let loading = $state<boolean>(true);
  let error = $state<string | null>(null);

  // Filters
  let category = $state<string>('1_2');
  let searchQuery = $state<string>('');
  let userQuery = $state<string>('');
  let page = $state<number>(1);

  // Modal State
  let selectedAnime = $state<AnimeMetadata | null>(null);
  let copiedMagnetId = $state<number | null>(null);
  let downloadingIds = $state<Set<number>>(new Set());
  let downloadedSuccessIds = $state<Set<number>>(new Set());

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
      if (userQuery.trim().length > 0) {
        releases = await api.explore.getUserReleases(userQuery.trim(), page);
      } else {
        releases = await api.explore.getLatest({
          category,
          page,
          query: searchQuery.trim() || undefined
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

  function openAnimeModal(meta?: AnimeMetadata | null) {
    if (meta) {
      selectedAnime = meta;
    }
  }

  function closeModal() {
    selectedAnime = null;
  }

  onMount(() => {
    loadReleases();
  });
</script>

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
            <span class="text-[#98989D]">User Filter:</span>
            <span class="text-[#00E5FF] font-bold">@{userQuery}</span>
            <button
              onclick={clearUserFilter}
              class="text-gray-400 hover:text-[#FF453A] ml-1 cursor-pointer font-bold"
              title="Clear user filter"
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
          disabled={userQuery.trim().length > 0}
          placeholder="e.g. Dragon Ball Super, Kozame..."
          class="w-full bg-[#121212] border border-[#2C2C2E] rounded-xl px-3 py-2 text-white placeholder-gray-600 focus:outline-none focus:border-[#00E5FF] disabled:opacity-40"
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
            <th class="py-3 px-4 w-24">Size</th>
            <th class="py-3 px-4 w-24">S / L</th>
            <th class="py-3 px-4 w-28 text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#2C2C2E]">
          {#if loading && releases.length === 0}
            <tr>
              <td colspan="6" class="py-16 text-center text-[#98989D] font-mono text-xs">
                <div class="inline-block w-5 h-5 border-2 border-[#00E5FF] border-t-transparent rounded-full animate-spin mb-2"></div>
                <div>Scraping Nyaa.si and querying AniList GraphQL API...</div>
              </td>
            </tr>
          {:else if releases.length === 0}
            <tr>
              <td colspan="6" class="py-12 text-center text-[#98989D] font-mono text-xs">
                No releases found matching your search criteria.
              </td>
            </tr>
          {:else}
            {#each releases as release (release.id)}
              <tr class="hover:bg-[#252525] transition-colors group">
                <!-- Cover Image Thumbnail -->
                <td class="py-3 px-4">
                  {#if release.anilist_metadata?.cover_image_local_path}
                    <button
                      onclick={() => openAnimeModal(release.anilist_metadata)}
                      class="block w-10 h-14 rounded overflow-hidden border border-[#2C2C2E] hover:border-[#00E5FF] transition-colors cursor-pointer bg-[#121212]"
                    >
                      <img
                        src={release.anilist_metadata.cover_image_local_path}
                        alt="Anime Poster"
                        class="w-full h-full object-cover"
                        loading="lazy"
                      />
                    </button>
                  {:else if release.anilist_metadata?.cover_image_url}
                    <button
                      onclick={() => openAnimeModal(release.anilist_metadata)}
                      class="block w-10 h-14 rounded overflow-hidden border border-[#2C2C2E] hover:border-[#00E5FF] transition-colors cursor-pointer bg-[#121212]"
                    >
                      <img
                        src={release.anilist_metadata.cover_image_url}
                        alt="Anime Poster"
                        class="w-full h-full object-cover"
                        loading="lazy"
                      />
                    </button>
                  {:else}
                    <div class="w-10 h-14 rounded bg-[#121212] border border-[#2C2C2E] flex items-center justify-center text-[10px] text-gray-600 font-mono">
                      N/A
                    </div>
                  {/if}
                </td>

                <!-- Release Title & Badges -->
                <td class="py-3 px-4 space-y-1.5">
                  <div class="flex items-center gap-2 flex-wrap">
                    {#if release.parsed.uploader}
                      <span class="text-xs font-mono font-semibold px-2 py-0.5 rounded bg-[#121212] text-[#00E5FF] border border-[#2C2C2E]">
                        [{release.parsed.uploader}]
                      </span>
                    {/if}
                    <span class="text-sm font-semibold text-white group-hover:text-[#00E5FF] transition-colors">
                      {release.parsed.anime_title}
                    </span>
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
                      onclick={() => openAnimeModal(release.anilist_metadata)}
                      class="text-left group/meta cursor-pointer"
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
                    <span class="text-xs font-mono text-gray-500 italic">No match</span>
                  {/if}
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
                      onclick={() => downloadRelease(release)}
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
                      onclick={() => copyMagnet(release)}
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

<!-- AniList Metadata Detail Modal -->
{#if selectedAnime}
  <div class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
    <div class="bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl max-w-2xl w-full p-6 shadow-2xl relative space-y-4">
      <button
        onclick={closeModal}
        class="absolute top-4 right-4 text-gray-400 hover:text-white font-mono text-sm cursor-pointer"
      >
        ✕
      </button>

      <div class="flex gap-4">
        {#if selectedAnime.cover_image_local_path || selectedAnime.cover_image_url}
          <img
            src={selectedAnime.cover_image_local_path || selectedAnime.cover_image_url}
            alt={selectedAnime.title_romaji}
            class="w-28 h-40 object-cover rounded-xl border border-[#2C2C2E] shrink-0"
          />
        {/if}

        <div class="space-y-1.5 flex-1 min-w-0">
          <h2 class="text-lg font-bold text-white tracking-tight line-clamp-1">
            {selectedAnime.title_english || selectedAnime.title_romaji}
          </h2>
          <div class="text-xs font-mono text-[#00E5FF] line-clamp-1">
            {selectedAnime.title_romaji}
          </div>

          <div class="flex items-center gap-2 text-xs font-mono pt-1">
            {#if selectedAnime.average_score}
              <span class="px-2 py-0.5 rounded bg-[#121212] text-[#32D74B] border border-[#2C2C2E] font-bold">
                ★ {selectedAnime.average_score}%
              </span>
            {/if}
            <span class="px-2 py-0.5 rounded bg-[#121212] text-gray-300 border border-[#2C2C2E]">
              {selectedAnime.format || 'TV'}
            </span>
            <span class="px-2 py-0.5 rounded bg-[#121212] text-gray-300 border border-[#2C2C2E]">
              {selectedAnime.status || 'RELEASING'}
            </span>
            {#if selectedAnime.episodes}
              <span class="text-[#98989D]">
                {selectedAnime.episodes} Episodes
              </span>
            {/if}
          </div>

          {#if selectedAnime.genres && selectedAnime.genres.length > 0}
            <div class="flex items-center gap-1 flex-wrap pt-1.5">
              {#each selectedAnime.genres as genre}
                <span class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-[#121212] text-[#98989D] border border-[#2C2C2E]">
                  {genre}
                </span>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      {#if selectedAnime.description}
        <div class="border-t border-[#2C2C2E] pt-3">
          <h3 class="text-xs font-mono uppercase text-[#98989D] mb-1">Synopsis</h3>
          <p class="text-xs text-gray-300 leading-relaxed max-h-48 overflow-y-auto pr-2">
            {@html selectedAnime.description}
          </p>
        </div>
      {/if}
    </div>
  </div>
{/if}
