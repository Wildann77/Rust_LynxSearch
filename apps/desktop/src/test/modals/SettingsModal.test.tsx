import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { toast } from 'sonner';
import { SettingsModal } from '../../components/modals/SettingsModal';
import { useUIStore } from '../../stores/uiStore';
import { TooltipProvider } from '../../components/ui/tooltip';
import type { AppSettings } from '../../types/settings';

const mockDefaultSettings: AppSettings = {
  max_file_size_bytes: 2 * 1024 * 1024,
  weights: {
    title: 3.0,
    tags: 2.0,
    content: 1.0,
  },
  ignore_patterns: ['.git', 'node_modules', 'target', 'dist', 'build'],
};

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

function renderSettingsModal() {
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
        <SettingsModal />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('SettingsModal', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.setState({
      settingsModalOpen: true,
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders loading state initially when settings are fetching', () => {
    globalThis.fetch = vi.fn().mockReturnValue(new Promise(() => {}));

    renderSettingsModal();

    expect(screen.getByText('Search Settings')).toBeDefined();
  });

  it('renders error state and allows retry on fetch error', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Koneksi backend terputus'));

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByText('Gagal Memuat Pengaturan')).toBeDefined();
      expect(screen.getByText('Koneksi backend terputus')).toBeDefined();
    });

    const retryBtn = screen.getByRole('button', { name: /Coba Lagi/i });
    expect(retryBtn).toBeDefined();
  });

  it('renders form controls with initial settings values', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDefaultSettings), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByText('Batas Ukuran Berkas Maksimum (MB)')).toBeDefined();
      expect(screen.getByText('Pola yang Diabaikan (Ignore Patterns)')).toBeDefined();
      expect(screen.getByText('Bobot Relevansi BM25')).toBeDefined();
    });

    // Check numeric input value (2 MB)
    const sizeInput = screen.getByRole('spinbutton') as HTMLInputElement;
    expect(sizeInput.value).toBe('2');

    // Check default ignore patterns
    expect(screen.getByText('.git')).toBeDefined();
    expect(screen.getByText('node_modules')).toBeDefined();
    expect(screen.getByText('target')).toBeDefined();
    expect(screen.getByText('dist')).toBeDefined();
    expect(screen.getByText('build')).toBeDefined();

    // Check sliders display labels
    expect(screen.getByText('3.0x')).toBeDefined();
    expect(screen.getByText('2.0x')).toBeDefined();
    expect(screen.getByText('1.0x')).toBeDefined();
  });

  it('adds a new ignore pattern and rejects duplicate', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDefaultSettings), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByText('node_modules')).toBeDefined();
    });

    const patternInput = screen.getByLabelText('Input pola baru');
    const addButton = screen.getByRole('button', { name: /Tambah/i });

    // Try adding duplicate
    fireEvent.change(patternInput, { target: { value: 'node_modules' } });
    fireEvent.click(addButton);

    expect(screen.getByText('Pola "node_modules" sudah terdaftar.')).toBeDefined();

    // Add unique pattern via Enter key
    fireEvent.change(patternInput, { target: { value: '*.log' } });
    fireEvent.keyDown(patternInput, { key: 'Enter', code: 'Enter' });

    expect(screen.getByText('*.log')).toBeDefined();
  });

  it('removes an ignore pattern badge when clicking remove button', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDefaultSettings), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByText('build')).toBeDefined();
    });

    const removeBuildBtn = screen.getByLabelText('Hapus pola build');
    fireEvent.click(removeBuildBtn);

    expect(screen.queryByText('build')).toBeNull();
  });

  it('resets form state to default values on Reset button click', async () => {
    const customSettings: AppSettings = {
      max_file_size_bytes: 10 * 1024 * 1024,
      weights: {
        title: 5.0,
        tags: 4.0,
        content: 2.0,
      },
      ignore_patterns: ['custom_dir'],
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(customSettings), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByText('custom_dir')).toBeDefined();
      expect(screen.getByText('5.0x')).toBeDefined();
    });

    const resetBtn = screen.getByRole('button', { name: /Reset ke Default/i });
    fireEvent.click(resetBtn);

    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('Nilai dikembalikan ke default'),
    );

    // Verify fields reset to defaults
    const sizeInput = screen.getByRole('spinbutton') as HTMLInputElement;
    expect(sizeInput.value).toBe('2');
    expect(screen.getByText('.git')).toBeDefined();
    expect(screen.getByText('3.0x')).toBeDefined();
  });

  it('saves settings to API, shows success toast, and closes modal', async () => {
    let putPayload: string | undefined;

    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'PUT' && url.includes('/api/settings')) {
        putPayload = opts.body as string;
        const updated = JSON.parse(putPayload) as AppSettings;
        return Promise.resolve(
          new Response(JSON.stringify(updated), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockDefaultSettings), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByRole('spinbutton')).toBeDefined();
    });

    // Update size input to 5 MB
    const sizeInput = screen.getByRole('spinbutton');
    fireEvent.change(sizeInput, { target: { value: '5' } });

    // Click Save
    const saveBtn = screen.getByRole('button', { name: /Simpan Pengaturan/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(putPayload).toBeDefined();
      const parsed = JSON.parse(putPayload!);
      expect(parsed.max_file_size_bytes).toBe(5 * 1024 * 1024);
      expect(toast.success).toHaveBeenCalledWith('Pengaturan berhasil disimpan');
      expect(useUIStore.getState().settingsModalOpen).toBe(false);
    });
  });

  it('handles save error with error toast', async () => {
    globalThis.fetch = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'PUT' && url.includes('/api/settings')) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              code: 'INTERNAL_SERVER_ERROR',
              message: 'Server gagal menyimpan data',
            }),
            {
              status: 500,
              headers: { 'Content-Type': 'application/json' },
            },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockDefaultSettings), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByRole('spinbutton')).toBeDefined();
    });

    const saveBtn = screen.getByRole('button', { name: /Simpan Pengaturan/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringContaining('Gagal menyimpan pengaturan: Server gagal menyimpan data'),
      );
    });
  });

  it('renders BM25 weight sliders with accessibility attributes', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDefaultSettings), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    renderSettingsModal();

    await waitFor(() => {
      expect(screen.getByRole('slider', { name: 'Bobot Judul BM25' })).toBeDefined();
      expect(screen.getByRole('slider', { name: 'Bobot Tag BM25' })).toBeDefined();
      expect(screen.getByRole('slider', { name: 'Bobot Isi Dokumen BM25' })).toBeDefined();
    });

    const titleSlider = screen.getByRole('slider', { name: 'Bobot Judul BM25' });
    expect(titleSlider.getAttribute('aria-valuenow')).toBe('3');
  });
});

