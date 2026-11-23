import { api, getWebSocketUrl } from '#lib/api/client';
import type {
  CreatePresetRequest,
  EncodeProgressTelemetry,
  EncodingJobItem,
  EncodingPreset,
  QueueJobRequest,
} from '#lib/api/types';

class EncoderStore {
  jobs = $state<EncodingJobItem[]>([]);
  presets = $state<EncodingPreset[]>([]);
  loading = $state<boolean>(false);
  activeTelemetry = $state<EncodeProgressTelemetry | null>(null);
  wsConnected = $state<boolean>(false);
  error = $state<string | null>(null);

  private ws: WebSocket | null = null;
  private reconnectTimer: any = null;

  // Derived metrics
  activeJob = $derived(this.jobs.find(j => j.status === 'ENCODING'));
  queuedCount = $derived(this.jobs.filter(j => j.status === 'QUEUED').length);
  completedCount = $derived(this.jobs.filter(j => j.status === 'COMPLETED').length);
  failedCount = $derived(this.jobs.filter(j => j.status === 'FAILED').length);

  totalSavedBytes = $derived(
    this.jobs
      .filter(j => j.status === 'COMPLETED')
      .reduce(
        (acc, j) =>
          acc + Math.max(0, j.source_size_bytes - j.encoded_size_bytes),
        0
      )
  );

  async init() {
    await Promise.all([this.fetchPresets(), this.fetchJobs()]);
    this.connectWs();
  }

  async fetchPresets() {
    try {
      this.presets = await api.encoder.getPresets();
    } catch (err: any) {
      console.warn('Failed to load presets:', err);
    }
  }

  async fetchJobs(status?: string) {
    this.loading = true;
    try {
      this.jobs = await api.encoder.getJobs(status);
    } catch (err: any) {
      this.error = err.message || 'Failed to load jobs';
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
          if (payload.event === 'encode_progress' && payload.data) {
            this.handleTelemetry(payload.data as EncodeProgressTelemetry);
          }
        } catch (e) {
          console.error('Failed parsing WS encode_progress:', e);
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

  private handleTelemetry(telemetry: EncodeProgressTelemetry) {
    this.activeTelemetry = telemetry;

    let modified = false;
    const nextJobs = this.jobs.map(job => {
      if (job.id === telemetry.job_id) {
        modified = true;
        return {
          ...job,
          status: telemetry.status as any,
          progress_pct: telemetry.progress_pct,
          current_frame: telemetry.current_frame,
          fps: telemetry.fps,
          speed: telemetry.speed,
          current_time_sec: telemetry.current_time_sec,
          total_duration_sec: telemetry.total_duration_sec,
          eta_seconds: telemetry.eta_seconds,
          encoded_size_bytes: telemetry.current_size_bytes,
        };
      }
      return job;
    });

    if (modified) {
      this.jobs = nextJobs;
      if (telemetry.status === 'FAILED' || telemetry.status === 'COMPLETED') {
        this.fetchJobs();
      }
    } else if (telemetry.status === 'ENCODING' || telemetry.status === 'FAILED' || telemetry.status === 'COMPLETED') {
      // Re-fetch to ensure active job list is synced
      this.fetchJobs();
    }
  }

  async queueJob(req: QueueJobRequest) {
    const job = await api.encoder.queueJob(req);
    await this.fetchJobs();
    return job;
  }

  async createPreset(req: CreatePresetRequest) {
    const preset = await api.encoder.createPreset(req);
    await this.fetchPresets();
    return preset;
  }

  async cancelJob(id: string) {
    await api.encoder.cancelJob(id);
    const job = this.jobs.find(j => j.id === id);
    if (job) {
      job.status = 'CANCELLED';
    }
  }

  async deleteJob(id: string) {
    await api.encoder.deleteJob(id);
    this.jobs = this.jobs.filter(j => j.id !== id);
    if (this.activeTelemetry?.job_id === id) {
      this.activeTelemetry = null;
    }
  }
}

export const encoder = new EncoderStore();
