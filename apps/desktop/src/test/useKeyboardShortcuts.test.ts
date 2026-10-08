import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { useUIStore } from '../stores/uiStore';
import { useSearchStore } from '../stores/searchStore';

describe('useKeyboardShortcuts (Phase 7.20 Keyboard Navigation)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useSearchStore.getState().resetSearch();
    useUIStore.setState({
      sidebarCollapsed: false,
      previewCollapsed: false,
      folderModalOpen: false,
      settingsModalOpen: false,
      statsModalOpen: false,
      jobDrawerExpanded: false,
    });
  });

  it('focuses search input on Cmd+K or /', () => {
    const focusFn = vi.fn();
    const selectFn = vi.fn();
    const searchInput = { focus: focusFn, select: selectFn } as unknown as HTMLInputElement;
    const searchInputRef = { current: searchInput };

    renderHook(() => useKeyboardShortcuts({ searchInputRef }));

    // Cmd+K
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }));
    });
    expect(focusFn).toHaveBeenCalledTimes(1);
    expect(selectFn).toHaveBeenCalledTimes(1);

    // /
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '/' }));
    });
    expect(focusFn).toHaveBeenCalledTimes(2);
    expect(selectFn).toHaveBeenCalledTimes(2);
  });

  it('toggles sidebar and preview using [ and ] when not in text input', () => {
    renderHook(() => useKeyboardShortcuts());

    // Initially open
    expect(useUIStore.getState().sidebarCollapsed).toBe(false);
    expect(useUIStore.getState().previewCollapsed).toBe(false);

    // [ toggles sidebar
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '[' }));
    });
    expect(useUIStore.getState().sidebarCollapsed).toBe(true);

    // ] toggles preview
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: ']' }));
    });
    expect(useUIStore.getState().previewCollapsed).toBe(true);
  });

  it('does NOT fire single-character shortcuts (/, [, ]) when typing in text input', () => {
    const focusFn = vi.fn();
    const searchInput = document.createElement('input');
    document.body.appendChild(searchInput);
    searchInput.focus();

    renderHook(() => useKeyboardShortcuts({ searchInputRef: { current: searchInput } }));

    expect(useUIStore.getState().sidebarCollapsed).toBe(false);

    // Press [ inside text input
    act(() => {
      searchInput.dispatchEvent(new KeyboardEvent('keydown', { key: '[', bubbles: true }));
    });
    // Sidebar should remain open (not toggled)
    expect(useUIStore.getState().sidebarCollapsed).toBe(false);

    // Press / inside text input
    act(() => {
      searchInput.dispatchEvent(new KeyboardEvent('keydown', { key: '/', bubbles: true }));
    });
    expect(focusFn).not.toHaveBeenCalled();

    document.body.removeChild(searchInput);
  });

  it('executes contextual Escape hierarchy strictly in order', () => {
    renderHook(() => useKeyboardShortcuts());

    // Step 1: When a modal is open, Escape closes the modal
    useUIStore.setState({ folderModalOpen: true });
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(useUIStore.getState().folderModalOpen).toBe(false);

    // Step 2: When search input has query, Escape clears query
    useSearchStore.getState().setRawQuery('active search text');
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.focus();

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(useSearchStore.getState().rawQuery).toBe('');

    // Step 3: When preview pane is open, Escape closes preview pane
    input.blur();
    document.body.removeChild(input);
    useUIStore.setState({ previewCollapsed: false });

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(useUIStore.getState().previewCollapsed).toBe(true);

    // Step 4: When an item is selected, Escape deselects the item
    useSearchStore.getState().setSelectedDocId('doc-uuid-123');
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(useSearchStore.getState().selectedDocId).toBeNull();
  });

  it('triggers onOpenSelectedFile and onCopySelectedPath on Cmd+O and Cmd+Shift+C', () => {
    const onOpen = vi.fn();
    const onCopy = vi.fn();

    renderHook(() =>
      useKeyboardShortcuts({
        onOpenSelectedFile: onOpen,
        onCopySelectedPath: onCopy,
      }),
    );

    // Cmd+O
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'o', metaKey: true }));
    });
    expect(onOpen).toHaveBeenCalledTimes(1);

    // Cmd+Shift+C
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'c', metaKey: true, shiftKey: true }),
      );
    });
    expect(onCopy).toHaveBeenCalledTimes(1);
  });
});
