<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { library } from '#lib/stores/library.svelte';
  import { api } from '#lib/api/client';
  import { TARGET_LANGUAGE_OPTIONS } from '#lib/stores/translator.svelte';
  import type { LibraryFileItem, EncodingPreset, SubtitleTrack } from '#lib/api/types';
  import {
    RefreshCw,
    Zap,
    FolderKanban,
    CheckCircle2,
    Clock,
    Film,
    Languages,
    Search,
    X,
    Move,
    Trash2,
    Code2,
    Sparkles,
    Check,
    LayoutGrid,
    List,
    Layers,
    Copy,
    FolderOpen,
    ArrowUpDown,
    ChevronDown,
    ChevronUp,
    Info,
    RotateCcw,
    AlertTriangle,
    FileVideo,
    CheckSquare,
    Square,
    HardDrive
  } from 'lucide-svelte';

  const DEFAULT_TEMPLATE =
    '{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}';

  const TEMPLATE_TOKENS = [
    { token: '{anime_title}', label: 'Anime Title' },
    { token: '{season}', label: 'Season' },
    { token: '{season:02d}', label: 'Season (2-digit)' },
    { token: '{episode:02d}', label: 'Episode (2-digit)' },
    { token: '{resolution}', label: 'Resolution' },
    { token: '{ext}', label: 'Extension' }
  ];

  // View, filter & sort controls
  let filterStatus = $state<'ALL' | 'PENDING' | 'ORGANIZED' | 'ENCODED'>('ALL');
  let searchQuery = $state('');
  let sortBy = $state<'newest' | 'title' | 'episode' | 'size'>('newest');
  let viewMode = $state<'grid' | 'table'>('grid');
  let groupBySeries = $state(false);
  let collapsedSeries = $state<Set<string>>(new Set());

  // Selection & batch actions
  let selectedIds = $state<Set<string>>(new Set());
  let organizingItemId = $state<string | null>(null);
  let isBatchProcessing = $state(false);

  // Template editor state
  let customTemplate = $state(DEFAULT_TEMPLATE);
  let isTemplateEditorOpen = $state(false);

  // Toast / status banner state
  type ToastTone = 'success' | 'error' | 'info';
  let toast = $state<{ message: string; tone: ToastTone } | null>(null);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;
  let copiedId = $state<string | null>(null);

  // Confirm dialog state (replaces blocking window.confirm)
  let confirmDialog = $state<{
    title: string;
    description: string;
    confirmLabel: string;
    tone: 'cyan' | 'rose';
    onConfirm: () => Promise<void>;
  } | null>(null);
  let isConfirmRunning = $state(false);

  // Details drawer/modal state
  let detailItem = $state<LibraryFileItem | null>(null);

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

  // AI Translate modal state
  let isTranslateModalOpen = $state(false);
  let translateTargetFile = $state<LibraryFileItem | null>(null);
  let translateTracks = $state<SubtitleTrack[]>([]);
  let translateWarnings = $state<string[]>([]);
  let translateProbing = $state(false);
  let translateStreamIndex = $state<number | null>(null);
  let translateLangPreset = $state<string>('Indonesian');
  let translateCustomLang = $state('');
  let translateLangCode = $state('ind');
  let translateConfiguredModel = $state<string | null>(null);
  let translateEmbedMode = $state<'mux_keep_original' | 'mux_replace'>('mux_keep_original');
  let translateOutputPath = $state('');
  let isTranslating = $state(false);
  let translateError = $state<string | null>(null);
  let translateSuccess = $state<string | null>(null);

  const selectedPreset = $derived(presets.find((p) => p.id === selectedPresetId));

  const organizedPct = $derived(
    library.totalCount > 0 ? Math.round((library.movedCount / library.totalCount) * 100) : 0
  );

  const filteredItems = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = library.items.filter((item) => {
      if (filterStatus === 'PENDING' && item.is_moved) return false;
      if (filterStatus === 'ORGANIZED' && !item.is_moved) return false;
      if (filterStatus === 'ENCODED' && !item.is_encoded) return false;
      if (q.length > 0) {
        return (
          item.original_filename.toLowerCase().includes(q) ||
          (item.anime_title && item.anime_title.toLowerCase().includes(q)) ||
          item.current_path.toLowerCase().includes(q) ||
          (item.target_library_path && item.target_library_path.toLowerCase().includes(q))
        );
      }
      return true;
    });

    return [...list].sort((a, b) => {
      if (sortBy === 'title') {
        const titleA = (a.anime_title || a.original_filename).toLowerCase();
        const titleB = (b.anime_title || b.original_filename).toLowerCase();
        const cmp = titleA.localeCompare(titleB);
        if (cmp !== 0) return cmp;
        return parseEpisodeNum(a.episode) - parseEpisodeNum(b.episode);
      }
      if (sortBy === 'episode') {
        const epCmp = parseEpisodeNum(a.episode) - parseEpisodeNum(b.episode);
        if (epCmp !== 0) return epCmp;
        return (a.season ?? 1) - (b.season ?? 1);
      }
      if (sortBy === 'size') {
        return (b.file_size_bytes || 0) - (a.file_size_bytes || 0);
      }
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    });
  });

  interface SeriesGroup {
    key: string;
    title: string;
    coverPath?: string | null;
    items: LibraryFileItem[];
    pendingItems: LibraryFileItem[];
    totalBytes: number;
    encodedCount: number;
  }

  const groupedSeries = $derived.by((): SeriesGroup[] => {
    const map = new Map<string, SeriesGroup>();
    for (const item of filteredItems) {
      const title = item.anime_title?.trim() || 'Unmatched / Standalone Files';
      const key = item.anilist_id ? `anilist:${item.anilist_id}` : `title:${title.toLowerCase()}`;
      let group = map.get(key);
      if (!group) {
        group = {
          key,
          title,
          coverPath: item.anime_cover_path,
          items: [],
          pendingItems: [],
          totalBytes: 0,
          encodedCount: 0
        };
        map.set(key, group);
      }
      if (!group.coverPath && item.anime_cover_path) {
        group.coverPath = item.anime_cover_path;
      }
      group.items.push(item);
      if (!item.is_moved) group.pendingItems.push(item);
      if (item.is_encoded) group.encodedCount += 1;
      group.totalBytes += item.file_size_bytes || 0;
    }
    return Array.from(map.values());
  });

  const allVisibleSelected = $derived(
    filteredItems.length > 0 && filteredItems.every((item) => selectedIds.has(item.id))
  );

  const templatePreview = $derived.by(() => {
    const sample = library.items[0];
    const title = sample?.anime_title || 'Frieren Beyond Journeys End';
    const season = sample?.season ?? 1;
    const epNum = parseEpisodeNum(sample?.episode) || 7;
    const resolution = sample?.resolution || '1080p';
    const ext = getFileExtension(sample?.original_filename || 'episode.mkv');
    return customTemplate
      .replaceAll('{anime_title}', title)
      .replaceAll('{season:02d}', String(season).padStart(2, '0'))
      .replaceAll('{season}', String(season))
      .replaceAll('{episode:02d}', String(epNum).padStart(2, '0'))
      .replaceAll('{episode}', String(epNum))
      .replaceAll('{resolution}', resolution)
      .replaceAll('{ext}', ext);
  });

  onMount(() => {
    library.init();
    loadPresets();
  });

  function showToast(message: string, tone: ToastTone = 'info') {
    toast = { message, tone };
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => {
      toast = null;
    }, 4500);
  }

  function parseEpisodeNum(ep?: string | null): number {
    if (!ep) return 0;
    const match = ep.match(/\d+(\.\d+)?/);
    return match ? parseFloat(match[0]) : 0;
  }

  function getFileExtension(filename: string): string {
    const idx = filename.lastIndexOf('.');
    return idx > -1 ? filename.slice(idx + 1) : 'mkv';
  }

  function formatPathTail(path: string, segments = 2): string {
    const parts = path.split(/[\\/]+/).filter(Boolean);
    if (parts.length <= segments) return path;
    return `…/${parts.slice(-segments).join('/')}`;
  }

  function formatDateShort(dateStr: string): string {
    try {
      const d = new Date(dateStr);
      if (isNaN(d.getTime())) return dateStr;
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
    } catch {
      return dateStr;
    }
  }

  async function copyText(text: string, key: string) {
    try {
      await navigator.clipboard.writeText(text);
      copiedId = key;
      setTimeout(() => {
        if (copiedId === key) copiedId = null;
      }, 1800);
    } catch {
      showToast('Clipboard copy failed', 'error');
    }
  }

  function toggleSelection(id: string) {
    const next = new Set(selectedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedIds = next;
  }

  function toggleSelectAllVisible() {
    if (allVisibleSelected) {
      const next = new Set(selectedIds);
      for (const item of filteredItems) next.delete(item.id);
      selectedIds = next;
    } else {
      const next = new Set(selectedIds);
      for (const item of filteredItems) next.add(item.id);
      selectedIds = next;
    }
  }

  function toggleSeriesCollapse(key: string) {
    const next = new Set(collapsedSeries);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsedSeries = next;
  }

  function insertTemplateToken(token: string) {
    customTemplate = `${customTemplate}${customTemplate && !customTemplate.endsWith('/') ? '' : ''}${token}`;
  }

  async function loadPresets() {
    try {
      presets = await api.encoder.getPresets();
      if (!selectedPresetId) {
        const defaultPreset = presets.find((p) => p.is_default) || presets[0];
        if (defaultPreset) {
          selectedPresetId = defaultPreset.id;
        }
      }
    } catch (e) {
      console.warn('Failed to load presets for library:', e);
    }
  }

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
      showToast(`Moved "${customFilename.trim() || selectedFile.original_filename}"`, 'success');
      isMoveModalOpen = false;
      selectedFile = null;
    } catch (err: any) {
      moveError = err.message || 'Move operation failed';
    } finally {
      isMoving = false;
    }
  }

  async function handleAutoOrganizeSingle(item: LibraryFileItem) {
    organizingItemId = item.id;
    try {
      await library.autoOrganize(item.id, customTemplate);
      showToast(`Organized "${item.anime_title || item.original_filename}"`, 'success');
    } catch (err: any) {
      showToast(`Auto-organize failed: ${err.message}`, 'error');
    } finally {
      organizingItemId = null;
    }
  }

  function requestAutoOrganizeAll() {
    if (library.pendingCount === 0) return;
    confirmDialog = {
      title: 'Auto-Organize All Pending Files',
      description: `Move and rename ${library.pendingCount} pending file${library.pendingCount === 1 ? '' : 's'} into your library directory using the active naming template?`,
      confirmLabel: `Organize ${library.pendingCount} Files`,
      tone: 'cyan',
      onConfirm: async () => {
        const res = await library.autoOrganize(undefined, customTemplate);
        const count = res?.organized_count ?? library.pendingCount;
        showToast(`Organized ${count} library file${count === 1 ? '' : 's'}`, 'success');
      }
    };
  }

  async function handleOrganizeGroup(group: SeriesGroup) {
    if (group.pendingItems.length === 0) return;
    isBatchProcessing = true;
    let moved = 0;
    try {
      for (const item of group.pendingItems) {
        await library.autoOrganize(item.id, customTemplate);
        moved += 1;
      }
      showToast(`Organized ${moved} episode${moved === 1 ? '' : 's'} of ${group.title}`, 'success');
    } catch (err: any) {
      showToast(`Organized ${moved} files before error: ${err.message}`, 'error');
    } finally {
      isBatchProcessing = false;
    }
  }

  async function handleBatchAutoOrganize() {
    const pendingSelected = library.items.filter((i) => selectedIds.has(i.id) && !i.is_moved);
    if (pendingSelected.length === 0) {
      showToast('No pending files among selected items', 'info');
      return;
    }
    isBatchProcessing = true;
    let moved = 0;
    try {
      for (const item of pendingSelected) {
        await library.autoOrganize(item.id, customTemplate);
        moved += 1;
      }
      selectedIds = new Set();
      showToast(`Organized ${moved} selected file${moved === 1 ? '' : 's'}`, 'success');
    } catch (err: any) {
      showToast(`Batch organize stopped after ${moved} files: ${err.message}`, 'error');
    } finally {
      isBatchProcessing = false;
    }
  }

  function requestBatchDelete() {
    const count = selectedIds.size;
    if (count === 0) return;
    confirmDialog = {
      title: `Untrack ${count} Selected File${count === 1 ? '' : 's'}?`,
      description:
        'This removes tracking metadata from Nyaalist for the selected entries. Actual video files on disk will NOT be deleted.',
      confirmLabel: `Untrack ${count} File${count === 1 ? '' : 's'}`,
      tone: 'rose',
      onConfirm: async () => {
        const ids = Array.from(selectedIds);
        for (const id of ids) {
          await library.deleteFile(id);
        }
        selectedIds = new Set();
        showToast(`Removed tracking for ${ids.length} file${ids.length === 1 ? '' : 's'}`, 'info');
      }
    };
  }

  async function handleScan() {
    try {
      const res = await library.scan();
      if (res) {
        showToast(
          `Scan complete: ${res.scanned_count} checked, ${res.new_files_count} new file${res.new_files_count === 1 ? '' : 's'} added`,
          'success'
        );
      } else {
        showToast('Library scan completed', 'success');
      }
    } catch (err: any) {
      showToast(`Scan failed: ${err.message}`, 'error');
    }
  }

  function requestDelete(item: LibraryFileItem) {
    confirmDialog = {
      title: 'Untrack Media File?',
      description: `Remove tracking for "${item.original_filename}"? The file on disk will NOT be deleted.`,
      confirmLabel: 'Untrack File',
      tone: 'rose',
      onConfirm: async () => {
        await library.deleteFile(item.id);
        if (selectedIds.has(item.id)) {
          const next = new Set(selectedIds);
          next.delete(item.id);
          selectedIds = next;
        }
        if (detailItem?.id === item.id) {
          detailItem = null;
        }
        showToast(`Untracked "${item.original_filename}"`, 'info');
      }
    };
  }

  async function executeConfirm() {
    if (!confirmDialog || isConfirmRunning) return;
    isConfirmRunning = true;
    try {
      await confirmDialog.onConfirm();
      confirmDialog = null;
    } catch (err: any) {
      showToast(err.message || 'Action failed', 'error');
    } finally {
      isConfirmRunning = false;
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
      const defaultPreset = presets.find((p) => p.is_default) || presets[0];
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

  function updateTranslateOutputSuggestion() {
    if (!translateTargetFile) return;
    const source = translateTargetFile.current_path;
    const lastDot = source.lastIndexOf('.');
    const code = translateLangCode.trim() || 'sub';
    if (lastDot > 0) {
      translateOutputPath = `${source.substring(0, lastDot)} [${code}]${source.substring(lastDot)}`;
    } else {
      translateOutputPath = `${source} [${code}].mkv`;
    }
  }

  function handleTranslateLangChange() {
    const found = TARGET_LANGUAGE_OPTIONS.find((o) => o.label === translateLangPreset);
    if (found) {
      translateLangCode = found.code;
      updateTranslateOutputSuggestion();
    }
  }

  async function openTranslateModal(item: LibraryFileItem) {
    translateTargetFile = item;
    translateError = null;
    translateSuccess = null;
    translateTracks = [];
    translateWarnings = [];
    translateStreamIndex = null;
    updateTranslateOutputSuggestion();
    isTranslateModalOpen = true;

    translateProbing = true;
    try {
      const [probeRes, statusRes] = await Promise.all([
        api.translator.probeSubtitles(item.current_path),
        api.translator.getStatus().catch(() => null)
      ]);
      translateTracks = probeRes.tracks;
      translateWarnings = probeRes.warnings;
      translateStreamIndex = probeRes.recommended_stream_index ?? null;
      if (statusRes?.default_model) {
        translateConfiguredModel = statusRes.default_model;
      }
    } catch (err: any) {
      translateError = err.message || 'Failed to inspect embedded subtitle tracks';
    } finally {
      translateProbing = false;
    }
  }

  async function handleQueueTranslate(e: Event) {
    e.preventDefault();
    if (!translateTargetFile) return;
    const targetLang =
      translateLangPreset === 'custom' ? translateCustomLang.trim() : translateLangPreset;
    if (!targetLang) return;

    isTranslating = true;
    translateError = null;
    translateSuccess = null;

    try {
      await api.library.translateFile(translateTargetFile.id, {
        source_stream_index: translateStreamIndex ?? undefined,
        target_language: targetLang,
        target_language_code: translateLangCode.trim() || undefined,
        embed_mode: translateEmbedMode,
        target_file_path: translateOutputPath.trim() || undefined
      });
      translateSuccess = `AI translation queued for "${translateTargetFile.original_filename}"!`;
      setTimeout(() => {
        isTranslateModalOpen = false;
        translateTargetFile = null;
        translateSuccess = null;
      }, 1500);
    } catch (err: any) {
      translateError = err.message || 'Failed to queue AI translation job';
    } finally {
      isTranslating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    if (confirmDialog && !isConfirmRunning) {
      confirmDialog = null;
    } else if (isTranslateModalOpen) {
      isTranslateModalOpen = false;
    } else if (isEncodeModalOpen) {
      isEncodeModalOpen = false;
    } else if (isMoveModalOpen) {
      isMoveModalOpen = false;
    } else if (detailItem) {
      detailItem = null;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>Library Management | Nyaalist</title>
</svelte:head>

{#snippet libraryCard(item: LibraryFileItem)}
  {@const isSelected = selectedIds.has(item.id)}
  {@const isOrganizingThis = organizingItemId === item.id}
  <article
    class="glass-card-interactive rounded-3xl overflow-hidden flex flex-col group relative {isSelected
      ? 'ring-2 ring-cyan-500 dark:ring-cyan-400 border-cyan-400/60'
      : ''}"
  >
    <!-- Visual Cover Header -->
    <div class="relative h-44 bg-slate-200/80 dark:bg-slate-950/80 overflow-hidden shrink-0">
      {#if item.anime_cover_path}
        <!-- Ambient blurred backdrop -->
        <img
          src={getCoverUrl(item.anime_cover_path)}
          alt=""
          aria-hidden="true"
          class="absolute inset-0 w-full h-full object-cover scale-110 blur-xl opacity-45 dark:opacity-40 transition-transform duration-500 group-hover:scale-125"
          loading="lazy"
        />
        <div class="absolute inset-0 bg-gradient-to-t from-slate-950/90 via-slate-950/35 to-slate-950/30"></div>

        <!-- Crisp inset poster -->
        <div class="relative z-10 h-full p-3.5 flex items-end gap-3">
          <button
            type="button"
            onclick={() => (detailItem = item)}
            class="w-20 h-28 rounded-2xl overflow-hidden border border-white/25 shadow-xl shrink-0 cursor-pointer transition-transform duration-300 group-hover:scale-[1.03]"
            title="Inspect file details"
          >
            <img
              src={getCoverUrl(item.anime_cover_path)}
              alt={item.anime_title || item.original_filename}
              class="w-full h-full object-cover"
              loading="lazy"
            />
          </button>

          <div class="min-w-0 flex-1 pb-0.5">
            <div class="flex items-center gap-1.5 flex-wrap mb-1.5">
              <span class="px-2 py-0.5 rounded-full bg-black/50 backdrop-blur-md border border-white/15 text-[10px] font-mono font-bold text-white">
                S{String(item.season || 1).padStart(2, '0')} · E{item.episode || '01'}
              </span>
              <span class="px-2 py-0.5 rounded-full bg-cyan-500/25 backdrop-blur-md border border-cyan-400/40 text-[10px] font-mono font-bold text-cyan-200">
                {item.resolution || '1080p'}
              </span>
            </div>
            <button
              type="button"
              onclick={() => (detailItem = item)}
              class="text-left text-sm font-bold text-white leading-snug line-clamp-2 hover:text-cyan-300 transition-colors cursor-pointer drop-shadow"
              title={item.anime_title || item.original_filename}
            >
              {item.anime_title || item.original_filename}
            </button>
          </div>
        </div>
      {:else}
        <!-- Clean fallback header when no cover image is linked -->
        <div class="absolute inset-0 bg-gradient-to-br from-slate-800 via-slate-900 to-slate-950"></div>
        <div class="relative z-10 h-full p-3.5 flex flex-col justify-between">
          <div class="h-12 flex items-center justify-center mt-5">
            <div class="p-3 rounded-2xl bg-white/5 border border-white/10 text-slate-400">
              <FileVideo class="w-7 h-7" />
            </div>
          </div>
          <div>
            <div class="flex items-center gap-1.5 flex-wrap mb-1.5">
              <span class="px-2 py-0.5 rounded-full bg-white/10 border border-white/15 text-[10px] font-mono font-bold text-slate-200">
                S{String(item.season || 1).padStart(2, '0')} · E{item.episode || '01'}
              </span>
              <span class="px-2 py-0.5 rounded-full bg-cyan-500/20 border border-cyan-400/35 text-[10px] font-mono font-bold text-cyan-200">
                {item.resolution || '1080p'}
              </span>
            </div>
            <button
              type="button"
              onclick={() => (detailItem = item)}
              class="text-left text-sm font-bold text-white leading-snug line-clamp-2 hover:text-cyan-300 transition-colors cursor-pointer"
              title={item.anime_title || item.original_filename}
            >
              {item.anime_title || item.original_filename}
            </button>
          </div>
        </div>
      {/if}

      <!-- Top Overlay Bar: Selection + Status -->
      <div class="absolute top-2.5 left-2.5 right-2.5 z-20 flex items-center justify-between gap-2">
        <button
          type="button"
          onclick={() => toggleSelection(item.id)}
          aria-label={isSelected ? 'Deselect file' : 'Select file'}
          class="p-1.5 rounded-xl backdrop-blur-md border transition-all cursor-pointer {isSelected
            ? 'bg-cyan-500 text-slate-950 border-cyan-300 shadow-[0_0_12px_rgba(0,229,255,0.45)]'
            : 'bg-slate-950/55 text-white/80 border-white/15 opacity-80 sm:opacity-0 sm:group-hover:opacity-100 hover:bg-slate-900/80 hover:text-white'}"
        >
          {#if isSelected}
            <CheckSquare class="w-3.5 h-3.5" />
          {:else}
            <Square class="w-3.5 h-3.5" />
          {/if}
        </button>

        <div class="flex items-center gap-1.5">
          {#if item.is_encoded}
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-purple-500/85 text-white border border-purple-300/40 backdrop-blur-md shadow">
              ENCODED
            </span>
          {/if}
          {#if item.is_moved}
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/85 text-white border border-emerald-300/40 backdrop-blur-md shadow">
              ✓ ORGANIZED
            </span>
          {:else}
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/90 text-slate-950 border border-amber-200/50 backdrop-blur-md shadow">
              PENDING
            </span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Card Metadata Body -->
    <div class="p-4 flex-1 flex flex-col justify-between gap-3">
      <div class="space-y-2.5">
        <!-- Original Filename -->
        <div
          class="text-[11px] font-mono text-slate-600 dark:text-slate-300 truncate"
          title={item.original_filename}
        >
          {item.original_filename}
        </div>

        <!-- Size & Added Date -->
        <div class="flex items-center justify-between text-[11px] font-mono text-slate-500 dark:text-slate-400">
          <span class="inline-flex items-center gap-1 font-semibold text-slate-800 dark:text-slate-200">
            <HardDrive class="w-3 h-3 text-cyan-600 dark:text-cyan-400" />
            {formatBytes(item.file_size_bytes)}
          </span>
          <span>{formatDateShort(item.created_at)}</span>
        </div>

        <!-- Compact Path Preview + Copy -->
        <div class="rounded-2xl bg-slate-100/85 dark:bg-slate-950/60 border border-slate-200/80 dark:border-white/8 p-2.5 space-y-1.5">
          <div class="flex items-center justify-between gap-2 text-[10px] font-mono">
            <span class="uppercase font-bold tracking-wider {item.target_library_path ? 'text-cyan-700 dark:text-cyan-300' : 'text-slate-500 dark:text-slate-400'}">
              {item.is_moved ? 'Library Path' : item.target_library_path ? 'Target Path' : 'Source Path'}
            </span>
            <button
              type="button"
              onclick={() => copyText(item.target_library_path || item.current_path, `path-${item.id}`)}
              class="inline-flex items-center gap-1 text-slate-500 hover:text-cyan-600 dark:text-slate-400 dark:hover:text-cyan-300 transition-colors cursor-pointer"
              title="Copy full path"
            >
              {#if copiedId === `path-${item.id}`}
                <Check class="w-3 h-3 text-emerald-500" />
                <span class="text-emerald-600 dark:text-emerald-400">Copied</span>
              {:else}
                <Copy class="w-3 h-3" />
                <span>Copy</span>
              {/if}
            </button>
          </div>
          <div
            class="text-[11px] font-mono text-slate-700 dark:text-slate-300 truncate"
            title={item.target_library_path || item.current_path}
          >
            {formatPathTail(item.target_library_path || item.current_path, 3)}
          </div>
        </div>
      </div>

      <!-- Card Footer Actions -->
      <div class="pt-2 border-t border-slate-200/70 dark:border-white/10 flex items-center justify-between gap-1.5 font-mono">
        {#if !item.is_moved}
          <button
            type="button"
            onclick={() => handleAutoOrganizeSingle(item)}
            disabled={isOrganizingThis || library.organizing}
            class="btn-expressive flex-1 px-3 py-1.5 rounded-xl bg-gradient-to-r from-cyan-500/20 to-sky-500/20 hover:from-cyan-500/30 hover:to-sky-500/30 text-cyan-700 dark:text-cyan-300 border border-cyan-500/35 text-xs font-bold transition-all cursor-pointer flex items-center justify-center gap-1.5 disabled:opacity-50"
            title="Auto-organize file into library directory"
          >
            {#if isOrganizingThis}
              <RefreshCw class="w-3.5 h-3.5 animate-spin" />
              <span>Moving...</span>
            {:else}
              <Zap class="w-3.5 h-3.5" />
              <span>Auto Move</span>
            {/if}
          </button>
        {:else}
          <button
            type="button"
            onclick={() => openEncodeModal(item)}
            class="btn-expressive flex-1 px-3 py-1.5 rounded-xl bg-purple-500/15 hover:bg-purple-500/25 text-purple-700 dark:text-purple-300 border border-purple-500/35 text-xs font-semibold transition-all cursor-pointer flex items-center justify-center gap-1.5"
            title="Encode / transcode file with FFmpeg"
          >
            <Film class="w-3.5 h-3.5" />
            <span>Encode</span>
          </button>
        {/if}

        <div class="flex items-center gap-1">
          {#if !item.is_moved}
            <button
              type="button"
              onclick={() => openEncodeModal(item)}
              class="btn-expressive p-1.5 rounded-xl bg-purple-500/15 hover:bg-purple-500/25 text-purple-700 dark:text-purple-300 border border-purple-500/35 transition-colors cursor-pointer"
              title="Encode / transcode with FFmpeg"
              aria-label="Encode file"
            >
              <Film class="w-3.5 h-3.5" />
            </button>
          {/if}

          <button
            type="button"
            onclick={() => openTranslateModal(item)}
            class="btn-expressive p-1.5 rounded-xl bg-cyan-500/15 hover:bg-cyan-500/25 text-cyan-700 dark:text-cyan-300 border border-cyan-500/35 transition-colors cursor-pointer"
            title="Extract, AI translate subtitle, and embed"
            aria-label="AI Translate subtitle"
          >
            <Languages class="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            onclick={() => openMoveModal(item)}
            class="btn-expressive p-1.5 rounded-xl glass-pill text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white transition-colors cursor-pointer"
            title="Move or rename manually"
            aria-label="Move or rename file"
          >
            <Move class="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            onclick={() => (detailItem = item)}
            class="btn-expressive p-1.5 rounded-xl glass-pill text-slate-700 dark:text-slate-300 hover:text-cyan-600 dark:hover:text-cyan-300 transition-colors cursor-pointer"
            title="Inspect full paths & metadata"
            aria-label="Inspect file details"
          >
            <Info class="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            onclick={() => requestDelete(item)}
            class="btn-expressive p-1.5 rounded-xl glass-pill text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 hover:bg-rose-500/15 hover:border-rose-500/30 transition-colors cursor-pointer"
            title="Untrack file"
            aria-label="Untrack file"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  </article>
{/snippet}

<div class="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto w-full space-y-5 pb-24">
  <!-- Header & Actions Deck -->
  <div class="glass-panel rounded-3xl p-5 sm:p-6 shadow-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-slate-200/80 dark:border-white/10">
    <div class="flex items-center gap-4">
      <div class="p-3.5 rounded-2xl bg-sky-500/10 text-sky-600 dark:text-sky-300 border border-sky-500/25 shadow-[0_0_20px_rgba(56,189,248,0.18)]">
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

    <div class="flex items-center gap-2.5 flex-wrap">
      <button
        type="button"
        onclick={handleScan}
        disabled={library.scanning}
        class="btn-expressive px-4 py-2.5 rounded-2xl glass-pill text-slate-700 dark:text-white font-mono text-xs transition-all disabled:opacity-50 cursor-pointer flex items-center gap-2"
      >
        <RefreshCw class="w-3.5 h-3.5 {library.scanning ? 'animate-spin text-cyan-600 dark:text-cyan-400' : ''}" />
        <span>{library.scanning ? 'Scanning...' : 'Scan Downloads'}</span>
      </button>

      <button
        type="button"
        onclick={requestAutoOrganizeAll}
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

  <!-- Non-blocking Toast Banner -->
  {#if toast}
    <div
      transition:fly={{ y: -8, duration: 200, easing: cubicOut }}
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
          <AlertTriangle class="w-4 h-4 shrink-0" />
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

  <!-- Interactive Telemetry KPI Cards -->
  <div class="grid grid-cols-2 lg:grid-cols-5 gap-3.5">
    <button
      type="button"
      onclick={() => (filterStatus = 'ALL')}
      class="glass-card-interactive rounded-3xl p-4 sm:p-5 text-left cursor-pointer {filterStatus === 'ALL'
        ? 'ring-2 ring-cyan-500/60 dark:ring-cyan-400/50'
        : ''}"
    >
      <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">TOTAL FILES</span>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-slate-900 dark:text-white">
        {library.totalCount}
      </div>
    </button>

    <button
      type="button"
      onclick={() => (filterStatus = filterStatus === 'ORGANIZED' ? 'ALL' : 'ORGANIZED')}
      class="glass-card-interactive rounded-3xl p-4 sm:p-5 text-left cursor-pointer {filterStatus === 'ORGANIZED'
        ? 'ring-2 ring-emerald-500/70 dark:ring-emerald-400/60'
        : ''}"
    >
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">ORGANIZED</span>
        <CheckCircle2 class="w-4 h-4 text-emerald-500 dark:text-emerald-400" />
      </div>
      <div class="mt-2 flex items-baseline justify-between gap-2">
        <span class="text-2xl sm:text-3xl font-extrabold font-mono text-emerald-600 dark:text-emerald-400">
          {library.movedCount}
        </span>
        <span class="text-[11px] font-mono text-emerald-600/80 dark:text-emerald-300/80">
          {organizedPct}%
        </span>
      </div>
      <div class="mt-2 h-1.5 w-full rounded-full bg-slate-200 dark:bg-white/10 overflow-hidden">
        <div
          class="h-full rounded-full bg-emerald-500 transition-all duration-500"
          style="width: {organizedPct}%"
        ></div>
      </div>
    </button>

    <button
      type="button"
      onclick={() => (filterStatus = filterStatus === 'PENDING' ? 'ALL' : 'PENDING')}
      class="glass-card-interactive rounded-3xl p-4 sm:p-5 text-left cursor-pointer {filterStatus === 'PENDING'
        ? 'ring-2 ring-amber-500/70 dark:ring-amber-400/60'
        : ''}"
    >
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">PENDING MOVE</span>
        <Clock class="w-4 h-4 text-amber-500 dark:text-amber-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-amber-600 dark:text-amber-300">
        {library.pendingCount}
      </div>
    </button>

    <button
      type="button"
      onclick={() => (filterStatus = filterStatus === 'ENCODED' ? 'ALL' : 'ENCODED')}
      class="glass-card-interactive rounded-3xl p-4 sm:p-5 text-left cursor-pointer {filterStatus === 'ENCODED'
        ? 'ring-2 ring-purple-500/70 dark:ring-purple-400/60'
        : ''}"
    >
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">ENCODED</span>
        <Film class="w-4 h-4 text-purple-500 dark:text-purple-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-purple-600 dark:text-purple-300">
        {library.encodedCount}
      </div>
    </button>

    <div class="glass-card-interactive rounded-3xl p-4 sm:p-5 col-span-2 lg:col-span-1">
      <div class="flex justify-between items-start">
        <span class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 tracking-wider uppercase">TRACKED STORAGE</span>
        <HardDrive class="w-4 h-4 text-cyan-600 dark:text-cyan-400" />
      </div>
      <div class="mt-2 text-2xl sm:text-3xl font-extrabold font-mono text-cyan-600 dark:text-cyan-300">
        {formatBytes(library.totalSizeBytes)}
      </div>
    </div>
  </div>

  <!-- Collapsible Interactive Naming Template Bar -->
  <div class="glass-panel rounded-2xl border border-slate-200/80 dark:border-white/10 text-xs font-mono overflow-hidden">
    <div class="p-3.5 flex flex-col md:flex-row md:items-center justify-between gap-3">
      <div class="flex items-center gap-2 overflow-x-auto min-w-0">
        <Code2 class="w-4 h-4 text-cyan-600 dark:text-cyan-400 shrink-0" />
        <span class="text-slate-600 dark:text-slate-400 font-bold shrink-0">PATTERN:</span>
        <code class="px-2.5 py-0.5 rounded-full bg-slate-100 dark:bg-slate-950/60 text-cyan-700 dark:text-cyan-300 border border-slate-200/80 dark:border-white/5 whitespace-nowrap truncate">
          {customTemplate}
        </code>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <button
          type="button"
          onclick={() => (isTemplateEditorOpen = !isTemplateEditorOpen)}
          class="btn-expressive px-3 py-1 rounded-xl glass-pill text-[11px] text-slate-700 dark:text-slate-200 cursor-pointer flex items-center gap-1.5"
        >
          <span>{isTemplateEditorOpen ? 'Hide Editor' : 'Customize Pattern'}</span>
          {#if isTemplateEditorOpen}
            <ChevronUp class="w-3.5 h-3.5" />
          {:else}
            <ChevronDown class="w-3.5 h-3.5" />
          {/if}
        </button>
      </div>
    </div>

    {#if isTemplateEditorOpen}
      <div class="px-3.5 pb-3.5 pt-2 border-t border-slate-200/70 dark:border-white/10 space-y-3 bg-slate-50/50 dark:bg-slate-950/30">
        <div class="flex flex-col sm:flex-row gap-2">
          <input
            type="text"
            bind:value={customTemplate}
            aria-label="Library naming template"
            class="glass-input flex-1 rounded-2xl px-3.5 py-2 text-xs font-mono"
          />
          <button
            type="button"
            onclick={() => (customTemplate = DEFAULT_TEMPLATE)}
            class="btn-expressive px-3.5 py-2 rounded-2xl glass-pill text-xs text-slate-700 dark:text-slate-300 cursor-pointer flex items-center justify-center gap-1.5"
            title="Reset to default naming template"
          >
            <RotateCcw class="w-3.5 h-3.5" />
            <span>Reset</span>
          </button>
        </div>

        <div class="flex flex-wrap items-center gap-1.5">
          <span class="text-[11px] text-slate-500 dark:text-slate-400 mr-1">Insert token:</span>
          {#each TEMPLATE_TOKENS as item}
            <button
              type="button"
              onclick={() => insertTemplateToken(item.token)}
              class="px-2.5 py-1 rounded-full bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/25 text-[11px] cursor-pointer transition-colors"
              title="Append {item.label}"
            >
              {item.token}
            </button>
          {/each}
        </div>

        <div class="text-[11px] text-slate-500 dark:text-slate-400 flex items-center gap-2 overflow-x-auto">
          <span class="font-bold uppercase shrink-0">Preview:</span>
          <span class="text-emerald-700 dark:text-emerald-300 whitespace-nowrap">{templatePreview}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Filters, Sort, View Switcher & Search Deck -->
  <div class="glass-panel p-3.5 rounded-3xl border border-slate-200/80 dark:border-white/10 flex flex-col lg:flex-row justify-between items-stretch lg:items-center gap-3">
    <div class="flex items-center gap-1.5 flex-wrap">
      {#each [
        { id: 'ALL', label: 'ALL', count: library.totalCount },
        { id: 'PENDING', label: 'PENDING', count: library.pendingCount },
        { id: 'ORGANIZED', label: 'ORGANIZED', count: library.movedCount },
        { id: 'ENCODED', label: 'ENCODED', count: library.encodedCount }
      ] as tab (tab.id)}
        <button
          type="button"
          onclick={() => (filterStatus = tab.id as typeof filterStatus)}
          class="btn-expressive px-3.5 py-1.5 rounded-full text-xs font-mono font-semibold transition-all cursor-pointer flex items-center gap-1.5 {filterStatus === tab.id
            ? 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 shadow-[0_0_12px_rgba(0,229,255,0.2)]'
            : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 border border-transparent'}"
        >
          <span>{tab.label}</span>
          <span class="text-[10px] opacity-75">({tab.count})</span>
        </button>
      {/each}
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <!-- Search -->
      <div class="relative flex-1 sm:w-64 sm:flex-initial">
        <Search class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search title, file, path..."
          class="glass-input w-full rounded-2xl pl-9 pr-8 py-2 text-xs font-mono"
        />
        {#if searchQuery}
          <button
            type="button"
            onclick={() => (searchQuery = '')}
            class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-700 dark:hover:text-white text-xs cursor-pointer"
            aria-label="Clear search"
          >
            ✕
          </button>
        {/if}
      </div>

      <!-- Sort -->
      <div class="relative flex items-center">
        <ArrowUpDown class="w-3.5 h-3.5 text-slate-400 absolute left-3 pointer-events-none" />
        <select
          bind:value={sortBy}
          aria-label="Sort files"
          class="glass-input rounded-2xl pl-8 pr-3 py-2 text-xs font-mono cursor-pointer"
        >
          <option value="newest" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Newest</option>
          <option value="title" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Title A–Z</option>
          <option value="episode" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Episode #</option>
          <option value="size" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Largest Size</option>
        </select>
      </div>

      <!-- Group by Series toggle -->
      <button
        type="button"
        onclick={() => (groupBySeries = !groupBySeries)}
        class="btn-expressive px-3 py-2 rounded-2xl text-xs font-mono cursor-pointer flex items-center gap-1.5 border transition-all {groupBySeries
          ? 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border-cyan-500/40'
          : 'glass-pill text-slate-700 dark:text-slate-300'}"
        title="Group episodes by anime series"
      >
        <Layers class="w-3.5 h-3.5" />
        <span>Group Series</span>
      </button>

      <!-- Select All Visible -->
      {#if filteredItems.length > 0}
        <button
          type="button"
          onclick={toggleSelectAllVisible}
          class="btn-expressive px-3 py-2 rounded-2xl text-xs font-mono cursor-pointer flex items-center gap-1.5 border transition-all {allVisibleSelected
            ? 'bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border-cyan-500/40'
            : 'glass-pill text-slate-700 dark:text-slate-300'}"
          title="Select or deselect all visible files"
        >
          {#if allVisibleSelected}
            <CheckSquare class="w-3.5 h-3.5" />
          {:else}
            <Square class="w-3.5 h-3.5" />
          {/if}
          <span class="hidden sm:inline">Select All</span>
        </button>
      {/if}

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

  <!-- Results Summary -->
  {#if library.items.length > 0}
    <div class="flex items-center justify-between px-1 text-xs font-mono text-slate-500 dark:text-slate-400">
      <span>
        Showing <strong class="text-slate-800 dark:text-slate-200">{filteredItems.length}</strong> of {library.totalCount} tracked files
        {#if groupBySeries}
          · <strong class="text-cyan-600 dark:text-cyan-300">{groupedSeries.length}</strong> series
        {/if}
      </span>
      {#if filterStatus !== 'ALL' || searchQuery.trim()}
        <button
          type="button"
          onclick={() => {
            filterStatus = 'ALL';
            searchQuery = '';
          }}
          class="text-cyan-600 dark:text-cyan-400 hover:underline cursor-pointer"
        >
          Reset filters
        </button>
      {/if}
    </div>
  {/if}

  <!-- Library Content -->
  {#if library.loading && library.items.length === 0}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {#each Array(8) as _, idx (idx)}
        <div class="glass-panel rounded-3xl overflow-hidden border border-slate-200/80 dark:border-white/10 animate-pulse">
          <div class="h-44 bg-slate-200/80 dark:bg-slate-800/60"></div>
          <div class="p-4 space-y-3">
            <div class="h-3 bg-slate-200 dark:bg-slate-800 rounded-full w-3/4"></div>
            <div class="h-3 bg-slate-200 dark:bg-slate-800 rounded-full w-1/2"></div>
            <div class="h-12 bg-slate-200/70 dark:bg-slate-800/50 rounded-2xl"></div>
            <div class="h-8 bg-slate-200 dark:bg-slate-800 rounded-xl"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if filteredItems.length === 0}
    <div class="text-center py-20 glass-panel rounded-3xl border border-slate-200/80 dark:border-white/10 space-y-4">
      <FolderKanban class="w-12 h-12 text-slate-400 dark:text-slate-600 mx-auto" />
      <h3 class="text-base font-bold text-slate-900 dark:text-white font-mono">
        {library.items.length === 0 ? 'No library files found' : 'No files match your filters'}
      </h3>
      <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto font-mono">
        {library.items.length === 0
          ? 'Scan your downloads folder to automatically detect completed video files.'
          : 'Try clearing your search query or switching status filters.'}
      </p>
      <div class="pt-2 flex items-center justify-center gap-3">
        {#if library.items.length > 0}
          <button
            type="button"
            onclick={() => {
              filterStatus = 'ALL';
              searchQuery = '';
            }}
            class="btn-expressive px-5 py-2.5 glass-pill rounded-2xl text-xs font-mono cursor-pointer"
          >
            Clear Filters
          </button>
        {/if}
        <button
          type="button"
          onclick={handleScan}
          class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-cyan-400 to-sky-400 text-slate-950 font-bold rounded-2xl text-xs font-mono shadow-[0_0_15px_rgba(0,229,255,0.25)] cursor-pointer"
        >
          Scan Downloads Now
        </button>
      </div>
    </div>
  {:else if viewMode === 'grid'}
    {#if groupBySeries}
      <div class="space-y-6">
        {#each groupedSeries as group (group.key)}
          {@const isCollapsed = collapsedSeries.has(group.key)}
          <section class="glass-panel rounded-3xl p-4 sm:p-5 border border-slate-200/80 dark:border-white/10 space-y-4">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <button
                type="button"
                onclick={() => toggleSeriesCollapse(group.key)}
                class="flex items-center gap-3 text-left cursor-pointer group/series min-w-0"
              >
                {#if group.coverPath}
                  <img
                    src={getCoverUrl(group.coverPath)}
                    alt={group.title}
                    class="w-11 h-14 object-cover rounded-xl border border-white/15 shrink-0 shadow"
                    loading="lazy"
                  />
                {:else}
                  <div class="w-11 h-14 rounded-xl bg-slate-200 dark:bg-slate-900 border border-slate-300/60 dark:border-white/10 flex items-center justify-center shrink-0 text-slate-400">
                    <FolderOpen class="w-5 h-5" />
                  </div>
                {/if}
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <h2 class="text-base font-bold text-slate-900 dark:text-white group-hover/series:text-cyan-600 dark:group-hover/series:text-cyan-300 transition-colors truncate">
                      {group.title}
                    </h2>
                    {#if isCollapsed}
                      <ChevronDown class="w-4 h-4 text-slate-400 shrink-0" />
                    {:else}
                      <ChevronUp class="w-4 h-4 text-slate-400 shrink-0" />
                    {/if}
                  </div>
                  <div class="flex items-center gap-2 mt-1 text-[11px] font-mono text-slate-500 dark:text-slate-400 flex-wrap">
                    <span>{group.items.length} file{group.items.length === 1 ? '' : 's'}</span>
                    <span>•</span>
                    <span>{formatBytes(group.totalBytes)}</span>
                    {#if group.pendingItems.length > 0}
                      <span class="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-700 dark:text-amber-300 border border-amber-500/30 text-[10px]">
                        {group.pendingItems.length} pending
                      </span>
                    {:else}
                      <span class="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-500/30 text-[10px]">
                        All organized
                      </span>
                    {/if}
                  </div>
                </div>
              </button>

              {#if group.pendingItems.length > 0}
                <button
                  type="button"
                  onclick={() => handleOrganizeGroup(group)}
                  disabled={isBatchProcessing || library.organizing}
                  class="btn-expressive px-3.5 py-2 rounded-2xl bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 text-xs font-mono font-bold cursor-pointer flex items-center gap-1.5 shrink-0 disabled:opacity-50"
                >
                  <Zap class="w-3.5 h-3.5" />
                  <span>Move Pending ({group.pendingItems.length})</span>
                </button>
              {/if}
            </div>

            {#if !isCollapsed}
              <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                {#each group.items as item (item.id)}
                  {@render libraryCard(item)}
                {/each}
              </div>
            {/if}
          </section>
        {/each}
      </div>
    {:else}
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {#each filteredItems as item (item.id)}
          {@render libraryCard(item)}
        {/each}
      </div>
    {/if}
  {:else}
    <!-- Compact Table View -->
    <div class="glass-panel rounded-3xl overflow-hidden shadow-2xl border border-slate-200/80 dark:border-white/10">
      <div class="overflow-x-auto">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="border-b border-slate-200/80 dark:border-white/5 bg-slate-100/90 dark:bg-slate-950/70 text-[10px] uppercase font-mono text-slate-600 dark:text-slate-300 backdrop-blur-md">
              <th class="py-3.5 px-3 w-10">
                <button
                  type="button"
                  onclick={toggleSelectAllVisible}
                  class="p-1 rounded-lg hover:bg-black/5 dark:hover:bg-white/10 cursor-pointer"
                  aria-label="Toggle select all visible"
                >
                  {#if allVisibleSelected}
                    <CheckSquare class="w-4 h-4 text-cyan-600 dark:text-cyan-400" />
                  {:else}
                    <Square class="w-4 h-4" />
                  {/if}
                </button>
              </th>
              <th class="py-3.5 px-4">Anime & Episode</th>
              <th class="py-3.5 px-4">File Path & Size</th>
              <th class="py-3.5 px-4">Destination Target</th>
              <th class="py-3.5 px-4">Status</th>
              <th class="py-3.5 px-4 text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200/60 dark:divide-white/5 text-xs">
            {#each filteredItems as item (item.id)}
              {@const isSelected = selectedIds.has(item.id)}
              <tr class="hover:bg-black/[0.02] dark:hover:bg-white/[0.04] transition-colors {isSelected ? 'bg-cyan-500/5' : ''}">
                <td class="py-3.5 px-3">
                  <button
                    type="button"
                    onclick={() => toggleSelection(item.id)}
                    class="p-1 rounded-lg hover:bg-black/5 dark:hover:bg-white/10 cursor-pointer"
                    aria-label="Select row"
                  >
                    {#if isSelected}
                      <CheckSquare class="w-4 h-4 text-cyan-600 dark:text-cyan-400" />
                    {:else}
                      <Square class="w-4 h-4 text-slate-400" />
                    {/if}
                  </button>
                </td>
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
                      <button
                        type="button"
                        onclick={() => (detailItem = item)}
                        class="font-bold text-slate-900 dark:text-white hover:text-cyan-600 dark:hover:text-cyan-300 truncate max-w-sm block text-left cursor-pointer"
                        title={item.anime_title || item.original_filename}
                      >
                        {item.anime_title || item.original_filename}
                      </button>
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
                        <span class="px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-400/25 text-[10px] font-mono text-cyan-700 dark:text-cyan-300 font-semibold">
                          {item.resolution || '1080p'}
                        </span>
                      </div>
                    </div>
                  </div>
                </td>

                <td class="py-3.5 px-4 font-mono">
                  <div class="text-slate-900 dark:text-white text-xs font-bold">
                    {formatBytes(item.file_size_bytes)}
                  </div>
                  <div class="text-[10px] text-slate-500 dark:text-slate-400 truncate max-w-xs mt-1" title={item.current_path}>
                    {item.current_path}
                  </div>
                </td>

                <td class="py-3.5 px-4 font-mono">
                  {#if item.target_library_path}
                    <div class="text-[11px] text-cyan-700 dark:text-cyan-300 truncate max-w-xs" title={item.target_library_path}>
                      {item.target_library_path}
                    </div>
                  {:else}
                    <span class="text-slate-400 dark:text-slate-500 italic text-[11px]">Unassigned</span>
                  {/if}
                </td>

                <td class="py-3.5 px-4 whitespace-nowrap">
                  <div class="space-y-1">
                    <div>
                      {#if item.is_moved}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border border-emerald-500/40">
                          ✓ ORGANIZED
                        </span>
                      {:else}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-amber-500/40">
                          PENDING MOVE
                        </span>
                      {/if}
                    </div>
                    <div>
                      {#if item.is_encoded}
                        <span class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-purple-500/20 text-purple-700 dark:text-purple-300 border border-purple-500/40">
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

                <td class="py-3.5 px-4 text-right whitespace-nowrap">
                  <div class="flex items-center justify-end gap-1.5 font-mono">
                    {#if !item.is_moved}
                      <button
                        type="button"
                        onclick={() => handleAutoOrganizeSingle(item)}
                        disabled={organizingItemId === item.id}
                        class="btn-expressive px-3 py-1 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 text-xs transition-colors cursor-pointer"
                        title="Auto organize file into destination folder"
                      >
                        ⚡ Auto Move
                      </button>
                    {/if}

                    <button
                      type="button"
                      onclick={() => openEncodeModal(item)}
                      class="btn-expressive px-3 py-1 rounded-xl bg-purple-500/20 hover:bg-purple-500/30 text-purple-700 dark:text-purple-300 border border-purple-500/40 text-xs transition-colors cursor-pointer flex items-center gap-1"
                    >
                      <Film class="w-3.5 h-3.5" />
                      <span>Encode...</span>
                    </button>

                    <button
                      type="button"
                      onclick={() => openTranslateModal(item)}
                      class="btn-expressive px-3 py-1 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 text-xs transition-colors cursor-pointer flex items-center gap-1"
                    >
                      <Languages class="w-3.5 h-3.5" />
                      <span>AI Sub...</span>
                    </button>

                    <button
                      type="button"
                      onclick={() => openMoveModal(item)}
                      class="btn-expressive px-3 py-1 rounded-xl glass-pill text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white text-xs transition-colors cursor-pointer"
                    >
                      Move...
                    </button>

                    <button
                      type="button"
                      onclick={() => requestDelete(item)}
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

<!-- Floating Batch Selection Bar -->
{#if selectedIds.size > 0}
  <div
    transition:fly={{ y: 20, duration: 220, easing: cubicOut }}
    class="fixed bottom-5 left-1/2 -translate-x-1/2 z-50 w-[calc(100%-2rem)] max-w-2xl"
  >
    <div class="glass-panel-elevated rounded-full px-4 py-2.5 border border-cyan-500/40 shadow-2xl flex flex-wrap items-center justify-between gap-2 text-xs font-mono">
      <div class="flex items-center gap-2.5 pl-1">
        <span class="px-2.5 py-0.5 rounded-full bg-cyan-500 text-slate-950 font-bold">
          {selectedIds.size} selected
        </span>
        <button
          type="button"
          onclick={() => (selectedIds = new Set())}
          class="text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white cursor-pointer"
        >
          Clear
        </button>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          onclick={handleBatchAutoOrganize}
          disabled={isBatchProcessing}
          class="btn-expressive px-3.5 py-1.5 rounded-full bg-gradient-to-r from-cyan-500 to-sky-500 text-white font-bold cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
        >
          <Zap class="w-3.5 h-3.5" />
          <span>Auto-Move Pending</span>
        </button>
        <button
          type="button"
          onclick={requestBatchDelete}
          disabled={isBatchProcessing}
          class="btn-expressive px-3.5 py-1.5 rounded-full bg-rose-500/15 hover:bg-rose-500/25 text-rose-600 dark:text-rose-300 border border-rose-500/35 font-semibold cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
        >
          <Trash2 class="w-3.5 h-3.5" />
          <span>Untrack</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- File Details Modal -->
{#if detailItem}
  <div
    transition:fade={{ duration: 180, easing: cubicOut }}
    onclick={(e) => { if (e.target === e.currentTarget) detailItem = null; }}
    onkeydown={(e) => { if (e.key === 'Escape') detailItem = null; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl overflow-y-auto"
  >
    <div
      transition:scale={{ duration: 240, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-2xl w-full shadow-2xl relative space-y-5 border border-slate-200/80 dark:border-white/15 my-8"
    >
      <div class="flex justify-between items-start gap-4">
        <div class="flex items-start gap-4 min-w-0">
          {#if detailItem.anime_cover_path}
            <img
              src={getCoverUrl(detailItem.anime_cover_path)}
              alt={detailItem.anime_title || detailItem.original_filename}
              class="w-16 h-24 object-cover rounded-2xl border border-white/15 shrink-0 shadow-lg"
            />
          {/if}
          <div class="min-w-0">
            <h3 class="text-lg font-bold text-slate-900 dark:text-white leading-snug">
              {detailItem.anime_title || detailItem.original_filename}
            </h3>
            <p class="text-xs font-mono text-slate-500 dark:text-slate-400 break-all mt-1">
              {detailItem.original_filename}
            </p>
            <div class="flex items-center gap-1.5 mt-2.5 flex-wrap">
              <span class="px-2.5 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[10px] font-mono font-semibold">
                Season {detailItem.season || 1} · Episode {detailItem.episode || '01'}
              </span>
              <span class="px-2.5 py-0.5 rounded-full bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 text-[10px] font-mono font-semibold">
                {detailItem.resolution || '1080p'}
              </span>
              <span class="px-2.5 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[10px] font-mono">
                {formatBytes(detailItem.file_size_bytes)}
              </span>
            </div>
          </div>
        </div>
        <button
          type="button"
          onclick={() => (detailItem = null)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer shrink-0"
          aria-label="Close details"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="space-y-3 text-xs font-mono">
        <div class="p-3.5 rounded-2xl bg-slate-100/80 dark:bg-slate-950/60 border border-slate-200/80 dark:border-white/10 space-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-[10px] uppercase font-bold text-slate-500 dark:text-slate-400">Current Disk Path</span>
            <button
              type="button"
              onclick={() => copyText(detailItem!.current_path, 'detail-cur')}
              class="text-cyan-600 dark:text-cyan-300 hover:underline cursor-pointer flex items-center gap-1"
            >
              {#if copiedId === 'detail-cur'}
                <Check class="w-3 h-3" /> Copied
              {:else}
                <Copy class="w-3 h-3" /> Copy
              {/if}
            </button>
          </div>
          <div class="break-all text-slate-800 dark:text-slate-200">{detailItem.current_path}</div>
        </div>

        <div class="p-3.5 rounded-2xl bg-slate-100/80 dark:bg-slate-950/60 border border-slate-200/80 dark:border-white/10 space-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-[10px] uppercase font-bold text-slate-500 dark:text-slate-400">Destination Library Path</span>
            {#if detailItem.target_library_path}
              <button
                type="button"
                onclick={() => copyText(detailItem!.target_library_path!, 'detail-tgt')}
                class="text-cyan-600 dark:text-cyan-300 hover:underline cursor-pointer flex items-center gap-1"
              >
                {#if copiedId === 'detail-tgt'}
                  <Check class="w-3 h-3" /> Copied
                {:else}
                  <Copy class="w-3 h-3" /> Copy
                {/if}
              </button>
            {/if}
          </div>
          <div class="break-all text-cyan-700 dark:text-cyan-300">
            {detailItem.target_library_path || 'Unassigned — use Auto Move or Move manually'}
          </div>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-slate-200/70 dark:border-white/10 text-xs font-mono">
        <div class="flex flex-wrap items-center gap-2">
          {#if !detailItem.is_moved}
            <button
              type="button"
              onclick={() => {
                const target = detailItem!;
                detailItem = null;
                handleAutoOrganizeSingle(target);
              }}
              class="btn-expressive px-3.5 py-2 rounded-2xl bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 font-bold cursor-pointer flex items-center gap-1.5"
            >
              <Zap class="w-3.5 h-3.5" />
              <span>Auto Move</span>
            </button>
          {/if}
          <button
            type="button"
            onclick={() => {
              const target = detailItem!;
              detailItem = null;
              openEncodeModal(target);
            }}
            class="btn-expressive px-3.5 py-2 rounded-2xl bg-purple-500/20 text-purple-700 dark:text-purple-300 border border-purple-500/40 cursor-pointer flex items-center gap-1.5"
          >
            <Film class="w-3.5 h-3.5" />
            <span>Encode</span>
          </button>
          <button
            type="button"
            onclick={() => {
              const target = detailItem!;
              detailItem = null;
              openTranslateModal(target);
            }}
            class="btn-expressive px-3.5 py-2 rounded-2xl bg-cyan-500/20 text-cyan-700 dark:text-cyan-300 border border-cyan-500/40 cursor-pointer flex items-center gap-1.5"
          >
            <Languages class="w-3.5 h-3.5" />
            <span>AI Sub</span>
          </button>
          <button
            type="button"
            onclick={() => {
              const target = detailItem!;
              detailItem = null;
              openMoveModal(target);
            }}
            class="btn-expressive px-3.5 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-200 cursor-pointer flex items-center gap-1.5"
          >
            <Move class="w-3.5 h-3.5" />
            <span>Move / Rename</span>
          </button>
        </div>

        <button
          type="button"
          onclick={() => requestDelete(detailItem!)}
          class="btn-expressive px-3.5 py-2 rounded-2xl bg-rose-500/15 text-rose-600 dark:text-rose-300 border border-rose-500/35 cursor-pointer flex items-center gap-1.5"
        >
          <Trash2 class="w-3.5 h-3.5" />
          <span>Untrack</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Confirmation Modal -->
{#if confirmDialog}
  <div
    transition:fade={{ duration: 180, easing: cubicOut }}
    onclick={(e) => { if (e.target === e.currentTarget && !isConfirmRunning) confirmDialog = null; }}
    onkeydown={(e) => { if (e.key === 'Escape' && !isConfirmRunning) confirmDialog = null; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    class="fixed inset-0 z-[70] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl"
  >
    <div
      transition:scale={{ duration: 220, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-md w-full shadow-2xl space-y-4 border border-slate-200/80 dark:border-white/15"
    >
      <div class="flex items-start gap-3">
        <div class="p-2.5 rounded-2xl {confirmDialog.tone === 'rose'
          ? 'bg-rose-500/15 text-rose-500 border border-rose-500/30'
          : 'bg-cyan-500/15 text-cyan-500 border border-cyan-500/30'}">
          {#if confirmDialog.tone === 'rose'}
            <AlertTriangle class="w-5 h-5" />
          {:else}
            <Zap class="w-5 h-5" />
          {/if}
        </div>
        <div>
          <h3 class="text-base font-bold text-slate-900 dark:text-white">{confirmDialog.title}</h3>
          <p class="text-xs font-mono text-slate-600 dark:text-slate-300 mt-1.5 leading-relaxed">
            {confirmDialog.description}
          </p>
        </div>
      </div>

      <div class="flex justify-end gap-2.5 pt-2 text-xs font-mono">
        <button
          type="button"
          disabled={isConfirmRunning}
          onclick={() => (confirmDialog = null)}
          class="btn-expressive px-4 py-2 rounded-2xl glass-pill text-slate-700 dark:text-slate-300 cursor-pointer disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={isConfirmRunning}
          onclick={executeConfirm}
          class="btn-expressive px-4 py-2 rounded-2xl font-bold text-white cursor-pointer flex items-center gap-2 disabled:opacity-50 {confirmDialog.tone === 'rose'
            ? 'bg-gradient-to-r from-rose-500 to-pink-600 shadow-[0_0_15px_rgba(244,63,94,0.3)]'
            : 'bg-gradient-to-r from-cyan-500 to-sky-500 shadow-[0_0_15px_rgba(0,229,255,0.28)]'}"
        >
          {#if isConfirmRunning}
            <RefreshCw class="w-3.5 h-3.5 animate-spin" />
            <span>Working...</span>
          {:else}
            <span>{confirmDialog.confirmLabel}</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Manual Move / Rename Modal -->
{#if isMoveModalOpen && selectedFile}
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    onclick={(e) => { if (e.target === e.currentTarget) isMoveModalOpen = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') isMoveModalOpen = false; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Move class="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
          <span>Move or Rename Media File</span>
        </h3>
        <button
          type="button"
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
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    onclick={(e) => { if (e.target === e.currentTarget) isEncodeModalOpen = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') isEncodeModalOpen = false; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Film class="w-5 h-5 text-purple-600 dark:text-purple-400" />
          <span>Encode Media File</span>
        </h3>
        <button
          type="button"
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

<!-- AI Translate Subtitle Modal -->
{#if isTranslateModalOpen && translateTargetFile}
  <div
    transition:fade={{ duration: 200, easing: cubicOut }}
    onclick={(e) => { if (e.target === e.currentTarget) isTranslateModalOpen = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') isTranslateModalOpen = false; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-xl overflow-y-auto"
  >
    <div
      transition:scale={{ duration: 260, start: 0.94, opacity: 0, easing: cubicOut }}
      class="glass-panel-elevated rounded-3xl p-6 max-w-xl w-full shadow-2xl relative space-y-4 border border-slate-200/80 dark:border-white/15 my-8"
    >
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Languages class="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
          <span>AI Subtitle Translator</span>
        </h3>
        <button
          type="button"
          onclick={() => (isTranslateModalOpen = false)}
          class="btn-expressive p-2 rounded-full glass-pill text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      {#if translateSuccess}
        <div class="p-3 bg-emerald-500/10 border border-emerald-500/40 rounded-2xl text-xs font-mono text-emerald-600 dark:text-emerald-300 flex items-center justify-between">
          <span class="flex items-center gap-1.5">
            <Check class="w-4 h-4" />
            <span>{translateSuccess}</span>
          </span>
          <a href="/translator" class="underline font-bold text-cyan-600 dark:text-cyan-300 ml-2">
            View in AI Sub &rarr;
          </a>
        </div>
      {/if}

      {#if translateError}
        <div class="p-3 bg-rose-500/10 border border-rose-500/40 rounded-2xl text-xs font-mono text-rose-600 dark:text-rose-300">
          {translateError}
        </div>
      {/if}

      <form onsubmit={handleQueueTranslate} class="space-y-4 text-xs font-mono">
        <div class="space-y-1.5">
          <label for="lib-tr-source" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">SOURCE FILE</label>
          <input
            id="lib-tr-source"
            type="text"
            readonly
            value={translateTargetFile.current_path}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5 text-slate-500 dark:text-slate-400 select-all"
          />
        </div>

        {#if translateProbing}
          <div class="p-3 glass-panel rounded-2xl text-slate-500 dark:text-slate-400 flex items-center gap-2">
            <RefreshCw class="w-3.5 h-3.5 animate-spin text-cyan-400" />
            <span>Inspecting embedded subtitle tracks...</span>
          </div>
        {:else if translateWarnings.length > 0}
          <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/40 text-amber-700 dark:text-amber-300 space-y-1">
            {#each translateWarnings as warn}
              <div>⚠ {warn}</div>
            {/each}
          </div>
        {/if}

        {#if translateTracks.length > 0}
          <div class="space-y-1.5">
            <label for="lib-tr-track" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">
              EXTRACTED SUBTITLE TRACK
            </label>
            <select
              id="lib-tr-track"
              bind:value={translateStreamIndex}
              class="glass-input w-full rounded-2xl px-3.5 py-2.5"
            >
              {#each translateTracks as track (track.stream_index)}
                <option
                  value={track.stream_index}
                  disabled={!track.is_text_based}
                  class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white"
                >
                  #{track.stream_index} [{track.codec_name.toUpperCase()}] {track.language || 'und'} - {track.title || 'Untitled'}{!track.is_text_based ? ' (Bitmap - unsupported)' : ''}
                </option>
              {/each}
            </select>
          </div>
        {/if}

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <div class="sm:col-span-2 space-y-1.5">
            <label for="lib-tr-lang" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">TARGET LANGUAGE</label>
            <select
              id="lib-tr-lang"
              bind:value={translateLangPreset}
              onchange={handleTranslateLangChange}
              class="glass-input w-full rounded-2xl px-3.5 py-2.5"
            >
              {#each TARGET_LANGUAGE_OPTIONS as opt}
                <option value={opt.label} class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
                  {opt.label} ({opt.code})
                </option>
              {/each}
              <option value="custom" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">Custom Language...</option>
            </select>
            {#if translateLangPreset === 'custom'}
              <input
                type="text"
                required
                bind:value={translateCustomLang}
                placeholder="Enter target language..."
                class="glass-input w-full rounded-2xl px-3.5 py-2 mt-1.5"
              />
            {/if}
          </div>

          <div class="space-y-1.5">
            <label for="lib-tr-code" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">ISO CODE</label>
            <input
              id="lib-tr-code"
              type="text"
              bind:value={translateLangCode}
              oninput={() => updateTranslateOutputSuggestion()}
              maxlength="8"
              class="glass-input w-full rounded-2xl px-3.5 py-2.5 lowercase"
            />
          </div>
        </div>

        <div class="space-y-1.5">
          <label for="lib-tr-mode" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">EMBED MODE</label>
          <select
            id="lib-tr-mode"
            bind:value={translateEmbedMode}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          >
            <option value="mux_keep_original" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
              Keep Original + Add Translated
            </option>
            <option value="mux_replace" class="bg-white text-slate-900 dark:bg-slate-900 dark:text-white">
              Replace Existing Subtitles
            </option>
          </select>
          {#if translateConfiguredModel}
            <div class="text-[11px] text-slate-500 dark:text-slate-400 pt-1">
              Configured Model: <strong class="text-cyan-600 dark:text-cyan-300">{translateConfiguredModel}</strong> (managed in <a href="/settings" class="underline">Settings</a>)
            </div>
          {/if}
        </div>

        <div class="space-y-1.5">
          <label for="lib-tr-target" class="block text-slate-600 dark:text-slate-400 text-[10px] uppercase font-bold">OUTPUT VIDEO PATH</label>
          <input
            id="lib-tr-target"
            type="text"
            bind:value={translateOutputPath}
            class="glass-input w-full rounded-2xl px-3.5 py-2.5"
          />
        </div>

        <div class="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onclick={() => (isTranslateModalOpen = false)}
            class="btn-expressive px-4 py-2 glass-pill text-slate-700 dark:text-slate-300 rounded-2xl cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isTranslating || translateProbing}
            class="btn-expressive px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-indigo-500 text-white font-bold rounded-2xl transition-all cursor-pointer disabled:opacity-40 flex items-center gap-2 shadow-[0_0_15px_rgba(0,229,255,0.28)]"
          >
            {#if isTranslating}
              <span class="inline-block animate-spin w-3 h-3 border-2 border-white border-t-transparent rounded-full"></span>
              <span>Queueing...</span>
            {:else}
              <Sparkles class="w-4 h-4" />
              <span>Translate & Embed</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
