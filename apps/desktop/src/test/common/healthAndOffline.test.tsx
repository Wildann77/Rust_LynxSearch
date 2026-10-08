import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { HealthDot } from '../../components/common/HealthDot';
import { ConnectionBanner } from '../../components/common/ConnectionBanner';
import { OfflineFallback } from '../../components/common/OfflineFallback';
import { TooltipProvider } from '../../components/ui/tooltip';
import { useUIStore } from '../../stores/uiStore';
import type { HealthSummaryResponse } from '../../types/health';

const mockHealthyResponse: HealthSummaryResponse = {
  status: 'ok',
  version: '0.1.0',
  timestamp: '2026-10-07T12:00:00Z',
  database: {
    status: 'ok',
    latency_ms: 1.2,
  },
  elasticsearch: {
    status: 'ok',
    latency_ms: 2.5,
  },
};

function renderWithClient(ui: React.ReactElement) {
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
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('Health and Offline (7.16 Health and offline)', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.setState({
      indexingJobId: null,
      indexingJobIds: [],
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  describe('HealthDot', () => {
    it('shows green dot when backend is healthy', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(
        new Response(JSON.stringify(mockHealthyResponse), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );

      renderWithClient(<HealthDot />);

      await waitFor(() => {
        const dot = screen.getByTestId('health-dot-indicator');
        expect(dot.className).toContain('bg-[#10a37f]'); // Green
      });

      const button = screen.getByTestId('health-dot-button');
      expect(button.getAttribute('data-status')).toBe('healthy');
      expect(screen.getByText('Ready')).toBeDefined();
    });

    it('shows red dot when backend is offline/error', async () => {
      globalThis.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

      renderWithClient(<HealthDot retry={false} />);

      await waitFor(() => {
        const dot = screen.getByTestId('health-dot-indicator');
        expect(dot.className).toContain('bg-destructive'); // Red
      });

      const button = screen.getByTestId('health-dot-button');
      expect(button.getAttribute('data-status')).toBe('offline');
      expect(screen.getByText('Offline')).toBeDefined();
    });

    it('shows yellow dot when an indexing job is running', async () => {
      const mockJobId = '123e4567-e89b-12d3-a456-426614174000';
      const mockFolderId = '123e4567-e89b-12d3-a456-426614174001';
      useUIStore.setState({ indexingJobId: mockJobId });

      globalThis.fetch = vi.fn().mockImplementation((url: string | URL) => {
        const urlStr = url.toString();
        if (urlStr.includes('/api/health')) {
          return Promise.resolve(
            new Response(JSON.stringify(mockHealthyResponse), {
              status: 200,
              headers: { 'Content-Type': 'application/json' },
            }),
          );
        }
        if (urlStr.includes(`/api/index/jobs/${mockJobId}`)) {
          return Promise.resolve(
            new Response(
              JSON.stringify({
                job_id: mockJobId,
                folder_id: mockFolderId,
                status: 'RUNNING',
                total_files: 50,
                processed_files: 20,
                indexed_files: 20,
                skipped_files: 0,
                failed_files: 0,
                started_at: '2026-10-07T12:00:00Z',
              }),
              {
                status: 200,
                headers: { 'Content-Type': 'application/json' },
              },
            ),
          );
        }
        return Promise.reject(new Error('Not found'));
      });

      renderWithClient(<HealthDot />);

      await waitFor(() => {
        const dot = screen.getByTestId('health-dot-indicator');
        expect(dot.className).toContain('bg-amber-400'); // Yellow
        const button = screen.getByTestId('health-dot-button');
        expect(button.getAttribute('data-status')).toBe('indexing');
        expect(screen.getByText('Indexing...')).toBeDefined();
      });
    });
  });

  describe('ConnectionBanner', () => {
    it('is hidden when backend is healthy', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(
        new Response(JSON.stringify(mockHealthyResponse), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );

      renderWithClient(<ConnectionBanner retry={false} />);

      await waitFor(() => {
        expect(screen.queryByTestId('connection-banner')).toBeNull();
      });
    });

    it('displays outage banner when backend is unreachable and triggers retry', async () => {
      globalThis.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));
      const onRetry = vi.fn();

      renderWithClient(<ConnectionBanner onRetry={onRetry} retry={false} />);

      await waitFor(() => {
        expect(screen.getByTestId('connection-banner')).toBeDefined();
      });

      expect(screen.getByText(/\[OFFLINE\]/i)).toBeDefined();
      expect(screen.getByText(/Koneksi ke backend LynxSearch.*terputus/i)).toBeDefined();

      const retryBtn = screen.getByRole('button', { name: /Hubungkan Ulang/i });
      fireEvent.click(retryBtn);
      expect(onRetry).toHaveBeenCalled();
    });
  });

  describe('OfflineFallback', () => {
    it('renders explanation, instructions, and handles retry callback', () => {
      const handleRetry = vi.fn();

      render(<OfflineFallback onRetry={handleRetry} />);

      expect(screen.getByTestId('offline-fallback')).toBeDefined();
      expect(screen.getByText('Backend Service Tidak Tersedia')).toBeDefined();
      expect(screen.getByText(/docker compose -f docker\/docker-compose\.yml up -d/i)).toBeDefined();
      expect(screen.getByText(/cargo run -p backend/i)).toBeDefined();

      const retryBtn = screen.getByRole('button', { name: /Hubungkan Ulang Sekarang/i });
      fireEvent.click(retryBtn);
      expect(handleRetry).toHaveBeenCalled();
    });
  });
});
