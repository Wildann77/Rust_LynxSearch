import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FacetSidebar } from '../../components/layout/FacetSidebar';
import { useSearchStore } from '../../stores/searchStore';
import { useUIStore } from '../../stores/uiStore';

function renderSidebar() {
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
      <FacetSidebar />
    </QueryClientProvider>,
  );
}

describe('FacetSidebar Micro-States (Phase 7.19)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useSearchStore.getState().resetSearch();
    useUIStore.setState({ sidebarCollapsed: false });
  });

  it('renders idle empty state when not searching', () => {
    renderSidebar();
    expect(screen.getByText(/Ketik pencarian untuk melihat agregasi kategori/i)).toBeDefined();
  });

  it('renders nothing when sidebar is collapsed', () => {
    useUIStore.setState({ sidebarCollapsed: true });
    const { container } = renderSidebar();
    expect(container.firstChild).toBeNull();
  });

  it('renders facet groups and handles filter toggle on success', () => {
    useSearchStore.getState().setRawQuery('auth');

    // Mock search query response with facets
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          query: 'auth',
          page: 1,
          size: 20,
          total: 10,
          took_ms: 12,
          items: [],
          facets: {
            types: [{ key: 'code', doc_count: 8 }, { key: 'doc', doc_count: 2 }],
            languages: [{ key: 'rust', doc_count: 6 }, { key: 'typescript', doc_count: 2 }],
            tags: [{ key: 'auth', doc_count: 5 }],
            projects: [{ key: 'backend', doc_count: 8 }],
          },
          warnings: [],
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        },
      ),
    );

    renderSidebar();

    // After resolution or mock, query is rendered
    return vi.waitFor(() => {
      expect(screen.getByText('Type')).toBeDefined();
      expect(screen.getByText('Language')).toBeDefined();
      expect(screen.getByText('rust')).toBeDefined();
      expect(screen.getByText('6')).toBeDefined(); // doc_count badge

      // Toggle facet
      const rustBtn = screen.getByText('rust').closest('button');
      expect(rustBtn).toBeDefined();
      if (rustBtn) {
        fireEvent.click(rustBtn);
        expect(useSearchStore.getState().activeFilters.language).toBe('rust');
      }
    });
  });

  it('renders error state with structured code when query fails', () => {
    useSearchStore.getState().setRawQuery('error-trigger');

    vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('Connection refused'));

    renderSidebar();

    return vi.waitFor(() => {
      expect(screen.getByText('Gagal Memuat Facet')).toBeDefined();
      expect(screen.getByTestId('facet-error-code').textContent).toBe('ERR_FACETS_UNAVAILABLE');
      expect(screen.getByText('Coba Lagi')).toBeDefined();
    });
  });

  it('renders extensions group, active filters panel, and supports multi-select', async () => {
    useSearchStore.getState().setRawQuery('test');

    vi.spyOn(globalThis, 'fetch').mockImplementation(() =>
      Promise.resolve(
        new Response(
          JSON.stringify({
            query: 'test',
            page: 1,
            size: 20,
            total: 5,
            took_ms: 10,
            items: [],
            facets: {
              extensions: [{ key: 'rs', doc_count: 3 }, { key: 'md', doc_count: 2 }],
              types: [{ key: 'code', doc_count: 3 }],
              languages: [{ key: 'rust', doc_count: 3 }],
              tags: [],
              projects: [],
            },
            warnings: [],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      ),
    );

    renderSidebar();

    // 1. Wait for Extensions group to appear
    await vi.waitFor(() => {
      expect(screen.getByTestId('facet-group-extension')).toBeDefined();
    });

    const extGroup = screen.getByTestId('facet-group-extension');
    expect(within(extGroup).getByText('Extensions')).toBeDefined();
    expect(within(extGroup).getByText('rs')).toBeDefined();

    // 2. Select first extension 'rs'
    const rsBtn = within(extGroup).getByText('rs').closest('button')!;
    fireEvent.click(rsBtn);
    expect(useSearchStore.getState().activeFilters.extension).toBe('rs');

    // 3. Wait for facets to re-render and select second extension 'md' -> multi-select
    await vi.waitFor(() => {
      expect(screen.getByTestId('facet-group-extension')).toBeDefined();
    });
    const mdBtn = within(screen.getByTestId('facet-group-extension'))
      .getByText('md')
      .closest('button')!;
    fireEvent.click(mdBtn);
    expect(useSearchStore.getState().activeFilters.extension).toBe('rs,md');

    // 4. Verify Active Filters panel rendered in DOM
    await vi.waitFor(() => {
      expect(screen.getByTestId('active-filters-panel')).toBeDefined();
    });

    const activePanel = screen.getByTestId('active-filters-panel');
    const removeRsBtn = within(activePanel).getByLabelText('Remove filter extension:rs');
    fireEvent.click(removeRsBtn);
    expect(useSearchStore.getState().activeFilters.extension).toBe('md');
  });
});
