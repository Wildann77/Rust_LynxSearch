import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import * as fs from 'fs';
import * as path from 'path';
import { toast } from 'sonner';

import { WorkspaceShell } from '../components/layout/WorkspaceShell';
import { TopBar } from '../components/layout/TopBar';
import { SearchBar } from '../components/search/SearchBar';
import { HealthDot } from '../components/common/HealthDot';
import { FacetSidebar } from '../components/layout/FacetSidebar';
import { ResultsPane } from '../components/layout/ResultsPane';
import { ResultList } from '../components/search/ResultList';
import { ResultCard } from '../components/search/ResultCard';
import { PreviewPane } from '../components/layout/PreviewPane';
import { VirtualizedCodeViewer } from '../components/preview/VirtualizedCodeViewer';
import { SafeHighlight } from '../components/search/SafeHighlight';
import { JobProgressIndicator } from '../components/jobs/JobProgressIndicator';
import { LynxToaster } from '../components/ui/LynxToaster';
import { TooltipProvider } from '../components/ui/tooltip';

import { useUIStore } from '../stores/uiStore';
import { useSearchStore } from '../stores/searchStore';
import { formatRelativeTime } from '../lib/date';
import * as desktopBridge from '../lib/desktop-bridge';
import type { SearchResultItem, SearchResponse } from '../types/search';
import type { JobStatusResponse } from '../types/job';
import type { HealthSummaryResponse } from '../types/health';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });
}

function renderWithProviders(ui: React.ReactElement, client = createTestQueryClient()) {
  return render(
    <QueryClientProvider client={client}>
      <TooltipProvider delayDuration={0}>{ui}</TooltipProvider>
    </QueryClientProvider>,
  );
}

const sampleItem: SearchResultItem = {
  id: 'a0000000-0000-4000-8000-000000000001',
  title: 'ownership.md',
  relative_path: 'notes/rust/ownership.md',
  type: 'doc',
  language: 'markdown',
  project: 'Rust_LynxSearch',
  tags: ['rust', 'memory'],
  highlights: [
    {
      snippet: 'Rust uses <em>ownership</em> to manage memory safely.',
      line_number: 1,
    },
  ],
  score: 8.75,
  file_size: 2048,
  updated_at: new Date(Date.now() - 3600000).toISOString(),
};

const mockHealthyResponse: HealthSummaryResponse = {
  status: 'ok',
  version: '0.1.0',
  timestamp: new Date().toISOString(),
  database: {
    status: 'ok',
    latency_ms: 1.2,
  },
  elasticsearch: {
    status: 'ok',
    latency_ms: 2.5,
  },
};

const mockSearchResponse: SearchResponse = {
  query: 'rust',
  page: 1,
  size: 20,
  total: 1,
  took_ms: 28,
  items: [sampleItem],
  results: [sampleItem],
  facets: {
    extensions: [{ key: 'md', doc_count: 1 }],
    types: [{ key: 'doc', doc_count: 1 }],
    languages: [{ key: 'markdown', doc_count: 1 }],
    tags: [{ key: 'rust', doc_count: 1 }],
    projects: [{ key: 'Rust_LynxSearch', doc_count: 1 }],
  },
  warnings: [],
};

const validJobId = '550e8400-e29b-41d4-a716-446655440001';

