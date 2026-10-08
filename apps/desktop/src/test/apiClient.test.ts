import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { z } from 'zod';
import {
  createApiClient,
  ApiClientError,
  ApiTimeoutError,
  ApiValidationError,
} from '../lib/api-client';
import {
  fetchHealth,
  fetchFolders,
  registerOrRescanFolder,
  searchDocuments,
} from '../api';

describe('ApiClient & HTTP Layer', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('uses default localhost base URL and allows custom override', () => {
    const client = createApiClient();
    expect(client.getBaseUrl()).toBe('http://127.0.0.1:3001');

    client.setBaseUrl('http://127.0.0.1:4000/');
    expect(client.getBaseUrl()).toBe('http://127.0.0.1:4000');
  });

  it('performs successful GET with query parameters and schema parsing', async () => {
    const mockData = { status: 'alive' };
    const schema = z.object({ status: z.string() });

    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockData), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );
    globalThis.fetch = fetchMock;

    const client = createApiClient({ baseUrl: 'http://127.0.0.1:3001' });
    const result = await client.get('/api/health/live', {
      params: { check: 'quick' },
      schema,
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const calledUrl = fetchMock.mock.calls[0][0];
    expect(calledUrl).toBe('http://127.0.0.1:3001/api/health/live?check=quick');
    expect(result).toEqual(mockData);
  });

  it('performs successful POST with serialized JSON body', async () => {
    const reqBody = { root_path: '/my/path' };
    const resBody = {
      job_id: '550e8400-e29b-41d4-a716-446655440001',
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      message: 'Started',
    };

    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(resBody), {
        status: 202,
        headers: { 'Content-Type': 'application/json' },
      }),
    );
    globalThis.fetch = fetchMock;

    const client = createApiClient();
    const result = await client.post('/api/index/folder', reqBody);

    const callArgs = fetchMock.mock.calls[0];
    expect(callArgs[0]).toBe('http://127.0.0.1:3001/api/index/folder');
    expect(callArgs[1].method).toBe('POST');
    expect(callArgs[1].body).toBe(JSON.stringify(reqBody));
    expect(callArgs[1].headers['Content-Type']).toBe('application/json');
    expect(result).toEqual(resBody);
  });

  it('throws ApiValidationError when response does not match schema', async () => {
    const invalidData = { status: 123 }; // status should be string
    const schema = z.object({ status: z.string() });

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(invalidData), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const client = createApiClient();
    await expect(
      client.get('/api/health', { schema }),
    ).rejects.toThrow(ApiValidationError);
  });

  it('parses structured backend error responses and throws ApiClientError', async () => {
    const errorPayload = {
      code: 'FOLDER_NOT_FOUND',
      message: 'Folder not found',
      details: { folder_id: 'abc' },
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(errorPayload), {
        status: 404,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const client = createApiClient();

    try {
      await client.get('/api/folders/unknown');
      expect.fail('Should have thrown ApiClientError');
    } catch (err: unknown) {
      expect(err).toBeInstanceOf(ApiClientError);
      const apiErr = err as ApiClientError;
      expect(apiErr.status).toBe(404);
      expect(apiErr.code).toBe('FOLDER_NOT_FOUND');
      expect(apiErr.message).toBe('Folder not found');
      expect(apiErr.details).toEqual({ folder_id: 'abc' });
    }
  });

  it('handles non-JSON error responses gracefully', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response('Internal Server Error', {
        status: 500,
        statusText: 'Internal Server Error',
      }),
    );

    const client = createApiClient();

    try {
      await client.get('/api/error');
      expect.fail('Should have thrown ApiClientError');
    } catch (err: unknown) {
      expect(err).toBeInstanceOf(ApiClientError);
      const apiErr = err as ApiClientError;
      expect(apiErr.status).toBe(500);
      expect(apiErr.code).toBe('UNKNOWN_ERROR');
      expect(apiErr.message).toBe('Internal Server Error');
    }
  });

  it('throws ApiTimeoutError when request exceeds timeout limit', async () => {
    vi.useFakeTimers();

    globalThis.fetch = vi.fn().mockImplementation((_url, options) => {
      return new Promise((_resolve, reject) => {
        options.signal.addEventListener('abort', () => {
          reject(new DOMException('The operation was aborted', 'AbortError'));
        });
      });
    });

    const client = createApiClient();
    const reqPromise = client.get('/api/slow', { timeoutMs: 50 });

    vi.advanceTimersByTime(60);

    await expect(reqPromise).rejects.toThrow(ApiTimeoutError);

    vi.useRealTimers();
  });

  it('respects external AbortSignal cancellation', async () => {
    const controller = new AbortController();

    globalThis.fetch = vi.fn().mockImplementation((_url, options) => {
      return new Promise((_resolve, reject) => {
        options.signal.addEventListener('abort', () => {
          reject(new DOMException('User cancelled', 'AbortError'));
        });
      });
    });

    const client = createApiClient();
    const reqPromise = client.get('/api/search', { signal: controller.signal });

    controller.abort(new DOMException('User cancelled', 'AbortError'));

    await expect(reqPromise).rejects.toThrow();
  });

  it('executes typed API functions successfully', async () => {
    const healthData = {
      status: 'ok',
      version: '1.0.0',
      timestamp: '2026-10-05T12:00:00Z',
      database: { status: 'up', latency_ms: 2 },
      elasticsearch: { status: 'up', latency_ms: 4 },
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(healthData), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const health = await fetchHealth();
    expect(health.status).toBe('ok');
    expect(health.version).toBe('1.0.0');

    // Folders
    const folderData = [
      {
        id: '550e8400-e29b-41d4-a716-446655440000',
        root_path: '/projects/repo',
        status: 'IDLE',
        document_count: 5,
        created_at: '2026-10-01T00:00:00Z',
      },
    ];
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(folderData), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const folders = await fetchFolders();
    expect(folders).toHaveLength(1);
    expect(folders[0].root_path).toBe('/projects/repo');

    // Register folder
    const indexFolderData = {
      job_id: '550e8400-e29b-41d4-a716-446655440001',
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      message: 'Indexing started',
    };
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(indexFolderData), {
        status: 202,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const job = await registerOrRescanFolder({ root_path: '/projects/repo' });
    expect(job.status).toBe('RUNNING');

    // Search documents
    const searchData = {
      query: 'test',
      page: 1,
      size: 20,
      total: 0,
      took_ms: 5,
      items: [],
      results: [],
      facets: { types: [], languages: [], tags: [], projects: [] },
      warnings: [],
    };
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(searchData), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const searchRes = await searchDocuments({ q: 'test' });
    expect(searchRes.query).toBe('test');
    expect(searchRes.total).toBe(0);
  });
});
