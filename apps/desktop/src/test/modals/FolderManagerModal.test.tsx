import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FolderManagerModal } from '../../components/modals/FolderManagerModal';
import { useUIStore } from '../../stores/uiStore';
import * as desktopBridge from '../../lib/desktop-bridge';

import { TooltipProvider } from '../../components/ui/tooltip';

function renderFolderManager() {
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

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <FolderManagerModal />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('FolderManagerModal', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
    useUIStore.setState({
      folderModalOpen: true,
      indexingJobId: null,
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders empty state when no folders are registered', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByText('Folder Manager')).toBeDefined();
      expect(screen.getByText('Belum ada folder yang terdaftar')).toBeDefined();
      expect(screen.getByText('Pilih Folder Lokal')).toBeDefined();
    });
  });

  it('renders folders list with path, document count, and status badges', async () => {
    const mockFolders = [
      {
        id: '110e8400-e29b-41d4-a716-446655440001',
        root_path: '/home/user/docs',
        status: 'IDLE',
        document_count: 142,
        last_scanned_at: '2026-10-05T12:00:00Z',
        created_at: '2026-10-01T10:00:00Z',
      },
      {
        id: '110e8400-e29b-41d4-a716-446655440002',
        root_path: '/home/user/code/rust',
        status: 'SCANNING',
        document_count: 850,
        last_scanned_at: null,
        created_at: '2026-10-02T10:00:00Z',
      },
      {
        id: '110e8400-e29b-41d4-a716-446655440003',
        root_path: '/home/user/invalid',
        status: 'ERROR',
        document_count: 0,
        last_scanned_at: '2026-10-03T10:00:00Z',
        created_at: '2026-10-03T10:00:00Z',
      },
    ];

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockFolders), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByText('/home/user/docs')).toBeDefined();
      expect(screen.getByText('/home/user/code/rust')).toBeDefined();
      expect(screen.getByText('/home/user/invalid')).toBeDefined();

      // Counts
      expect(screen.getByText('142')).toBeDefined();
      expect(screen.getByText('850')).toBeDefined();

      // Badges
      expect(screen.getByText('IDLE')).toBeDefined();
      expect(screen.getByText('SCANNING')).toBeDefined();
      expect(screen.getByText('ERROR')).toBeDefined();
    });
  });

  it('triggers + Tambah Folder via directory picker and registers new folder', async () => {
    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST' && url.includes('/api/index/folder')) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              job_id: '990e8400-e29b-41d4-a716-446655440099',
              folder_id: '110e8400-e29b-41d4-a716-446655440001',
              status: 'RUNNING',
              message: 'Indexing started',
            }),
            { status: 202, headers: { 'Content-Type': 'application/json' } },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    vi.spyOn(desktopBridge, 'pickDirectory').mockResolvedValue('/home/user/new-project');

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByText('Tambah Folder')).toBeDefined();
    });

    const addBtn = screen.getByText('Tambah Folder');
    fireEvent.click(addBtn);

    await waitFor(() => {
      expect(useUIStore.getState().indexingJobId).toBe('990e8400-e29b-41d4-a716-446655440099');
    });
  });

  it('opens confirmation modal and executes folder deletion', async () => {
    const mockFolders = [
      {
        id: '110e8400-e29b-41d4-a716-446655440001',
        root_path: '/home/user/docs',
        status: 'IDLE',
        document_count: 142,
        last_scanned_at: '2026-10-05T12:00:00Z',
        created_at: '2026-10-01T10:00:00Z',
      },
    ];

    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'DELETE' && url.includes('/api/folders/')) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              success: true,
              folder_id: '110e8400-e29b-41d4-a716-446655440001',
              deleted_documents: 142,
            }),
            { status: 200, headers: { 'Content-Type': 'application/json' } },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockFolders), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByText('/home/user/docs')).toBeDefined();
    });

    const deleteBtn = screen.getByLabelText('Hapus /home/user/docs');
    fireEvent.click(deleteBtn);

    // Confirmation dialog is shown
    await waitFor(() => {
      expect(screen.getByText('Hapus Folder dari Indeks?')).toBeDefined();
      expect(screen.getByText('Hapus Folder')).toBeDefined();
    });

    const confirmDeleteBtn = screen.getByRole('button', { name: /Hapus Folder/i });
    fireEvent.click(confirmDeleteBtn);

    await waitFor(() => {
      // Confirmation dialog closes after deletion
      expect(screen.queryByText('Hapus Folder dari Indeks?')).toBeNull();
    });
  });

  it('opens strong warning modal and executes index rebuild', async () => {
    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST' && url.includes('/api/index/rebuild')) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              job_id: '770e8400-e29b-41d4-a716-446655440077',
              target_index: 'lynxsearch_docs_v2',
              status: 'RUNNING',
              message: 'Rebuild started',
            }),
            { status: 202, headers: { 'Content-Type': 'application/json' } },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Rebuild Index/i })).toBeDefined();
    });

    const rebuildBtn = screen.getByRole('button', { name: /Rebuild Index/i });
    fireEvent.click(rebuildBtn);

    // Strong warning dialog is shown
    await waitFor(() => {
      expect(screen.getByText('Peringatan Keras: Bangun Ulang Seluruh Indeks?')).toBeDefined();
      expect(screen.getByRole('button', { name: /Ya, Bangun Ulang Seluruh Indeks/i })).toBeDefined();
    });

    const confirmRebuildBtn = screen.getByRole('button', { name: /Ya, Bangun Ulang Seluruh Indeks/i });
    fireEvent.click(confirmRebuildBtn);

    await waitFor(() => {
      expect(useUIStore.getState().indexingJobId).toBe('770e8400-e29b-41d4-a716-446655440077');
    });
  });

  it('triggers Pindai Semua (Re-scan All) concurrently for all folders', async () => {
    const mockFolders = [
      {
        id: '110e8400-e29b-41d4-a716-446655440001',
        root_path: '/home/user/docs',
        status: 'IDLE',
        document_count: 50,
        last_scanned_at: '2026-10-05T12:00:00Z',
        created_at: '2026-10-01T10:00:00Z',
      },
      {
        id: '220e8400-e29b-41d4-a716-446655440002',
        root_path: '/home/user/code',
        status: 'IDLE',
        document_count: 120,
        last_scanned_at: '2026-10-05T12:00:00Z',
        created_at: '2026-10-01T10:00:00Z',
      },
    ];

    const calls: unknown[] = [];
    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST' && url.includes('/api/index/folder')) {
        calls.push(opts.body ? JSON.parse(opts.body as string) : {});
        return Promise.resolve(
          new Response(
            JSON.stringify({
              job_id: '880e8400-e29b-41d4-a716-446655440088',
              folder_id: '110e8400-e29b-41d4-a716-446655440001',
              status: 'RUNNING',
              message: 'Scan started',
            }),
            { status: 202, headers: { 'Content-Type': 'application/json' } },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockFolders), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderFolderManager();

    await waitFor(() => {
      expect(screen.getByText('Pindai Semua')).toBeDefined();
    });

    const rescanAllBtn = screen.getByText('Pindai Semua');
    fireEvent.click(rescanAllBtn);

    await waitFor(() => {
      expect(calls.length).toBe(2);
      expect(useUIStore.getState().indexingJobId).toBe('880e8400-e29b-41d4-a716-446655440088');
    });
  });
});