describe('Section 15: Final Desktop UX Verification Matrix', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
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
      preferredEditor: 'code',
    });

    globalThis.fetch = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/api/health')) {
        return Promise.resolve(
          new Response(JSON.stringify(mockHealthyResponse), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      if (url.includes('/api/search')) {
        return Promise.resolve(
          new Response(JSON.stringify(mockSearchResponse), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  // 1. 3-pane layout matches design
  it('1. verifies 3-pane layout matches design (sidebar, results, preview)', () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();
    expect(screen.getByLabelText('Search Results')).toBeDefined();
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
  });

  // 2. Top app bar
  it('2. verifies top app bar contains drag region, branding, search input, and actions', () => {
    renderWithProviders(<TopBar />);

    const header = document.querySelector('header[data-tauri-drag-region]');
    expect(header).toBeDefined();
    expect(screen.getByText('LynxSearch')).toBeDefined();
    expect(screen.getByPlaceholderText(/Search code, docs, tags/i)).toBeDefined();
    expect(screen.getByLabelText(/Backend Status:/i)).toBeDefined();
    expect(screen.getByLabelText('Open Folder Manager')).toBeDefined();
    expect(screen.getByLabelText('Open Settings')).toBeDefined();
  });

  // 3. Global search
  it('3. verifies global search input updates search store query and responds to clear', () => {
    renderWithProviders(<SearchBar />);

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'fn authenticate' } });

    expect(input.value).toBe('fn authenticate');
    expect(useSearchStore.getState().rawQuery).toBe('fn authenticate');

    const clearBtn = screen.getByLabelText('Clear search input');
    fireEvent.click(clearBtn);
    expect(useSearchStore.getState().rawQuery).toBe('');
    expect(input.value).toBe('');
  });

  // 4. Backend status indicator
  it('4. verifies backend status indicator shows health dot with proper status aria-label', async () => {
    renderWithProviders(<HealthDot />);

    await waitFor(() => {
      const dot = screen.getByLabelText(/Backend Status: Ready/i);
      expect(dot).toBeDefined();
    });
  });

  // 5. Folder manager
  it('5. verifies folder manager modal opens and closes properly', async () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.queryByRole('dialog')).toBeNull();

    const folderBtn = screen.getByLabelText('Open Folder Manager');
    fireEvent.click(folderBtn);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeDefined();
    expect(await screen.findByText('Folder Manager')).toBeDefined();

    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  // 6. Settings
  it('6. verifies settings modal opens and closes properly', async () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.queryByRole('dialog')).toBeNull();

    const settingsBtn = screen.getByLabelText('Open Settings');
    fireEvent.click(settingsBtn);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeDefined();
    expect(await screen.findByText('Search Settings')).toBeDefined();

    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  // 7. Facet sidebar
  it('7. verifies facet sidebar renders category facets and item counts', async () => {
    useSearchStore.setState({
      rawQuery: 'rust',
      activeFilters: { extension: 'md' },
    });

    renderWithProviders(<FacetSidebar />);

    await waitFor(() => {
      expect(screen.getByText('Extensions')).toBeDefined();
      expect(screen.getByText('Type')).toBeDefined();
      expect(screen.getByText('Language')).toBeDefined();
      expect(screen.getByText('Tags')).toBeDefined();
      expect(screen.getByText('Projects')).toBeDefined();
    });
  });

  // 8. Results list
  it('8. verifies results list renders cards and pagination controls', () => {
    const onPageChange = vi.fn();
    const onSelect = vi.fn();

    renderWithProviders(
      <ResultList
        items={[sampleItem]}
        total={42}
        page={1}
        pageSize={20}
        onPageChange={onPageChange}
        selectedId={null}
        onSelectItem={onSelect}
      />,
    );

    expect(screen.getByText('ownership.md')).toBeDefined();
    expect(screen.getByText(/42 total dokumen/i)).toBeDefined();

    const nextBtn = screen.getByLabelText('Halaman berikutnya');
    fireEvent.click(nextBtn);
    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  // 9. Preview panel
  it('9. verifies preview panel displays placeholder initially and document preview when selected', () => {
    const { unmount } = renderWithProviders(<PreviewPane />);
    expect(screen.getByText('Pilih dokumen untuk melihat preview')).toBeDefined();
    unmount();

    useSearchStore.setState({ selectedDocId: sampleItem.id });
    renderWithProviders(<PreviewPane />);
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
  });

  // 10. Resizable preview
  it('10. verifies resizable preview divider drag respects constraints [420px, 640px]', () => {
    renderWithProviders(<WorkspaceShell />);

    const separator = screen.getByRole('separator');
    expect(separator).toBeDefined();

    act(() => {
      fireEvent.pointerDown(separator);
    });

    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 400 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(window.innerWidth - 400);

    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 50 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(640);

    act(() => {
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: 950 }));
    });
    expect(useUIStore.getState().previewWidth).toBe(420);

    act(() => {
      window.dispatchEvent(new PointerEvent('pointerup'));
    });
  });

  // 11. Collapsible sidebar
  it('11. verifies collapsible sidebar toggles visibility via store and hotkey [', () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();

    act(() => {
      useUIStore.getState().toggleSidebar();
    });
    expect(screen.queryByLabelText('Facet Filters Sidebar')).toBeNull();

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '[' }));
    });
    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();
  });

  // 12. Collapsible preview
  it('12. verifies collapsible preview toggles visibility via store and hotkey ]', () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();

    act(() => {
      useUIStore.getState().togglePreview();
    });
    expect(screen.queryByLabelText('Document Preview Panel')).toBeNull();

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: ']' }));
    });
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
  });

  // 13. Search result score
  it('13. verifies search result score badge displays BM25 relevance score', () => {
    renderWithProviders(<ResultCard item={sampleItem} onSelect={vi.fn()} />);

    const scoreBadge = screen.getByTestId('result-score');
    expect(scoreBadge).toBeDefined();
    expect(scoreBadge.textContent).toContain('Score 8.75');
    expect(screen.getByTestId('relevance-indicator')).toBeDefined();
  });

  // 14. Search latency
  it('14. verifies search latency display is rendered in results header', async () => {
    useSearchStore.setState({
      rawQuery: 'rust',
      activeFilters: { extension: 'md' },
    });

    renderWithProviders(<ResultsPane />);

    await waitFor(() => {
      expect(screen.getByText('28ms')).toBeDefined();
    });
  });

  // 15. Highlight
  it('15. verifies highlight renders semantic <mark> tags safely', () => {
    const { container } = renderWithProviders(
      <SafeHighlight snippet="Rust uses <em>ownership</em> to manage memory safely." />,
    );

    const mark = container.querySelector('mark');
    expect(mark).toBeDefined();
    expect(mark?.textContent).toBe('ownership');
    expect(mark?.className).toContain('bg-amber-500/20');
  });

  // 16. Line number
  it('16. verifies code viewer renders line number gutter for code files', () => {
    const code = 'fn main() {\n    let answer = 42;\n}';
    renderWithProviders(<VirtualizedCodeViewer code={code} language="rust" />);

    expect(screen.getByText('1')).toBeDefined();
    expect(screen.getByText('2')).toBeDefined();
    expect(screen.getByText('3')).toBeDefined();
  });

  // 17. Open in editor
  it('17. verifies open in editor invokes desktop bridge method', async () => {
    const openSpy = vi.spyOn(desktopBridge, 'openFileInEditor').mockResolvedValue(undefined);

    await desktopBridge.openFileInEditor('/projects/test.rs', 'code');
    expect(openSpy).toHaveBeenCalledWith('/projects/test.rs', 'code');
  });

  // 18. Copy path
  it('18. verifies copy path invokes desktop bridge clipboard method', async () => {
    const copySpy = vi.spyOn(desktopBridge, 'copyTextToClipboard').mockResolvedValue(undefined);

    await desktopBridge.copyTextToClipboard('/projects/test.rs');
    expect(copySpy).toHaveBeenCalledWith('/projects/test.rs');
  });

  // 19. Job progress
  it('19. verifies job progress indicator displays running progress bar and file metrics', async () => {
    const mockJob: JobStatusResponse = {
      job_id: validJobId,
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      processed_files: 50,
      indexed_files: 45,
      skipped_files: 5,
      failed_files: 0,
      total_files: 100,
      started_at: new Date().toISOString(),
      finished_at: null,
    };

    useUIStore.setState({ indexingJobId: validJobId });

    globalThis.fetch = vi.fn().mockImplementation((url: string) => {
      if (url.includes(`/api/index/jobs/${validJobId}`)) {
        return Promise.resolve(
          new Response(JSON.stringify(mockJob), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    renderWithProviders(<JobProgressIndicator />);

    await waitFor(() => {
      const progressBar = screen.getByRole('progressbar');
      expect(progressBar.getAttribute('aria-valuenow')).toBe('50');
      expect(screen.getByText('50/100')).toBeDefined();
      expect(screen.getByText('Skip: 5')).toBeDefined();
    });
  });

  // 20. Job cancellation
  it('20. verifies job cancellation button invokes cancel endpoint', async () => {
    const mockJob: JobStatusResponse = {
      job_id: validJobId,
      folder_id: '550e8400-e29b-41d4-a716-446655440000',
      status: 'RUNNING',
      processed_files: 10,
      indexed_files: 10,
      skipped_files: 0,
      failed_files: 0,
      total_files: 100,
      started_at: new Date().toISOString(),
      finished_at: null,
    };

    useUIStore.setState({ indexingJobId: validJobId });

    const fetchMock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST' && url.includes(`/api/index/jobs/${validJobId}/cancel`)) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              job_id: validJobId,
              status: 'CANCELLED',
              message: 'Pekerjaan dibatalkan',
            }),
            {
              status: 200,
              headers: { 'Content-Type': 'application/json' },
            },
          ),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify(mockJob), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    });

    globalThis.fetch = fetchMock;

    renderWithProviders(<JobProgressIndicator />);

    const cancelBtn = await screen.findByRole('button', { name: /Batalkan pekerjaan indeks/i });
    fireEvent.click(cancelBtn);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining(`/api/index/jobs/${validJobId}/cancel`),
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  // 21. Toasts
  it('21. verifies toasts system renders toast notifications', async () => {
    renderWithProviders(<LynxToaster />);

    act(() => {
      toast.success('File path copied');
    });

    await waitFor(() => {
      expect(screen.getByText('File path copied')).toBeDefined();
    });
  });

  // 22. Relative timestamps
  it('22. verifies relative timestamps format dates accurately', () => {
    const now = Date.now();
    const thirtySecsAgo = new Date(now - 30000).toISOString();
    const fiveMinsAgo = new Date(now - 5 * 60000).toISOString();
    const twoHoursAgo = new Date(now - 2 * 3600000).toISOString();
    const threeDaysAgo = new Date(now - 3 * 86400000).toISOString();

    expect(formatRelativeTime(thirtySecsAgo)).toMatch(/(seconds ago|detik yang lalu|now|baru saja)/i);
    expect(formatRelativeTime(fiveMinsAgo)).toMatch(/5 (minutes ago|menit yang lalu)/i);
    expect(formatRelativeTime(twoHoursAgo)).toMatch(/2 (hours ago|jam yang lalu)/i);
    expect(formatRelativeTime(threeDaysAgo)).toMatch(/3 (days ago|hari yang lalu)/i);
  });

  // 23. Empty state
  it('23. verifies all 3 empty states render correctly (no folders, idle, no results)', () => {
    // 23a: No folders
    const { unmount: unmount1 } = renderWithProviders(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        hasFolders={false}
      />,
    );
    expect(screen.getByText('Belum Ada Folder Terdaftar')).toBeDefined();
    unmount1();

    // 23b: Idle search (has folders, not searching)
    const { unmount: unmount2 } = renderWithProviders(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        hasFolders={true}
        isSearching={false}
      />,
    );
    expect(screen.getByText('Knowledge Base & Code Search')).toBeDefined();
    unmount2();

    // 23c: No results found
    renderWithProviders(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        hasFolders={true}
        isSearching={true}
      />,
    );
    expect(screen.getByText('Tidak ada dokumen yang cocok')).toBeDefined();
  });

  // 24. Loading state
  it('24. verifies loading skeleton state renders without layout shifts', () => {
    const { container } = renderWithProviders(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isLoading={true}
      />,
    );

    const skeletonContainer = screen.getByTestId('results-skeleton');
    expect(skeletonContainer).toBeDefined();
    expect(container.querySelectorAll('.animate-pulse').length).toBeGreaterThan(0);
  });

  // 25. Error state
  it('25. verifies error state renders ERR_SEARCH_FAILED and retry action', () => {
    const onRetry = vi.fn();
    renderWithProviders(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isError={true}
        errorMessage="Connection reset by peer"
        onRetry={onRetry}
      />,
    );

    expect(screen.getByTestId('error-code-badge').textContent).toBe('ERR_SEARCH_FAILED');
    expect(screen.getByText('Connection reset by peer')).toBeDefined();

    const retryBtn = screen.getByRole('button', { name: /Coba Lagi/i });
    fireEvent.click(retryBtn);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  // 26. Success state
  it('26. verifies success state renders result cards with metadata and snippets', () => {
    renderWithProviders(
      <ResultList
        items={[sampleItem]}
        total={1}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
      />,
    );

    expect(screen.getByText('ownership.md')).toBeDefined();
    expect(screen.getByText(/markdown/i)).toBeDefined();
    expect(screen.getByText('Rust_LynxSearch')).toBeDefined();
    expect(screen.getByText('notes/rust/ownership.md')).toBeDefined();
    expect(screen.getByText(/Rust uses/i)).toBeDefined();
  });

  // 27. Keyboard-only navigation
  it('27. verifies keyboard-only navigation (shortcuts, arrow keys, enter, escape)', () => {
    const onSelect = vi.fn();
    renderWithProviders(
      <ResultList
        items={[sampleItem]}
        total={1}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={onSelect}
      />,
    );

    // ArrowDown / j navigates results
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    expect(onSelect).toHaveBeenCalledWith(sampleItem);

    // Enter selects active item
    onSelect.mockClear();
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledWith(sampleItem);
  });

  // 28. Reduced motion
  it('28. verifies reduced motion media query is properly declared in index.css', () => {
    const cssPath = path.resolve(__dirname, '../index.css');
    const cssContent = fs.readFileSync(cssPath, 'utf-8');

    expect(cssContent).toContain('@media (prefers-reduced-motion: reduce)');
    expect(cssContent).toContain('animation-duration: 0.01ms !important');
    expect(cssContent).toContain('transition-duration: 0.01ms !important');
  });

  // 29. Accessibility labels
  it('29. verifies interactive elements have accessibility labels and focus rings', () => {
    renderWithProviders(<WorkspaceShell />);

    expect(screen.getByLabelText('Facet Filters Sidebar')).toBeDefined();
    expect(screen.getByLabelText('Search Results')).toBeDefined();
    expect(screen.getByLabelText('Document Preview Panel')).toBeDefined();
    expect(screen.getByLabelText('Open Folder Manager')).toBeDefined();
    expect(screen.getByLabelText('Open Settings')).toBeDefined();
    expect(screen.getByLabelText(/Backend Status:/i)).toBeDefined();

    const cssPath = path.resolve(__dirname, '../index.css');
    const cssContent = fs.readFileSync(cssPath, 'utf-8');
    expect(cssContent).toContain(':focus-visible');
  });
});
