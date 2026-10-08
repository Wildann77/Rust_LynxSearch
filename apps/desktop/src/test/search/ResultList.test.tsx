import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ResultList } from '../../components/search/ResultList';
import { ResultCard } from '../../components/search/ResultCard';
import type { SearchResultItem } from '../../types/search';

const mockItems: SearchResultItem[] = [
  {
    id: '550e8400-e29b-41d4-a716-446655440001',
    title: 'auth_service.rs',
    relative_path: 'crates/backend/src/auth_service.rs',
    project: 'backend',
    type: 'code',
    language: 'rust',
    tags: ['auth', 'backend'],
    highlights: [
      {
        snippet: 'pub async fn <em>authenticate_user</em>() -> Result<User>',
        line_number: 42,
      },
    ],
    score: 8.423,
    file_size: 1024 * 14,
    updated_at: '2026-10-01T12:00:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440002',
    title: 'ARCHITECTURE.md',
    relative_path: 'docs/ARCHITECTURE.md',
    project: null,
    type: 'doc',
    language: 'markdown',
    tags: ['docs', 'architecture'],
    highlights: [
      {
        snippet: 'Clean <em>Hexagonal Architecture</em> pattern',
        line_number: 10,
      },
    ],
    score: 5.12,
    file_size: 1024 * 50,
    updated_at: '2026-10-02T12:00:00Z',
  },
];

describe('ResultCard (Phase 7.10)', () => {
  it('displays title, relative path, snippet, score, line number, tags, and file size', () => {
    const onSelect = vi.fn();
    render(<ResultCard item={mockItems[0]} onSelect={onSelect} isSelected={false} />);

    // Title
    expect(screen.getByText('auth_service.rs')).toBeDefined();
    // Language & Project
    expect(screen.getByText('rust')).toBeDefined();
    expect(screen.getAllByText('backend').length).toBeGreaterThanOrEqual(1);
    // Path
    expect(screen.getByText('crates/backend/src/auth_service.rs')).toBeDefined();
    // Score & Relevance Indicator
    expect(screen.getByText('Score 8.42')).toBeDefined();
    expect(screen.getByTestId('relevance-indicator')).toBeDefined();
    // Line number
    expect(screen.getByText('L42')).toBeDefined();
    // Snippet content
    expect(screen.getByText('authenticate_user')).toBeDefined();
    // Tags
    expect(screen.getByText('auth')).toBeDefined();
    // File size
    expect(screen.getByText('14.0 KB')).toBeDefined();
  });

  it('reflects selected state and triggers onSelect on click or Enter', () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <ResultCard item={mockItems[0]} onSelect={onSelect} isSelected={false} />,
    );

    const card = screen.getByRole('button');
    expect(card.getAttribute('aria-selected')).toBe('false');

    fireEvent.click(card);
    expect(onSelect).toHaveBeenCalledWith(mockItems[0]);

    fireEvent.keyDown(card, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledTimes(2);

    rerender(<ResultCard item={mockItems[0]} onSelect={onSelect} isSelected={true} />);
    expect(card.getAttribute('aria-selected')).toBe('true');
  });
});

describe('ResultList (Phase 7.10)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders idle welcome state when not searching and items empty', () => {
    render(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isSearching={false}
      />,
    );

    expect(screen.getByText(/Knowledge Base & Code Search/i)).toBeDefined();
    expect(screen.getAllByText('⌘K').length).toBeGreaterThanOrEqual(1);
  });

  it('renders FolderPlus empty state when no folders are registered yet', () => {
    const onAddFolder = vi.fn();
    render(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isSearching={false}
        hasFolders={false}
        onAddFolder={onAddFolder}
      />,
    );

    expect(screen.getByText('Belum Ada Folder Terdaftar')).toBeDefined();
    const addBtn = screen.getByText('Tambah Folder');
    fireEvent.click(addBtn);
    expect(onAddFolder).toHaveBeenCalled();
  });

  it('renders empty state with clear filters button when searching yields 0 items', () => {
    const onClearFilters = vi.fn();
    render(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isSearching={true}
        onClearFilters={onClearFilters}
      />,
    );

    expect(screen.getByText(/Tidak ada dokumen yang cocok/i)).toBeDefined();
    const clearBtn = screen.getByText('Bersihkan Filter');
    fireEvent.click(clearBtn);
    expect(onClearFilters).toHaveBeenCalled();
  });

  it('renders loading skeleton with exact dimensions to eliminate CLS', () => {
    render(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isLoading={true}
        isSearching={true}
      />,
    );

    const skeletonContainer = screen.getByTestId('results-skeleton');
    expect(skeletonContainer).toBeDefined();
    expect(skeletonContainer.querySelectorAll('.animate-pulse').length).toBe(5);
  });

  it('renders error state with structured error code and retry button', () => {
    const onRetry = vi.fn();
    render(
      <ResultList
        items={[]}
        total={0}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={null}
        onSelectItem={vi.fn()}
        isError={true}
        errorMessage="Backend service offline"
        onRetry={onRetry}
      />,
    );

    expect(screen.getByText('Kesalahan Pencarian')).toBeDefined();
    expect(screen.getByText('Backend service offline')).toBeDefined();
    expect(screen.getByTestId('error-code-badge').textContent).toBe('ERR_SEARCH_FAILED');

    const retryBtn = screen.getByText('Coba Lagi');
    fireEvent.click(retryBtn);
    expect(onRetry).toHaveBeenCalled();
  });

  it('renders items, handles pagination controls, and triggers onPageChange', () => {
    const onPageChange = vi.fn();
    const onSelectItem = vi.fn();

    render(
      <ResultList
        items={mockItems}
        total={45}
        page={1}
        pageSize={20}
        onPageChange={onPageChange}
        selectedId={mockItems[0].id}
        onSelectItem={onSelectItem}
        isSearching={true}
      />,
    );

    // Items rendered
    expect(screen.getByText('auth_service.rs')).toBeDefined();
    expect(screen.getByText('ARCHITECTURE.md')).toBeDefined();

    // Pagination info
    expect(screen.getByText(/Halaman/i)).toBeDefined();
    expect(screen.getByText('3')).toBeDefined(); // 45 / 20 = 3 pages
    expect(screen.getByText(/45 total dokumen/)).toBeDefined();

    // Previous should be disabled on page 1
    const prevBtn = screen.getByLabelText('Halaman sebelumnya') as HTMLButtonElement;
    expect(prevBtn.disabled).toBe(true);

    // Next should be enabled and trigger onPageChange(2)
    const nextBtn = screen.getByLabelText('Halaman berikutnya') as HTMLButtonElement;
    expect(nextBtn.disabled).toBe(false);
    fireEvent.click(nextBtn);
    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  it('handles keyboard navigation (ArrowDown/j and ArrowUp/k) across results', () => {
    const onSelectItem = vi.fn();

    render(
      <ResultList
        items={mockItems}
        total={2}
        page={1}
        pageSize={20}
        onPageChange={vi.fn()}
        selectedId={mockItems[0].id}
        onSelectItem={onSelectItem}
        isSearching={true}
      />,
    );

    // Press j or ArrowDown -> selects next item
    fireEvent.keyDown(window, { key: 'j' });
    expect(onSelectItem).toHaveBeenCalledWith(mockItems[1]);

    // Press k or ArrowUp -> selects previous item
    fireEvent.keyDown(window, { key: 'k' });
    expect(onSelectItem).toHaveBeenCalledWith(mockItems[0]);

    // Press Enter -> selects active item
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(onSelectItem).toHaveBeenCalledWith(mockItems[0]);
  });
});
