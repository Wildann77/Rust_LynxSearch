import { describe, it, expect, beforeAll, afterEach, afterAll } from 'vitest';
import { server } from './mocks/server';
import {
  fetchHealth,
  fetchHealthLive,
  fetchHealthReady,
  fetchStats,
  fetchFolders,
  registerOrRescanFolder,
  deleteFolder,
  fetchJob,
  cancelJob,
  rebuildIndex,
  fetchDocument,
  deleteDocument,
  searchDocuments,
  suggestQueries,
  fetchSettings,
  updateSettings,
  resetSettings,
} from '../api';
import { DOC_UUID, JOB_UUID } from './mocks/handlers';

describe('MSW API Contracts (Phase 10.8)', () => {
  beforeAll(() => server.listen({ onUnhandledRequest: 'bypass' }));
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());
  it('mocks /api/health and sub-routes', async () => {
    const health = await fetchHealth();
    expect(health.status).toBe('ok');
    expect(health.database.status).toBe('healthy');

    const live = await fetchHealthLive();
    expect(live.status).toBe('ok');

    const ready = await fetchHealthReady();
    expect(ready.status).toBe('ready');
    expect(ready.database?.status).toBe('healthy');
  });

  it('mocks /api/search with valid results, empty, and 500 error', async () => {
    // 1. Success query
    const res = await searchDocuments({ q: 'main' });
    expect(res.total).toBe(1);
    expect(res.items[0].title).toBe('Main Entrypoint');
    expect(res.facets.languages[0].key).toBe('rust');

    // 2. Empty query
    const emptyRes = await searchDocuments({ q: '' });
    expect(emptyRes.total).toBe(0);
    expect(emptyRes.items).toHaveLength(0);

    // 3. 500 error query
    await expect(searchDocuments({ q: 'error_500' })).rejects.toThrow();
  });

  it('mocks /api/suggest endpoint', async () => {
    const suggest = await suggestQueries({ q: 'mai' });
    expect(suggest.suggestions).toContain('main');
  });

  it('mocks /api/folders and /api/index/folder endpoints', async () => {
    const folders = await fetchFolders();
    expect(folders).toHaveLength(1);
    expect(folders[0].root_path).toBe('/home/user/workspace/repo');

    const job = await registerOrRescanFolder({
      folder_id: folders[0].id,
    });
    expect(job.status).toBe('RUNNING');

    const del = await deleteFolder(folders[0].id);
    expect(del.folder_id).toBe(folders[0].id);
  });

  it('mocks job endpoints (status, cancel, rebuild)', async () => {
    const job = await fetchJob(JOB_UUID);
    expect(job.status).toBe('RUNNING');
    expect(job.processed_files).toBe(50);

    const cancel = await cancelJob(job.job_id);
    expect(cancel.status).toBe('CANCELLED');

    const rebuild = await rebuildIndex();
    expect(rebuild.status).toBe('RUNNING');
  });

  it('mocks /api/documents/:id and delete endpoint', async () => {
    const doc = await fetchDocument(DOC_UUID);
    expect(doc.title).toBe('Main Entrypoint');
    expect(doc.language).toBe('rust');

    const del = await deleteDocument(doc.id);
    expect(del.id).toBe(doc.id);
    expect(del.status).toBe('DELETED');
  });

  it('mocks settings endpoints and stats endpoint', async () => {
    const stats = await fetchStats();
    expect(stats.total_documents).toBe(42);

    const settings = await fetchSettings();
    expect(settings.max_file_size_bytes).toBe(2097152);

    const updated = await updateSettings({
      max_file_size_bytes: 4194304,
    });
    expect(updated.max_file_size_bytes).toBe(4194304);

    const reset = await resetSettings();
    expect(reset.weights.title).toBe(3.0);
  });
});
