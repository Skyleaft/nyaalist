import { api, getWebSocketUrl } from '#lib/api/client';
import type { DownloadJob, TorrentStatsSnapshot } from '#lib/api/types';

class TorrentStore {
  jobs = $state<DownloadJob[]>([]);
  loading = $state<boolean>(false);
  wsConnected = $state<boolean>(false);
  error = $state<string | null>(null);

  private ws: WebSocket | null = null;
  private reconnectTimer: any = null;

  // Derived metrics
  activeCount = $derived(this.jobs.filter(j => j.status === 'DOWNLOADING').length);
  completedCount = $derived(this.jobs.filter(j => j.status === 'COMPLETED').length);
  pausedCount = $derived(this.jobs.filter(j => j.status === 'PAUSED').length);

  totalDownloadSpeed = $derived(
    this.jobs
      .filter(j => j.status === 'DOWNLOADING')
      .reduce((acc, j) => acc + (j.download_speed || 0), 0)
  );

  totalUploadSpeed = $derived(
    this.jobs
      .filter(j => j.status === 'DOWNLOADING')
      .reduce((acc, j) => acc + (j.upload_speed || 0), 0)
  );

  async init() {
    await this.fetchJobs();
    this.connectWs();
  }

  async fetchJobs() {
    this.loading = true;
    try {
      this.jobs = await api.downloader.getJobs();
    } catch (err: any) {
      console.warn('Failed to fetch download jobs:', err);
    } finally {
      this.loading = false;
    }
  }

  connectWs() {
    if (typeof window === 'undefined') return;
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }

    try {
      const url = getWebSocketUrl('/api/ws');
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        this.wsConnected = true;
        if (this.reconnectTimer) {
          clearTimeout(this.reconnectTimer);
          this.reconnectTimer = null;
        }
      };

      this.ws.onmessage = (event) => {
        try {
          const payload = JSON.parse(event.data);
          if (payload.event === 'torrent_progress' && Array.isArray(payload.data)) {
            this.handleProgressUpdate(payload.data as TorrentStatsSnapshot[]);
          }
        } catch (e) {
          console.error('Failed parsing WS message:', e);
        }
      };

      this.ws.onclose = () => {
        this.wsConnected = false;
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        this.wsConnected = false;
      };
    } catch (err) {
      this.wsConnected = false;
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connectWs();
    }, 3000);
  }

  private handleProgressUpdate(snapshots: TorrentStatsSnapshot[]) {
    const snapMap = new Map<string, TorrentStatsSnapshot>();
    for (const s of snapshots) {
      snapMap.set(s.info_hash.toLowerCase(), s);
    }

    let modified = false;
    const nextJobs = this.jobs.map(job => {
      const snap = snapMap.get(job.info_hash.toLowerCase());
      if (snap) {
        modified = true;
        return {
          ...job,
          total_bytes: snap.total_bytes > 0 ? snap.total_bytes : job.total_bytes,
          downloaded_bytes: snap.downloaded_bytes,
          uploaded_bytes: snap.uploaded_bytes,
          download_speed: snap.download_speed,
          upload_speed: snap.upload_speed,
          progress_pct: snap.progress_pct,
          eta_seconds: snap.eta_seconds,
          peers: snap.peers,
          status: (snap.status as any) || job.status
        };
      }
      return job;
    });

    if (modified) {
      this.jobs = nextJobs;
    }
  }

  async addTorrent(magnet_uri: string, nyaa_id?: number | null) {
    const job = await api.downloader.add({ magnet_uri, nyaa_id });
    const idx = this.jobs.findIndex(j => j.id === job.id || j.info_hash === job.info_hash);
    if (idx >= 0) {
      this.jobs[idx] = job;
    } else {
      this.jobs.unshift(job);
    }
    return job;
  }

  async pauseTorrent(id: string) {
    await api.downloader.pause(id);
    const job = this.jobs.find(j => j.id === id);
    if (job) {
      job.status = 'PAUSED';
      job.download_speed = 0;
      job.upload_speed = 0;
    }
  }

  async resumeTorrent(id: string) {
    await api.downloader.resume(id);
    const job = this.jobs.find(j => j.id === id);
    if (job) {
      job.status = 'DOWNLOADING';
    }
  }

  async deleteTorrent(id: string, deleteFiles = false) {
    await api.downloader.delete(id, deleteFiles);
    this.jobs = this.jobs.filter(j => j.id !== id);
  }
}

export const torrents = new TorrentStore();
