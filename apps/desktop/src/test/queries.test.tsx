import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  useHealthQuery,
  useSearchQuery,
  useSuggestQuery,
  useFoldersQuery,
  useRegisterFolderMutation,
  useJobQuery,
  useCancelJobMutation,
  useDocumentDetailQuery,
  useSettingsQuery,
  useStatsQuery,
  useDebounce,
} from '../hooks';

function createTestWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });

  return {
    queryClient,
    wrapper: ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
  };
}

describe('TanStack Query Hooks', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('useHealthQuery fetches and returns health data', async () => {
    const mockHealth = {
      status: 'ok',
      version: '1.0.0',
      timestamp: '2026-10-05T12:00:00Z',
      database: { status: 'up', latency_ms: 1 },
      elasticsearch: { status: 'up', latency_ms: 2 },
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockHealth), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(() => useHealthQuery({ refetchInterval: false }), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.status).toBe('ok');
    expect(result.current.data?.version).toBe('1.0.0');
  });

  it('useSearchQuery fetches results when search query is provided', async () => {
    const mockSearch = {
      query: 'auth',
      page: 1,
      size: 20,
      total: 1,
      took_ms: 10,
      items: [
        {
          id: '550e8400-e29b-41d4-a716-446655440000',
          title: 'Auth Doc',
          relative_path: 'src/auth.rs',
          type: 'code',
          score: 1.0,
          file_size: 100,
          tags: [],
          highlights: [],
        },
      ],
      results: [],
      facets: { types: [], languages: [], tags: [], projects: [] },
      warnings: [],
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockSearch), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(() => useSearchQuery({ q: 'auth' }), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.total).toBe(1);
    expect(result.current.data?.items[0].title).toBe('Auth Doc');
  });

  it('useSuggestQuery fetches suggestions for query term', async () => {
    const mockSuggest = { suggestions: ['authentication', 'authorization'] };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockSuggest), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(() => useSuggestQuery({ q: 'auth' }), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.suggestions).toEqual(['authentication', 'authorization']);
  });

  it('useFoldersQuery and useRegisterFolderMutation work seamlessly', async () => {
    const mockFolders = [
      {
        id: '550e8400-e29b-41d4-a716-446655440000',
        root_path: '/projects/repo',
        status: 'IDLE',
        document_count: 10,
        created_at: '2026-10-01T00:00:00Z',
      },
    ];

    const mockJob = {
      job_id: '550e8400-e29b-41d4-a716-446655440001',
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      message: 'Indexing started',
    };

    globalThis.fetch = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/api/folders')) {
        return Promise.resolve(
          new Response(JSON.stringify(mockFolders), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockJob), {
          status: 202,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    const { wrapper } = createTestWrapper();
    const { result: folderResult } = renderHook(() => useFoldersQuery(), { wrapper });

    await waitFor(() => expect(folderResult.current.isSuccess).toBe(true));
    expect(folderResult.current.data).toHaveLength(1);

    const { result: mutationResult } = renderHook(() => useRegisterFolderMutation(), { wrapper });
    mutationResult.current.mutate({ root_path: '/projects/new' });

    await waitFor(() => expect(mutationResult.current.isSuccess).toBe(true));
    expect(mutationResult.current.data?.status).toBe('RUNNING');
  });

  it('useFoldersQuery dynamically sets refetchInterval to 1500ms when any folder is SCANNING', async () => {
    const scanningFolders = [
      {
        id: '550e8400-e29b-41d4-a716-446655440000',
        root_path: '/projects/repo',
        status: 'SCANNING',
        document_count: 10,
        created_at: '2026-10-01T00:00:00Z',
      },
    ];

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(scanningFolders), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(() => useFoldersQuery(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.[0].status).toBe('SCANNING');
  });

  it('useJobQuery fetches job status and computes polling interval', async () => {
    const mockJob = {
      job_id: '550e8400-e29b-41d4-a716-446655440001',
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      processed_files: 50,
      indexed_files: 40,
      skipped_files: 10,
      failed_files: 0,
      total_files: 100,
      started_at: '2026-10-05T12:00:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(
      () => useJobQuery('550e8400-e29b-41d4-a716-446655440001'),
      { wrapper },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.processed_files).toBe(50);
    expect(result.current.data?.status).toBe('RUNNING');
  });

  it('useCancelJobMutation cancels job and validates CancelJobResponse', async () => {
    const cancelPayload = {
      job_id: '550e8400-e29b-41d4-a716-446655440001',
      status: 'CANCELLED',
      message: 'Job cancellation signal sent.',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(cancelPayload), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(() => useCancelJobMutation(), { wrapper });

    result.current.mutate('550e8400-e29b-41d4-a716-446655440001');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.status).toBe('CANCELLED');
    expect(result.current.data?.message).toBe('Job cancellation signal sent.');
  });

  it('useDocumentDetailQuery loads document preview content', async () => {
    const mockDoc = {
      id: '990e8400-e29b-41d4-a716-446655440002',
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      folder_root_path: '/projects',
      relative_path: 'main.rs',
      title: 'Main',
      content: 'fn main() {}',
      type: 'code',
      tags: [],
      file_size: 12,
      content_hash: 'hash',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDoc), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { wrapper } = createTestWrapper();
    const { result } = renderHook(
      () => useDocumentDetailQuery('990e8400-e29b-41d4-a716-446655440002'),
      { wrapper },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.content).toBe('fn main() {}');
  });

  it('useSettingsQuery and useStatsQuery load system configuration and statistics', async () => {
    const mockSettings = {
      max_file_size_bytes: 2097152,
      weights: { title: 3.0, tags: 2.0, content: 1.0 },
      ignore_patterns: ['.git'],
    };

    const mockStats = {
      total_documents: 100,
      total_size_bytes: 500000,
      types: { code: 80, doc: 20 },
      languages: { rust: 80 },
      indexed_folders: 1,
    };

    globalThis.fetch = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/api/settings')) {
        return Promise.resolve(
          new Response(JSON.stringify(mockSettings), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockStats), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    const { wrapper } = createTestWrapper();
    const { result: settingsResult } = renderHook(() => useSettingsQuery(), { wrapper });
    const { result: statsResult } = renderHook(() => useStatsQuery(), { wrapper });

    await waitFor(() => expect(settingsResult.current.isSuccess).toBe(true));
    await waitFor(() => expect(statsResult.current.isSuccess).toBe(true));

    expect(settingsResult.current.data?.weights.title).toBe(3.0);
    expect(statsResult.current.data?.total_documents).toBe(100);
  });

  it('useDebounce delays value propagation', () => {
    vi.useFakeTimers();

    const { result, rerender } = renderHook(
      ({ val, delay }: { val: string; delay?: number }) => useDebounce(val, delay),
      { initialProps: { val: 'first', delay: 150 } },
    );

    expect(result.current).toBe('first');

    rerender({ val: 'second', delay: 150 });
    expect(result.current).toBe('first');

    act(() => {
      vi.advanceTimersByTime(149);
    });
    expect(result.current).toBe('first');

    act(() => {
      vi.advanceTimersByTime(2);
    });
    expect(result.current).toBe('second');

    vi.useRealTimers();
  });
});
