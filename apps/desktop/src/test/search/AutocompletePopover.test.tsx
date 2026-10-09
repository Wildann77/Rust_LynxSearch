import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AutocompletePopover } from '../../components/search/AutocompletePopover';
import { SearchBar } from '../../components/search/SearchBar';
import { useSearchStore } from '../../stores/searchStore';

describe('AutocompletePopover UI component', () => {
  it('does not render when isOpen is false or suggestions is empty', () => {
    const { rerender } = render(
      <AutocompletePopover
        isOpen={false}
        suggestions={['auth', 'authenticate']}
        query="au"
        highlightedIndex={-1}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByTestId('autocomplete-popover')).toBeNull();

    rerender(
      <AutocompletePopover
        isOpen={true}
        suggestions={[]}
        query="au"
        highlightedIndex={-1}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByTestId('autocomplete-popover')).toBeNull();
  });

  it('renders suggestion items and highlights active item', () => {
    const onSelect = vi.fn();
    render(
      <AutocompletePopover
        isOpen={true}
        suggestions={['Authentication Service', 'Authorize Middleware']}
        query="Auth"
        highlightedIndex={0}
        onSelect={onSelect}
      />,
    );

    const popover = screen.getByTestId('autocomplete-popover');
    expect(popover).toBeDefined();

    const item0 = screen.getByTestId('autocomplete-item-0');
    const item1 = screen.getByTestId('autocomplete-item-1');
    expect(item0.getAttribute('aria-selected')).toBe('true');
    expect(item1.getAttribute('aria-selected')).toBe('false');

    // Clicking option triggers onSelect
    fireEvent.mouseDown(item1);
    expect(onSelect).toHaveBeenCalledWith('Authorize Middleware');
  });
});

describe('SearchBar with Autocomplete Integration', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.restoreAllMocks();
    useSearchStore.getState().resetSearch();

    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: 0 },
      },
    });
  });

  const renderWithClient = (ui: React.ReactElement) =>
    render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);

  it('does not show autocomplete for short queries (< 2 chars)', async () => {
    renderWithClient(<SearchBar debounceMs={0} />);

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: 'a' } });

    expect(screen.queryByTestId('autocomplete-popover')).toBeNull();
  });

  it('navigates suggestions with ArrowDown / ArrowUp and selects on Enter', async () => {
    // Mock fetch response for suggest endpoint
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (url) => {
      const urlStr = url.toString();
      if (urlStr.includes('/api/suggest')) {
        return new Response(
          JSON.stringify({
            suggestions: ['Authentication Service', 'Authorize Controller'],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      return new Response(JSON.stringify({}), { status: 200 });
    });

    renderWithClient(<SearchBar debounceMs={0} />);

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    fireEvent.focus(input);

    act(() => {
      fireEvent.change(input, { target: { value: 'auth' } });
    });

    // Wait for popover to render
    const popover = await screen.findByTestId('autocomplete-popover');
    expect(popover).toBeDefined();

    const item0 = screen.getByTestId('autocomplete-item-0');
    const item1 = screen.getByTestId('autocomplete-item-1');
    expect(item0).toBeDefined();
    expect(item1).toBeDefined();

    // ArrowDown moves highlight to index 0
    act(() => {
      fireEvent.keyDown(input, { key: 'ArrowDown' });
    });
    expect(item0.getAttribute('aria-selected')).toBe('true');

    // ArrowDown moves highlight to index 1
    act(() => {
      fireEvent.keyDown(input, { key: 'ArrowDown' });
    });
    expect(item1.getAttribute('aria-selected')).toBe('true');

    // Enter selects highlighted item and updates store
    act(() => {
      fireEvent.keyDown(input, { key: 'Enter' });
    });

    expect(useSearchStore.getState().rawQuery).toBe('Authorize Controller');
    expect(screen.queryByTestId('autocomplete-popover')).toBeNull();
  });

  it('Escape closes suggestion popover on first press, and clears query on second press', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (url) => {
      const urlStr = url.toString();
      if (urlStr.includes('/api/suggest')) {
        return new Response(
          JSON.stringify({
            suggestions: ['Auth Service'],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      return new Response(JSON.stringify({}), { status: 200 });
    });

    renderWithClient(<SearchBar debounceMs={0} />);

    const input = screen.getByPlaceholderText(/Search code, docs, tags/i) as HTMLInputElement;
    fireEvent.focus(input);

    act(() => {
      fireEvent.change(input, { target: { value: 'auth' } });
    });

    await screen.findByTestId('autocomplete-popover');

    // First Escape: closes popover without clearing rawQuery
    act(() => {
      fireEvent.keyDown(input, { key: 'Escape' });
    });
    expect(screen.queryByTestId('autocomplete-popover')).toBeNull();
    expect(useSearchStore.getState().rawQuery).toBe('auth');

    // Second Escape: clears query
    act(() => {
      fireEvent.keyDown(input, { key: 'Escape' });
    });
    expect(useSearchStore.getState().rawQuery).toBe('');
  });
});
