import * as React from 'react';
import {
  FileSearch,
  SearchX,
  AlertCircle,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import type { SearchResultItem } from '../../types/search';
import { ResultCard } from './ResultCard';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface ResultListProps {
  items: SearchResultItem[];
  total: number;
  page: number;
  pageSize: number;
  onPageChange: (newPage: number) => void;
  selectedId: string | null;
  onSelectItem: (item: SearchResultItem) => void;
  isLoading?: boolean;
  isError?: boolean;
  errorMessage?: string;
  onRetry?: () => void;
  isSearching?: boolean;
  onClearFilters?: () => void;
  className?: string;
}

export function ResultList({
  items,
  total,
  page,
  pageSize,
  onPageChange,
  selectedId,
  onSelectItem,
  isLoading = false,
  isError = false,
  errorMessage = 'Gagal memuat hasil pencarian',
  onRetry,
  isSearching = false,
  onClearFilters,
  className,
}: ResultListProps) {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const itemRefs = React.useRef<Map<string, HTMLDivElement>>(new Map());

  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  // Maintain optimistic selection for instantaneous keyboard feedback without cascading effects
  const [optimisticSelectedId, setOptimisticSelectedId] = React.useState<string | null>(null);

  if (optimisticSelectedId !== null && optimisticSelectedId === selectedId) {
    setOptimisticSelectedId(null);
  }

  const activeId = optimisticSelectedId ?? selectedId;

  // Determine current selected index
  const selectedIndex = React.useMemo(() => {
    if (!activeId) return -1;
    return items.findIndex((item) => item.id === activeId);
  }, [items, activeId]);

  // Scroll active item into view when selection changes
  React.useEffect(() => {
    if (activeId) {
      const el = itemRefs.current.get(activeId);
      if (el) {
        el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }, [activeId]);

  // Global result navigation (ArrowDown / j, ArrowUp / k, Enter)
  React.useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (items.length === 0) return;

      const activeEl = document.activeElement;
      const isInputActive =
        activeEl instanceof HTMLInputElement ||
        activeEl instanceof HTMLTextAreaElement ||
        (activeEl instanceof HTMLElement && activeEl.isContentEditable);

      // When focused in an input, only ArrowDown, ArrowUp, and Enter navigate results.
      // Single characters 'j' and 'k' are preserved for typing into the input.
      if (isInputActive && e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Enter') {
        return;
      }

      if (e.key === 'ArrowDown' || (!isInputActive && e.key === 'j')) {
        e.preventDefault();
        const nextIndex = selectedIndex < items.length - 1 ? selectedIndex + 1 : 0;
        const nextItem = items[nextIndex];
        if (nextItem) {
          setOptimisticSelectedId(nextItem.id);
          onSelectItem(nextItem);
        }
      } else if (e.key === 'ArrowUp' || (!isInputActive && e.key === 'k')) {
        e.preventDefault();
        const prevIndex = selectedIndex > 0 ? selectedIndex - 1 : items.length - 1;
        const prevItem = items[prevIndex];
        if (prevItem) {
          setOptimisticSelectedId(prevItem.id);
          onSelectItem(prevItem);
        }
      } else if (e.key === 'Enter') {
        if (selectedIndex >= 0 && selectedIndex < items.length) {
          e.preventDefault();
          onSelectItem(items[selectedIndex]);
        } else if (items.length > 0) {
          e.preventDefault();
          onSelectItem(items[0]);
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [items, selectedIndex, onSelectItem]);

  // 1. Error state
  if (isError) {
    return (
      <div className={cn('flex flex-col items-center justify-center p-8 text-center', className)}>
        <div className="max-w-md w-full rounded-lg border border-destructive/30 bg-destructive/10 p-6 flex flex-col items-center gap-3 text-destructive-foreground">
          <AlertCircle className="h-8 w-8 text-destructive shrink-0" />
          <h3 className="text-sm font-semibold text-foreground">Kesalahan Pencarian</h3>
          <p className="text-xs text-muted-foreground leading-relaxed">{errorMessage}</p>
          {onRetry ? (
            <Button
              variant="outline"
              size="sm"
              onClick={onRetry}
              className="mt-2 text-xs border-destructive/40 hover:bg-destructive/20 cursor-pointer"
            >
              <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
              Coba Lagi
            </Button>
          ) : null}
        </div>
      </div>
    );
  }

  // 2. Loading Skeleton state (when loading and no items yet)
  if (isLoading && items.length === 0) {
    return (
      <div
        className={cn('flex flex-col gap-3 p-4 w-full overflow-y-auto', className)}
        data-testid="results-skeleton"
      >
        {Array.from({ length: 5 }).map((_, i) => (
          <div
            key={i}
            className="rounded-md border border-border/40 bg-card/40 p-4 space-y-3 animate-pulse"
          >
            <div className="flex items-center justify-between">
              <div className="h-4 w-1/3 bg-muted/60 rounded" />
              <div className="h-3 w-16 bg-muted/40 rounded" />
            </div>
            <div className="h-3 w-1/2 bg-muted/40 rounded font-mono" />
            <div className="h-10 w-full bg-muted/30 rounded" />
            <div className="flex items-center justify-between pt-1">
              <div className="h-3 w-24 bg-muted/40 rounded" />
              <div className="h-3 w-12 bg-muted/40 rounded" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  // 3. Empty state: query/filters were active but 0 items found
  if (isSearching && items.length === 0) {
    return (
      <div className={cn('flex flex-1 flex-col items-center justify-center p-8 text-center', className)}>
        <div className="max-w-sm flex flex-col items-center gap-3">
          <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-secondary/80 border border-border text-muted-foreground shadow-inner">
            <SearchX className="h-6 w-6" />
          </div>
          <h3 className="text-base font-semibold text-foreground tracking-tight">
            Tidak ada dokumen yang cocok
          </h3>
          <p className="text-xs text-muted-foreground leading-relaxed">
            Periksa salah ketik kata kunci atau hapus beberapa filter yang terlalu spesifik.
          </p>
          {onClearFilters ? (
            <Button
              variant="outline"
              size="sm"
              onClick={onClearFilters}
              className="mt-2 text-xs cursor-pointer"
            >
              Bersihkan Filter
            </Button>
          ) : null}
        </div>
      </div>
    );
  }

  // 4. Idle state: not searching yet and 0 items
  if (!isSearching && items.length === 0) {
    return (
      <div className={cn('flex flex-1 flex-col items-center justify-center p-8 text-center', className)}>
        <div className="max-w-md space-y-4">
          <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-secondary/80 border border-border text-primary shadow-inner">
            <FileSearch className="h-6 w-6" />
          </div>
          <div className="space-y-1.5">
            <h2 className="text-base font-semibold tracking-tight text-foreground">
              Knowledge Base & Code Search
            </h2>
            <p className="text-xs text-muted-foreground leading-relaxed">
              Type query in top search bar or press / or ⌘K to start searching markdown notes and
              code repositories.
            </p>
          </div>
          <div className="inline-flex items-center gap-2 rounded-md border border-border/60 bg-secondary/40 px-3 py-1.5 text-[11px] font-mono text-muted-foreground">
            <span>Shortcuts:</span>
            <kbd className="rounded border border-border bg-card px-1 py-0.5 text-[10px]">⌘K</kbd>
            <span>Search</span>
            <kbd className="rounded border border-border bg-card px-1 py-0.5 text-[10px]">[</kbd>
            <span>Sidebar</span>
            <kbd className="rounded border border-border bg-card px-1 py-0.5 text-[10px]">]</kbd>
            <span>Preview</span>
          </div>
        </div>
      </div>
    );
  }

  // 5. Success state: Results list with pagination
  return (
    <div
      ref={containerRef}
      tabIndex={0}
      role="region"
      aria-label="Search results list"
      className={cn('flex flex-1 flex-col h-full overflow-hidden outline-hidden', className)}
    >
      {/* Scrollable list of cards */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2.5">
        {items.map((item) => (
          <ResultCard
            key={item.id}
            ref={(node) => {
              if (node) {
                itemRefs.current.set(item.id, node);
              } else {
                itemRefs.current.delete(item.id);
              }
            }}
            item={item}
            isSelected={item.id === activeId}
            onSelect={onSelectItem}
          />
        ))}
      </div>

      {/* Pagination Bar */}
      <div className="flex h-11 items-center justify-between border-t border-border bg-card/30 px-4 shrink-0 font-mono text-xs select-none">
        <div className="text-muted-foreground text-[11px]">
          <span>
            Halaman <span className="font-semibold text-foreground">{page}</span> dari{' '}
            <span className="font-semibold text-foreground">{totalPages}</span>
          </span>
          <span className="mx-2 text-border">|</span>
          <span>{total} total dokumen</span>
        </div>

        <div className="flex items-center gap-1.5">
          <Button
            variant="outline"
            size="sm"
            disabled={page <= 1 || isLoading}
            onClick={() => onPageChange(page - 1)}
            className="h-7 px-2 text-xs cursor-pointer disabled:cursor-not-allowed"
            aria-label="Halaman sebelumnya"
          >
            <ChevronLeft className="h-3.5 w-3.5 mr-1" />
            Prev
          </Button>

          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages || isLoading}
            onClick={() => onPageChange(page + 1)}
            className="h-7 px-2 text-xs cursor-pointer disabled:cursor-not-allowed"
            aria-label="Halaman berikutnya"
          >
            Next
            <ChevronRight className="h-3.5 w-3.5 ml-1" />
          </Button>
        </div>
      </div>
    </div>
  );
}
