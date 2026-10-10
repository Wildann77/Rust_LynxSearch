import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, within, act, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { WorkspaceShell } from '../components/layout/WorkspaceShell';
import { useUIStore } from '../stores/uiStore';
import { useSearchStore } from '../stores/searchStore';

function renderWorkspace() {
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
      <WorkspaceShell />
    </QueryClientProvider>,
  );
}

describe('WorkspaceShell (3-pane layout)', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          status: 'UP',
          timestamp: '2026-10-06T12:00:00Z',
          checks: {
            elasticsearch: 'UP',
            postgres: 'UP',
          },
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        },
      ),
    );
    useSearchStore.getState().resetSearch();

    useUIStore.setState({
      sidebarCollapsed: false,
      previewCollapsed: false,
      previewWidth: 480,
      theme: 'dark',
      activeTab: 'search',
      indexingJobId: null,
      indexingJobIds: [],
      jobDrawerExpanded: false,
      folderModalOpen: false,
      settingsModalOpen: false,
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders top bar with drag region, branding, search input, and buttons', () => {

    renderWorkspace();

    // TopBar drag region
    const header = document.querySelector('header[data-tauri-drag-region]');
    expect(header).toBeDefined();

    // Brand title
    expect(screen.getByText('LynxSearch')).toBeDefined();

    // Search input
    const searchInput = screen.getByPlaceholderText(/Search code, docs, tags/i);
    expect(searchInput).toBeDefined();

    // Health indicator
    expect(screen.getByLabelText(/Backend Status:/i)).toBeDefined();

    // Action buttons
    expect(screen.getByLabelText('Open Folder Manager')).toBeDefined();
    expect(screen.getByLabelText('Open Settings')).toBeDefined();
  });

  it('renders all three panes in the workspace shell', () => {
    renderWorkspace();

    // Left pane
    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();
    expect(screen.getByText('Filters')).toBeDefined();

    // Center pane
    expect(screen.getByLabelText('Search Results')).toBeDefined();
    expect(screen.getByText(/Knowledge Base & Code Search/i)).toBeDefined();

    // Right pane
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
    expect(screen.getByText('Pilih dokumen untuk melihat preview')).toBeDefined();
  });

  it('toggles facet sidebar via button and shortcut [', () => {
    renderWorkspace();

    // Initially visible
    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();

    // Toggle via topbar button
    const toggleSidebarBtn = screen.getByLabelText('Toggle Facet Sidebar ([)');
    act(() => {
      fireEvent.click(toggleSidebarBtn);
    });
    expect(screen.queryByLabelText('Facet Filters Sidebar')).toBeNull();

    // Toggle via hotkey [
    act(() => {
      fireEvent.keyDown(window, { key: '[' });
    });
    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();
  });

  it('toggles preview pane via button and shortcut ]', () => {
    renderWorkspace();

    // Initially visible
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();

    // Toggle via topbar button
    const togglePreviewBtn = screen.getByLabelText('Toggle Document Preview (])');
    act(() => {
      fireEvent.click(togglePreviewBtn);
    });
    expect(screen.queryByLabelText('Document Preview Panel')).toBeNull();

    // Toggle via hotkey ]
    act(() => {
      fireEvent.keyDown(window, { key: ']' });
    });
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
  });

  it('resizes preview pane between 420px and 640px via divider drag', () => {
    renderWorkspace();

    const separator = screen.getByRole('separator');
    expect(separator).toBeDefined();

    // Initial width is 480px
    expect(useUIStore.getState().previewWidth).toBe(480);

    // Simulate pointer down on separator
    act(() => {
      fireEvent.pointerDown(separator);
    });

    // Simulate pointer move: window.innerWidth is default (1024), clientX = 400 => newWidth = 624 (valid)
    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 400 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(window.innerWidth - 400);

    // Simulate move that exceeds max constraint (e.g. clientX = 100 => width > 640)
    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 100 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(640);

    // Simulate move below min constraint (e.g. clientX = 900 => width < 420)
    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 900 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(420);

    // End drag
    act(() => {
      window.dispatchEvent(new PointerEvent('pointerup'));
    });
  });

  it('focuses search input when Cmd+K or / is pressed', () => {
    renderWorkspace();

    const searchInput = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;

    // Hotkey / focuses input
    fireEvent.keyDown(window, { key: '/' });
    expect(document.activeElement).toBe(searchInput);

    // Unfocus
    searchInput.blur();
    expect(document.activeElement).not.toBe(searchInput);

    // Hotkey Cmd+K focuses input
    fireEvent.keyDown(window, { key: 'k', metaKey: true });
    expect(document.activeElement).toBe(searchInput);
  });

  it('renders bottom status bar with hotkey badges and status', () => {
    renderWorkspace();

    const footer = screen.getByLabelText('Application Status and Hotkeys Bar');
    expect(footer).toBeDefined();
    expect(within(footer).getByText('⌘K')).toBeDefined();
    expect(within(footer).getByText('j/k')).toBeDefined();
  });

  it('opens and closes Folder Manager modal dialog', async () => {
    renderWorkspace();

    expect(screen.queryByRole('dialog')).toBeNull();

    const folderBtn = screen.getByLabelText('Open Folder Manager');
    fireEvent.click(folderBtn);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeDefined();
    expect(await screen.findByText('Folder Manager')).toBeDefined();

    // Escape closes modal
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  it('opens and closes Settings modal dialog', async () => {
    renderWorkspace();

    expect(screen.queryByRole('dialog')).toBeNull();

    const settingsBtn = screen.getByLabelText('Open Settings');
    fireEvent.click(settingsBtn);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeDefined();
    expect(await screen.findByText('Search Settings')).toBeDefined();

    // Escape closes modal
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });
});
