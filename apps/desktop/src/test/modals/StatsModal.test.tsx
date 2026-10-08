import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StatsModal } from '../../components/modals/StatsModal';
import { useUIStore } from '../../stores/uiStore';
import { TooltipProvider } from '../../components/ui/tooltip';
import type { StatsResponse } from '../../types/stats';

const mockSampleStats: StatsResponse = {
  total_documents: 1420,
  total_size_bytes: 52428800, // 50 MB
  indexed_folders: 4,
  types: {
    code: 950,
    markdown: 350,
    config: 120,
  },
  languages: {
    rust: 600,
    typescript: 350,
    markdown: 350,
    yaml: 120,
  },
};

const mockEmptyStats: StatsResponse = {
  total_documents: 0,
  total_size_bytes: 0,
  indexed_folders: 0,
  types: {},
  languages: {},
};

function renderStatsModal() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <StatsModal />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('StatsModal (7.15 Stats)', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.setState({
      statsModalOpen: true,
      folderModalOpen: false,
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders loading skeleton while fetching statistics', () => {
    // Return unresolved promise to simulate loading
    globalThis.fetch = vi.fn().mockImplementation(() => new Promise(() => {}));

    renderStatsModal();

    expect(screen.getByText('Index Statistics')).toBeDefined();
    expect(screen.getByTestId('stats-loading')).toBeDefined();
  });

  it('renders error state and retry button on fetch error', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Koneksi terputus ke server'));

    renderStatsModal();

    await waitFor(() => {
      expect(screen.getByTestId('stats-error')).toBeDefined();
    });

    expect(screen.getByText('Gagal Mengambil Statistik')).toBeDefined();
    expect(screen.getByText(/Koneksi terputus/i)).toBeDefined();

    const retryButton = screen.getByRole('button', { name: /Coba Lagi/i });
    expect(retryButton).toBeDefined();

    // Clicking retry calls fetch again
    fireEvent.click(retryButton);
    expect(globalThis.fetch).toHaveBeenCalled();
  });

  it('renders empty state when total documents is zero', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockEmptyStats), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderStatsModal();

    await waitFor(() => {
      expect(screen.getByTestId('stats-empty')).toBeDefined();
    });

    expect(screen.getByText('Belum Ada Dokumen Terindeks')).toBeDefined();

    const folderButton = screen.getByRole('button', { name: /Buka Folder Manager/i });
    expect(folderButton).toBeDefined();

    // Clicking button closes stats modal and opens folder modal
    fireEvent.click(folderButton);
    expect(useUIStore.getState().statsModalOpen).toBe(false);
    expect(useUIStore.getState().folderModalOpen).toBe(true);
  });

  it('renders complete metrics, type and language distributions in success state', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockSampleStats), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderStatsModal();

    await waitFor(() => {
      expect(screen.getByTestId('stats-content')).toBeDefined();
    });

    // Metric cards
    expect(screen.getByText('1,420')).toBeDefined(); // total_documents
    expect(screen.getByText('50.0 MB')).toBeDefined(); // total_size_bytes
    expect(screen.getByText('4')).toBeDefined(); // indexed_folders

    // Type distribution
    expect(screen.getByText('Distribusi Tipe Dokumen')).toBeDefined();
    expect(screen.getByText('code')).toBeDefined();
    expect(screen.getAllByText('markdown').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('config')).toBeDefined();

    // Language distribution
    expect(screen.getByText('Distribusi Bahasa Pemrograman')).toBeDefined();
    expect(screen.getByText('rust')).toBeDefined();
    expect(screen.getByText('typescript')).toBeDefined();

    // Close button
    const closeBtn = screen.getByRole('button', { name: /Tutup/i });
    fireEvent.click(closeBtn);
    expect(useUIStore.getState().statsModalOpen).toBe(false);
  });
});
