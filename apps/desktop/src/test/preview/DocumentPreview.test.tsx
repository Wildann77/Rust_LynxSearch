import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { DocumentPreview } from '../../components/preview/DocumentPreview';
import { useSearchStore } from '../../stores/searchStore';
import { useUIStore } from '../../stores/uiStore';
import * as desktopBridge from '../../lib/desktop-bridge';

import { TooltipProvider } from '../../components/ui/tooltip';

function renderPreview(documentId: string, onClose?: () => void) {
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
        <DocumentPreview documentId={documentId} onClose={onClose} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('DocumentPreview', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
    useSearchStore.getState().resetSearch();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders loading skeleton while query is in progress', () => {
    globalThis.fetch = vi.fn().mockImplementation(() => new Promise(() => {}));

    renderPreview('550e8400-e29b-41d4-a716-446655440000');

    // Skeleton elements are present
    const skeletons = document.querySelectorAll('.animate-pulse');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders error state when document cannot be fetched', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          code: 'RESOURCE_NOT_FOUND',
          message: 'Document not found',
        }),
        { status: 404, headers: { 'Content-Type': 'application/json' } },
      ),
    );

    renderPreview('550e8400-e29b-41d4-a716-446655440000');

    await waitFor(() => {
      expect(screen.getByText('Gagal Memuat Dokumen')).toBeDefined();
      expect(screen.getByText('Coba Lagi')).toBeDefined();
    });
  });

  it('renders markdown document with formatting and action buttons', async () => {
    const mockDoc = {
      id: '550e8400-e29b-41d4-a716-446655440000',
      folder_id: '660e8400-e29b-41d4-a716-446655440000',
      folder_root_path: '/home/user/docs',
      relative_path: 'notes/architecture.md',
      title: 'Architecture Overview',
      content: '# Heading 1\n\nThis is a paragraph about **Rust**.\n\n- Point 1\n- Point 2',
      type: 'doc',
      language: 'markdown',
      tags: ['rust', 'arch'],
      file_size: 1024,
      content_hash: 'abc123hash',
      updated_at: '2026-10-05T10:00:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDoc), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const onClose = vi.fn();
    renderPreview(mockDoc.id, onClose);

    await waitFor(() => {
      expect(screen.getByText('Architecture Overview')).toBeDefined();
      expect(screen.getByText('notes/architecture.md')).toBeDefined();
      expect(screen.getByText('Heading 1')).toBeDefined();
      expect(screen.getByText('Point 1')).toBeDefined();
      expect(screen.getByText('Point 2')).toBeDefined();
    });

    // Close button triggers onClose
    const closeBtn = screen.getByLabelText('Close Preview');
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('renders code document with line numbers gutter and highlights query terms', async () => {
    const mockCode = {
      id: '550e8400-e29b-41d4-a716-446655440001',
      folder_id: '660e8400-e29b-41d4-a716-446655440000',
      folder_root_path: '/home/user/code',
      relative_path: 'src/main.rs',
      title: 'main.rs',
      content: 'fn main() {\n    println!("Hello LynxSearch");\n}',
      type: 'code',
      language: 'rust',
      tags: ['rust'],
      file_size: 50,
      content_hash: 'code123hash',
      updated_at: '2026-10-05T10:00:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockCode), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    useSearchStore.getState().setRawQuery('LynxSearch');

    renderPreview(mockCode.id);

    await waitFor(() => {
      expect(screen.getByText('main.rs')).toBeDefined();
      expect(screen.getByText('src/main.rs')).toBeDefined();
      expect(screen.getByText('3 baris')).toBeDefined();
      expect(screen.getByText('1')).toBeDefined();
      expect(screen.getByText('2')).toBeDefined();
      expect(screen.getByText('3')).toBeDefined();
    });

    // Highlight mark for "LynxSearch" is rendered
    const marks = screen.getAllByText('LynxSearch');
    const highlightMark = marks.find((el) => el.tagName.toLowerCase() === 'mark');
    expect(highlightMark).toBeDefined();
  });

  it('calls desktop bridge methods on Copy Path and Open in Editor', async () => {
    const mockDoc = {
      id: '550e8400-e29b-41d4-a716-446655440002',
      folder_id: '660e8400-e29b-41d4-a716-446655440000',
      folder_root_path: '/workspace',
      relative_path: 'README.md',
      title: 'README.md',
      content: '# LynxSearch',
      type: 'doc',
      language: 'markdown',
      tags: [],
      file_size: 100,
      content_hash: 'hash1',
      updated_at: '2026-10-05T10:00:00Z',
    };

    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(mockDoc), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const copySpy = vi.spyOn(desktopBridge, 'copyTextToClipboard').mockResolvedValue();
    const openSpy = vi.spyOn(desktopBridge, 'openFileInEditor').mockResolvedValue();

    renderPreview(mockDoc.id);

    await waitFor(() => {
      expect(screen.getAllByText('README.md').length).toBeGreaterThan(0);
    });

    const copyBtn = screen.getByLabelText('Copy File Content (Cmd+Shift+C)');
    fireEvent.click(copyBtn);
    expect(copySpy).toHaveBeenCalledWith('# LynxSearch');

    const revealSpy = vi.spyOn(desktopBridge, 'revealFileInFolder').mockResolvedValue();
    const revealBtn = screen.getByLabelText('Tampilkan di Folder');
    fireEvent.click(revealBtn);
    expect(revealSpy).toHaveBeenCalledWith('/workspace/README.md');

    // 1. When preferredEditor is null, click triggers openWith modal
    act(() => {
      useUIStore.setState({ preferredEditor: null, openWithModalOpen: false, openWithTargetPath: null });
    });
    const openBtn = screen.getByLabelText('Open in Editor (Cmd+O)');
    fireEvent.click(openBtn);
    expect(useUIStore.getState().openWithModalOpen).toBe(true);
    expect(useUIStore.getState().openWithTargetPath).toBe('/workspace/README.md');

    // 2. When preferredEditor is configured, click invokes openFileInEditor directly
    act(() => {
      useUIStore.setState({ preferredEditor: 'antigravity-ide' });
    });
    fireEvent.click(openBtn);
    await waitFor(() => {
      expect(openSpy).toHaveBeenCalledWith('/workspace/README.md', 'antigravity-ide');
    });
  });
});
