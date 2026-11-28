import { api, getWebSocketUrl } from '#lib/api/client';
import type {
  QueueTranslationRequest,
  TranslationJobItem,
  TranslationProgressTelemetry,
  TranslatorStatusResponse
} from '#lib/api/types';

export const TARGET_LANGUAGE_OPTIONS = [
  { label: 'Indonesian', code: 'ind' },
  { label: 'English', code: 'eng' },
  { label: 'Japanese', code: 'jpn' },
  { label: 'Malay', code: 'msa' },
  { label: 'Spanish', code: 'spa' },
  { label: 'French', code: 'fre' },
  { label: 'German', code: 'ger' },
  { label: 'Korean', code: 'kor' },
  { label: 'Chinese (Simplified)', code: 'chi' },
  { label: 'Chinese (Traditional)', code: 'chi' },
  { label: 'Arabic', code: 'ara' },
  { label: 'Thai', code: 'tha' },
  { label: 'Vietnamese', code: 'vie' },
  { label: 'Portuguese', code: 'por' }
] as const;

const ACTIVE_STATUSES = new Set(['EXTRACTING', 'TRANSLATING', 'EMBEDDING']);

class TranslatorStore {
  jobs = $state<TranslationJobItem[]>([]);
  status = $state<TranslatorStatusResponse | null>(null);
  loading = $state<boolean>(false);
  activeTelemetry = $state<TranslationProgressTelemetry | null>(null);
  wsConnected = $state<boolean>(false);
  error = $state<string | null>(null);

  private ws: WebSocket | null = null;
  private reconnectTimer: any = null;

  activeJob = $derived(this.jobs.find(j => ACTIVE_STATUSES.has(j.status)));
  queuedCount = $derived(this.jobs.filter(j => j.status === 'QUEUED').length);
  completedCount = $derived(this.jobs.filter(j => j.status === 'COMPLETED').length);
  failedCount = $derived(this.jobs.filter(j => j.status === 'FAILED').length);
  totalLinesTranslated = $derived(
    this.jobs
      .filter(j => j.status === 'COMPLETED')
      .reduce((acc, j) => acc + (j.translated_cues || 0), 0)
  );

  async init() {
    await Promise.all([this.fetchStatus(), this.fetchJobs()]);
    this.connectWs();
  }

  async fetchStatus() {
    try {
      this.status = await api.translator.getStatus();
    } catch (err: any) {
      console.warn('Failed to load translator status:', err);
    }
  }

  async fetchJobs(statusFilter?: string) {
    this.loading = true;
    try {
      this.jobs = await api.translator.getJobs(statusFilter);
    } catch (err: any) {
      this.error = err.message || 'Failed to load translation jobs';
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
          if (payload.event === 'translate_progress' && payload.data) {
            this.handleTelemetry(payload.data as TranslationProgressTelemetry);
          }
        } catch (e) {
          console.error('Failed parsing WS translate_progress:', e);
        }
      };

      this.ws.onclose = () => {
        this.wsConnected = false;
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        this.wsConnected = false;
      };
    } catch {
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

  private handleTelemetry(telemetry: TranslationProgressTelemetry) {
    this.activeTelemetry = telemetry;

    let modified = false;
    const nextJobs = this.jobs.map(job => {
      if (job.id === telemetry.job_id) {
        modified = true;
        return {
          ...job,
          status: telemetry.status,
          progress_pct: telemetry.progress_pct,
          total_cues: telemetry.total_cues,
          translated_cues: telemetry.translated_cues,
          current_batch: telemetry.current_batch,
          total_batches: telemetry.total_batches,
          stage_message: telemetry.stage_message,
          error_message: telemetry.error_message
        };
      }
      return job;
    });

    if (modified) {
      this.jobs = nextJobs;
      if (telemetry.status === 'FAILED' || telemetry.status === 'COMPLETED' || telemetry.status === 'CANCELLED') {
        this.fetchJobs();
      }
    } else if (
      ACTIVE_STATUSES.has(telemetry.status) ||
      telemetry.status === 'FAILED' ||
      telemetry.status === 'COMPLETED'
    ) {
      this.fetchJobs();
    }
  }

  async queueJob(req: QueueTranslationRequest) {
    const job = await api.translator.queueJob(req);
    await this.fetchJobs();
    return job;
  }

  async cancelJob(id: string) {
    await api.translator.cancelJob(id);
    const job = this.jobs.find(j => j.id === id);
    if (job) {
      job.status = 'CANCELLED';
      job.stage_message = 'Cancelled by user';
    }
  }

  async deleteJob(id: string) {
    await api.translator.deleteJob(id);
    this.jobs = this.jobs.filter(j => j.id !== id);
    if (this.activeTelemetry?.job_id === id) {
      this.activeTelemetry = null;
    }
  }
}

export const translator = new TranslatorStore();
