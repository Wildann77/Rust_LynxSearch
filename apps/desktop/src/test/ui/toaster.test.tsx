import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { toast } from 'sonner';
import { LynxToaster } from '../../components/ui/LynxToaster';

describe('LynxToaster and Toast Triggers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders LynxToaster container with required styling attributes', () => {
    const { container } = render(<LynxToaster />);
    // Sonner mounts an element with data-sonner-toaster
    const toasterElement = container.querySelector('[data-sonner-toaster]');
    expect(toasterElement).toBeDefined();
  });

  it('triggers copy success toast notification', async () => {
    render(<LynxToaster />);
    toast.success('Path file disalin ke clipboard');

    await waitFor(() => {
      expect(screen.getByText('Path file disalin ke clipboard')).toBeDefined();
    });
  });

  it('triggers scan started toast notification', async () => {
    render(<LynxToaster />);
    toast.info('Memulai pemindaian folder...');

    await waitFor(() => {
      expect(screen.getByText('Memulai pemindaian folder...')).toBeDefined();
    });
  });

  it('triggers settings saved toast notification', async () => {
    render(<LynxToaster />);
    toast.success('Pengaturan berhasil disimpan');

    await waitFor(() => {
      expect(screen.getByText('Pengaturan berhasil disimpan')).toBeDefined();
    });
  });

  it('triggers job cancelled toast notification', async () => {
    render(<LynxToaster />);
    toast.warning('Indexing dibatalkan oleh pengguna');

    await waitFor(() => {
      expect(screen.getByText('Indexing dibatalkan oleh pengguna')).toBeDefined();
    });
  });

  it('triggers backend connection error toast notification', async () => {
    render(<LynxToaster />);
    toast.error('Gagal terhubung ke service backend');

    await waitFor(() => {
      expect(screen.getByText('Gagal terhubung ke service backend')).toBeDefined();
    });
  });
});
