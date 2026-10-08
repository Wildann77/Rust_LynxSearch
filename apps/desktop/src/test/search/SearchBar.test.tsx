import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SearchBar } from '../../components/search/SearchBar';
import { useSearchStore } from '../../stores/searchStore';

function renderSearchBar(props = {}) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <SearchBar {...props} />
    </QueryClientProvider>,
  );
}

describe('SearchBar (Phase 7.9)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useSearchStore.getState().resetSearch();
  });

  it('renders input with placeholder and shortcut badge', () => {
    renderSearchBar();

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    expect(input).toBeDefined();
    expect(input.value).toBe('');
    expect(screen.getByText('K')).toBeDefined();
  });

  it('updates rawQuery in store when typing', () => {
    renderSearchBar();

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'tokio async' } });

    expect(useSearchStore.getState().rawQuery).toBe('tokio async');
    expect(input.value).toBe('tokio async');
  });

  it('clears rawQuery and preserves focus when clear button is clicked', () => {
    useSearchStore.setState({ rawQuery: 'search query' });
    renderSearchBar();

    const clearBtn = screen.getByLabelText('Clear search input');
    expect(clearBtn).toBeDefined();

    act(() => {
      fireEvent.click(clearBtn);
    });

    expect(useSearchStore.getState().rawQuery).toBe('');
    const input = screen.getByPlaceholderText(/Search code, docs, tags/i);
    expect(document.activeElement).toBe(input);
  });

  it('shows loader spinner when isLoading is true or while debouncing with non-empty input', () => {
    // Explicit isLoading
    const { unmount } = renderSearchBar({ isLoading: true });
    expect(screen.getByTestId('search-loader')).toBeDefined();
    unmount();

    // While typing with debounce
    renderSearchBar({ debounceMs: 500 });
    const input = screen.getByPlaceholderText(/Search code, docs, tags/i);
    fireEvent.change(input, { target: { value: 'testing loader' } });

    // Should immediately show loader during debounce period
    expect(screen.getByTestId('search-loader')).toBeDefined();
  });

  it('renders active filter chips scaffold and removes a chip on click', () => {
    useSearchStore.setState({
      activeFilters: { type: 'code', language: 'rust' },
    });

    renderSearchBar();

    expect(screen.getByTestId('filter-chip-type')).toBeDefined();
    expect(screen.getByText('code')).toBeDefined();
    expect(screen.getByTestId('filter-chip-language')).toBeDefined();
    expect(screen.getByText('rust')).toBeDefined();

    const removeTypeBtn = screen.getByLabelText('Remove filter type');
    act(() => {
      fireEvent.click(removeTypeBtn);
    });

    expect(useSearchStore.getState().activeFilters.type).toBeUndefined();
    expect(useSearchStore.getState().activeFilters.language).toBe('rust');
  });

  it('handles Escape key to clear query or blur input', () => {
    useSearchStore.setState({ rawQuery: 'escape test' });
    renderSearchBar();

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    input.focus();

    // First Escape clears query
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(useSearchStore.getState().rawQuery).toBe('');

    // Second Escape blurs
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(document.activeElement).not.toBe(input);
  });
});
