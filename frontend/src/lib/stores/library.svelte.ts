import { api } from '#lib/api/client';
import type { LibraryFileItem, MoveFileRequest } from '#lib/api/types';

class LibraryStore {
  items = $state<LibraryFileItem[]>([]);
  loading = $state<boolean>(false);
  scanning = $state<boolean>(false);
  organizing = $state<boolean>(false);
  error = $state<string | null>(null);

  // Derived metrics
  totalCount = $derived(this.items.length);
  movedCount = $derived(this.items.filter(i => i.is_moved).length);
  pendingCount = $derived(this.items.filter(i => !i.is_moved).length);
  encodedCount = $derived(this.items.filter(i => i.is_encoded).length);
  totalSizeBytes = $derived(
    this.items.reduce((acc, i) => acc + (i.file_size_bytes || 0), 0)
  );

  async init() {
    await this.fetchFiles();
  }

  async fetchFiles(params?: { is_moved?: boolean; search?: string }) {
    this.loading = true;
    this.error = null;
    try {
      this.items = await api.library.getFiles(params);
    } catch (err: any) {
      this.error = err.message || 'Failed to fetch library files';
      console.warn('Library files fetch error:', err);
    } finally {
      this.loading = false;
    }
  }

  async scan() {
    this.scanning = true;
    this.error = null;
    try {
      await api.library.scan();
      await this.fetchFiles();
    } catch (err: any) {
      this.error = err.message || 'Scan failed';
      throw err;
    } finally {
      this.scanning = false;
    }
  }

  async autoOrganize(fileId?: string, template?: string) {
    this.organizing = true;
    this.error = null;
    try {
      const res = await api.library.autoOrganize({
        file_id: fileId,
        template
      });
      await this.fetchFiles();
      return res;
    } catch (err: any) {
      this.error = err.message || 'Auto organize failed';
      throw err;
    } finally {
      this.organizing = false;
    }
  }

  async moveFile(req: MoveFileRequest) {
    this.loading = true;
    this.error = null;
    try {
      const res = await api.library.move(req);
      await this.fetchFiles();
      return res;
    } catch (err: any) {
      this.error = err.message || 'Manual move failed';
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async deleteFile(id: string) {
    try {
      await api.library.delete(id);
      this.items = this.items.filter(i => i.id !== id);
    } catch (err: any) {
      this.error = err.message || 'Failed to remove file tracking';
      throw err;
    }
  }

  async encodeFile(fileId: string, presetId?: string, targetFilePath?: string) {
    this.error = null;
    try {
      const job = await api.library.encodeFile(fileId, {
        preset_id: presetId,
        target_file_path: targetFilePath
      });
      return job;
    } catch (err: any) {
      this.error = err.message || 'Failed to queue encode job';
      throw err;
    }
  }
}

export const library = new LibraryStore();
