import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { OpenWithModal } from '../../components/modals/OpenWithModal';
import { useUIStore } from '../../stores/uiStore';
import * as desktopBridge from '../../lib/desktop-bridge';

describe('OpenWithModal', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useUIStore.setState({
      openWithModalOpen: false,
      openWithTargetPath: null,
      preferredEditor: null,
    });
  });

  it('does not render when modal is closed', () => {
    render(<OpenWithModal />);
    expect(screen.queryByText('Buka Berkas Dengan...')).toBeNull();
  });

  it('renders editor choices when opened with target path', () => {
    useUIStore.setState({
      openWithModalOpen: true,
      openWithTargetPath: '/path/to/main.rs',
    });

    render(<OpenWithModal />);
    expect(screen.getByText('Buka Berkas Dengan...')).toBeDefined();
    expect(screen.getByText('main.rs')).toBeDefined();
    expect(screen.getByText('Antigravity IDE')).toBeDefined();
    expect(screen.getByText('Visual Studio Code')).toBeDefined();
    expect(screen.getByText('GNOME Text Editor')).toBeDefined();
  });

  it('calls openFileInEditor with selected editor on click', async () => {
    const openSpy = vi.spyOn(desktopBridge, 'openFileInEditor').mockResolvedValue();

    useUIStore.setState({
      openWithModalOpen: true,
      openWithTargetPath: '/path/to/main.rs',
    });

    render(<OpenWithModal />);

    // Select Visual Studio Code
    const vscodeOption = screen.getByText('Visual Studio Code');
    fireEvent.click(vscodeOption);

    const submitBtn = screen.getByRole('button', { name: 'Buka Berkas' });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(openSpy).toHaveBeenCalledWith('/path/to/main.rs', 'code');
      expect(useUIStore.getState().openWithModalOpen).toBe(false);
    });
  });

  it('saves preferredEditor when remember choice is checked', async () => {
    vi.spyOn(desktopBridge, 'openFileInEditor').mockResolvedValue();

    useUIStore.setState({
      openWithModalOpen: true,
      openWithTargetPath: '/path/to/main.rs',
    });

    render(<OpenWithModal />);

    const geditOption = screen.getByText('GNOME Text Editor');
    fireEvent.click(geditOption);

    const rememberCheckbox = screen.getByRole('checkbox');
    expect(rememberCheckbox.getAttribute('data-state')).toBe('checked');

    const submitBtn = screen.getByRole('button', { name: 'Buka Berkas' });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(useUIStore.getState().preferredEditor).toBe('gnome-text-editor');
    });
  });
});
