import * as React from 'react';
import { ArrowUpDown, AlertTriangle } from 'lucide-react';
import { useSearchStore } from '../../stores/searchStore';
import { useUIStore } from '../../stores/uiStore';
import { useDebounce } from '../../hooks/useDebounce';
import { useSearchQuery } from '../../hooks/useSearchQueries';
import { useFoldersQuery } from '../../hooks/useFolderQueries';
import { ResultList } from '../search/ResultList';
import { OfflineFallback } from '../common/OfflineFallback';
import { useHealthQuery } from '../../hooks/useHealthQuery';
import type { SearchRequest, SearchResultItem } from '../../types/search';
import { cn } from '../../lib/utils';

export interface ResultsPaneProps {
  className?: string;
}

export function ResultsPane({ className }: ResultsPaneProps) {
  const rawQuery = useSearchStore((state) => state.rawQuery);
  const activeFilters = useSearchStore((state) => state.activeFilters);
  const page = useSearchStore((state) => state.page);
  const pageSize = useSearchStore((state) => state.pageSize);
  const selectedDocId = useSearchStore((state) => state.selectedDocId);
  const setPage = useSearchStore((state) => state.setPage);
  const setSelectedDocId = useSearchStore((state) => state.setSelectedDocId);
  const resetSearch = useSearchStore((state) => state.resetSearch);
  const setPreviewCollapsed = useUIStore((state) => state.setPreviewCollapsed);
  const setFolderModalOpen = useUIStore((state) => state.setFolderModalOpen);

  const handleClearSearchAndFilters = React.useCallback(() => {
    resetSearch();
  }, [resetSearch]);

  const { data: folders } = useFoldersQuery();
  const hasFolders = folders ? folders.length > 0 : true;

  // Debounce query string for network efficiency (200ms)
  const debouncedQuery = useDebounce(rawQuery, 200);

  // Determine if active search or filters are applied
  const isSearching = Boolean(
    debouncedQuery.trim() ||
      activeFilters.type ||
      activeFilters.language ||
      activeFilters.tag ||
      activeFilters.project,
  );

  const searchParams: SearchRequest = React.useMemo(
    () => ({
      q: debouncedQuery.trim() || undefined,
      page,
      size: pageSize,
      type: activeFilters.type || undefined,
      language: activeFilters.language || undefined,
      tag: activeFilters.tag || undefined,
      project: activeFilters.project || undefined,
    }),
    [debouncedQuery, page, pageSize, activeFilters],
  );

  const { data, isLoading, isFetching, isError, error, refetch } = useSearchQuery(searchParams, {
    enabled: isSearching,
  });

  const {
    isError: isHealthError,
    isSuccess: isHealthSuccess,
    isLoading: isHealthLoading,
    isRefetching: isHealthRefetching,
    refetch: refetchHealth,
  } = useHealthQuery();

  const isBackendOffline = isHealthError || (!isHealthLoading && !isHealthSuccess);

  const items: SearchResultItem[] = data?.items?.length ? data.items : (data?.results ?? []);
  const total = data?.total ?? 0;
  const tookMs = data?.took_ms ?? 0;
  const warnings = data?.warnings ?? [];

  // Preserve selected document between rerenders:
  // If selectedDocId is in the new list, it remains selected.
  // When a user selects an item, open preview pane if collapsed.
  const handleSelectItem = (item: SearchResultItem) => {
    setSelectedDocId(item.id);
    setPreviewCollapsed(false);
  };

  const handleRetryAll = () => {
    void refetchHealth();
    if (isSearching) {
      void refetch();
    }
  };

  return (
    <section
      className={cn(
        'flex flex-1 min-w-0 flex-col bg-background h-full overflow-hidden select-none',
        className,
      )}
      aria-label="Search Results"
    >
      {/* Results Header: Metrics and Sorting */}
      <div className="flex h-10 items-center justify-between border-b border-border px-4 shrink-0 bg-card/20">
        <div className="flex items-center gap-2 text-xs font-mono text-muted-foreground truncate">
          {isSearching ? (
            <div className="flex items-center gap-2 truncate">
              <span>
                Total: <span className="text-foreground font-semibold">{total}</span> docs
              </span>
              <span className="text-border">|</span>
              <span>{tookMs}ms</span>
              {rawQuery ? (
                <>
                  <span className="text-border">|</span>
                  <span className="truncate max-w-[200px]">
                    "{rawQuery}"
                  </span>
                </>
              ) : null}
            </div>
          ) : (
            <span>Ready for search</span>
          )}
        </div>

        <div className="flex items-center gap-2 text-xs text-muted-foreground font-mono shrink-0">
          {warnings.length > 0 ? (
            <div
              className="flex items-center gap-1 rounded bg-amber-500/10 border border-amber-500/30 px-2 py-0.5 text-[10px] text-amber-400 font-mono"
              title={warnings.join('; ')}
            >
              <AlertTriangle className="h-3 w-3 shrink-0" />
              <span>Warning</span>
            </div>
          ) : null}

          <div className="flex items-center gap-1.5 rounded-md border border-border bg-secondary/40 px-2 py-0.5 text-[11px]">
            <ArrowUpDown className="h-3 w-3 text-muted-foreground" />
            <span>Sort: BM25 Relevance</span>
          </div>
        </div>
      </div>

      {/* Main Results Viewport */}
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        {isBackendOffline && (isError || isSearching) ? (
          <OfflineFallback
            onRetry={handleRetryAll}
            isRetrying={isHealthRefetching || isFetching}
          />
        ) : (
          <ResultList
            items={items}
            total={total}
            page={page}
            pageSize={pageSize}
            onPageChange={setPage}
            selectedId={selectedDocId}
            onSelectItem={handleSelectItem}
            isLoading={isLoading || (isFetching && items.length === 0)}
            isError={isError}
            errorMessage={error?.message}
            onRetry={() => refetch()}
            isSearching={isSearching}
            onClearFilters={handleClearSearchAndFilters}
            hasFolders={hasFolders}
            onAddFolder={() => setFolderModalOpen(true)}
          />
        )}
      </div>
    </section>
  );
}
