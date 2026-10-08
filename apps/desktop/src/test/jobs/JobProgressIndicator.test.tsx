import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TooltipProvider } from '../../components/ui/tooltip';
import { JobProgressIndicator } from '../../components/jobs/JobProgressIndicator';
import { useUIStore } from '../../stores/uiStore';
import type { JobStatusResponse } from '../../types/job';

function createTestWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={0}>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

describe('JobProgressIndicator', () => {
  const sampleJob: JobStatusResponse = {
    job_id: '550e8400-e29b-41d4-a716-446655440001',
    folder_id: '550e8400-e29b-41d4-a716-446655440000',
    status: 'RUNNING',
    processed_files: 45,
    indexed_files: 40,
    skipped_files: 5,
    failed_files: 0,
    total_files: 100,
    started_at: '2026-10-06T10:00:00Z',
    finished_at: null,
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    useUIStore.setState({
      indexingJobId: null,
      indexingJobIds: [],
      jobDrawerExpanded: false,
    });
  });

  it('renders nothing when indexingJobId is null', () => {
    const { container } = render(<JobProgressIndicator />, {
      wrapper: createTestWrapper(),
    });
    expect(container.firstChild).toBeNull();
  });

  it('renders progress bar, processed files, skipped files, and cancel button when job is running', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(sampleJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({ indexingJobId: sampleJob.job_id });

    render(<JobProgressIndicator />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByText('RUNNING')).toBeDefined();
    });

    expect(screen.getByText('45/100')).toBeDefined();
    expect(screen.getByText('Skip: 5')).toBeDefined();
    expect(screen.getByText('Fail: 0')).toBeDefined();
    expect(screen.getByLabelText('Batalkan pekerjaan indeks')).toBeDefined();

    const progressBar = screen.getByRole('progressbar');
    expect(progressBar.getAttribute('aria-valuenow')).toBe('45');
    expect(progressBar.getAttribute('aria-valuemax')).toBe('100');
  });

  it('triggers cancel mutation with lightweight cancel response without validation error', async () => {
    const cancelResponse = {
      job_id: sampleJob.job_id,
      status: 'CANCELLED',
      message: 'Job cancellation signal sent.',
    };

    globalThis.fetch = vi.fn().mockImplementation((_url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST') {
        return Promise.resolve(
          new Response(JSON.stringify(cancelResponse), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(sampleJob), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    useUIStore.setState({ indexingJobId: sampleJob.job_id, indexingJobIds: [sampleJob.job_id] });

    render(<JobProgressIndicator />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByLabelText('Batalkan pekerjaan indeks')).toBeDefined();
    });

    const cancelButton = screen.getByLabelText('Batalkan pekerjaan indeks');
    fireEvent.click(cancelButton);

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        expect.stringContaining(`/api/index/jobs/${sampleJob.job_id}/cancel`),
        expect.anything(),
      );
    });
  });

  it('renders multi-job tab switcher when multiple jobs are active and allows switching', async () => {
    const secondJobId = '550e8400-e29b-41d4-a716-446655440002';

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(sampleJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({
      indexingJobId: sampleJob.job_id,
      indexingJobIds: [sampleJob.job_id, secondJobId],
    });

    render(<JobProgressIndicator />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByText(/Pekerjaan \(2\):/i)).toBeDefined();
    });

    expect(screen.getByText('Job #1')).toBeDefined();
    expect(screen.getByText('Job #2')).toBeDefined();

    const job2Tab = screen.getByLabelText(`Pilih pekerjaan ${secondJobId}`);
    fireEvent.click(job2Tab);

    expect(useUIStore.getState().indexingJobId).toBe(secondJobId);
  });

  it('toggles collapsible details drawer when chevron button is clicked', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(sampleJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({ indexingJobId: sampleJob.job_id, jobDrawerExpanded: false });

    render(<JobProgressIndicator />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByText('RUNNING')).toBeDefined();
    });

    expect(screen.queryByText('Rincian Pekerjaan Indeks')).toBeNull();

    const toggleButton = screen.getByLabelText('Buka rincian pekerjaan');
    fireEvent.click(toggleButton);

    expect(useUIStore.getState().jobDrawerExpanded).toBe(true);
    expect(screen.getByText('Rincian Pekerjaan Indeks')).toBeDefined();
    expect(screen.getByText('Total Berkas')).toBeDefined();
    expect(screen.getByText('Berhasil Diindeks')).toBeDefined();
  });

  it('displays terminal completed summary with dismiss button and clears indexingJobId on dismiss', async () => {
    const completedJob: JobStatusResponse = {
      ...sampleJob,
      status: 'COMPLETED',
      processed_files: 100,
      indexed_files: 90,
      skipped_files: 10,
      failed_files: 0,
      finished_at: '2026-10-06T10:01:15Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(completedJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({ indexingJobId: completedJob.job_id });

    render(<JobProgressIndicator autoDismissMs={100_000} />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByText('COMPLETED')).toBeDefined();
    });

    expect(screen.getByText('Pemindaian Indeks Selesai')).toBeDefined();
    expect(screen.getByText('100/100')).toBeDefined();
    expect(screen.queryByLabelText('Batalkan pekerjaan indeks')).toBeNull();

    const dismissButton = screen.getByLabelText('Tutup ringkasan pekerjaan');
    expect(dismissButton).toBeDefined();

    fireEvent.click(dismissButton);
    expect(useUIStore.getState().indexingJobId).toBeNull();
  });

  it('displays failed job state with error details in drawer', async () => {
    const failedJob: JobStatusResponse = {
      ...sampleJob,
      status: 'FAILED',
      error: 'Disk I/O failure while reading file',
      failed_files: 3,
      finished_at: '2026-10-06T10:01:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(failedJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({
      indexingJobId: failedJob.job_id,
      jobDrawerExpanded: true,
    });

    render(<JobProgressIndicator autoDismissMs={100_000} />, {
      wrapper: createTestWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByText('FAILED')).toBeDefined();
    });

    expect(screen.getByText('Pemindaian Indeks Gagal')).toBeDefined();
    expect(screen.getByText('Disk I/O failure while reading file')).toBeDefined();
    expect(screen.getByText('Fail: 3')).toBeDefined();
  });

  it('auto-dismisses terminal state job after autoDismissMs', async () => {
    vi.useFakeTimers();

    const completedJob: JobStatusResponse = {
      ...sampleJob,
      status: 'COMPLETED',
      finished_at: '2026-10-06T10:01:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(completedJob), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useUIStore.setState({ indexingJobId: completedJob.job_id, jobDrawerExpanded: false });

    render(<JobProgressIndicator autoDismissMs={3000} />, {
      wrapper: createTestWrapper(),
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(useUIStore.getState().indexingJobId).toBe(completedJob.job_id);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3500);
    });

    expect(useUIStore.getState().indexingJobId).toBeNull();

    vi.useRealTimers();
  });
});
