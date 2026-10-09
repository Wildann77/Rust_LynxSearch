import * as React from 'react';
import {
  SlidersHorizontal,
  ChevronLeft,
  Tag,
  FileCode,
  File,
  Layers,
  FolderGit2,
  AlertCircle,
  RefreshCw,
  SearchX,
  X,
  Check,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useSearchStore, type ActiveFilters } from '../../stores/searchStore';
import { useDebounce } from '../../hooks/useDebounce';
import { useSearchQuery } from '../../hooks/useSearchQueries';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import type { FacetBucket, SearchRequest } from '../../types/search';
import { cn } from '../../lib/utils';

export interface FacetSidebarProps {
  className?: string;
}

export function FacetSidebar({ className }: FacetSidebarProps) {
  const { sidebarCollapsed, toggleSidebar } = useUIStore();
  const rawQuery = useSearchStore((state) => state.rawQuery);
  const activeFilters = useSearchStore((state) => state.activeFilters);
  const setFilter = useSearchStore((state) => state.setFilter);
  const toggleFilterValue = useSearchStore((state) => state.toggleFilterValue);
  const removeFilterValue = useSearchStore((state) => state.removeFilterValue);
  const clearFilters = useSearchStore((state) => state.clearFilters);
  const page = useSearchStore((state) => state.page);
  const pageSize = useSearchStore((state) => state.pageSize);

  const debouncedQuery = useDebounce(rawQuery, 200);
  const isSearching = Boolean(
    debouncedQuery.trim() ||
      activeFilters.extension ||
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
      extension: activeFilters.extension || undefined,
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

  // Extract flat list of active filter pills for top display
  const activeFilterPills = React.useMemo(() => {
    const pills: { key: keyof ActiveFilters; val: string }[] = [];
    (Object.entries(activeFilters) as [keyof ActiveFilters, string | undefined][]).forEach(
      ([key, str]) => {
        if (!str) return;
        str
          .split(',')
          .map((v) => v.trim())
          .filter(Boolean)
          .forEach((val) => {
            pills.push({ key, val });
          });
      },
    );
    return pills;
  }, [activeFilters]);

  const totalActiveFilterCount = activeFilterPills.length;

  if (sidebarCollapsed) {
    return null;
  }

  const facets = data?.facets;
  const hasFacets = Boolean(
    facets &&
      ((facets.extensions?.length ?? 0) > 0 ||
        (facets.types?.length ?? 0) > 0 ||
        (facets.languages?.length ?? 0) > 0 ||
        (facets.tags?.length ?? 0) > 0 ||
        (facets.projects?.length ?? 0) > 0),
  );

  const renderFacetGroup = (
    title: string,
    icon: React.ReactNode,
    filterKey: keyof ActiveFilters,
    buckets?: FacetBucket[],
  ) => {
    if (!buckets || buckets.length === 0) return null;
    const currentVal = activeFilters[filterKey] ?? '';
    const activeValues = currentVal
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    return (
      <div className="space-y-1.5" data-testid={`facet-group-${filterKey}`}>
        <div className="flex items-center justify-between text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
          <div className="flex items-center gap-1.5">
            {icon}
            <span>{title}</span>
          </div>
          {activeValues.length > 0 ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setFilter(filterKey, null)}
              className="h-5 px-1 text-[10px] text-muted-foreground hover:text-foreground cursor-pointer"
            >
              Clear
            </Button>
          ) : null}
        </div>

        <div className="flex flex-col gap-1">
          {buckets.map((bucket) => {
            const isSelected = activeValues.includes(bucket.key);
            return (
              <button
                key={bucket.key}
                type="button"
                onClick={() => toggleFilterValue(filterKey, bucket.key)}
                className={cn(
                  'flex items-center justify-between rounded px-2 py-1 text-xs transition-colors cursor-pointer text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary group',
                  isSelected
                    ? 'bg-primary/15 text-primary font-medium border border-primary/30'
                    : 'text-neutral-300 hover:bg-secondary/60 hover:text-foreground border border-transparent',
                )}
                aria-pressed={isSelected}
              >
                <div className="flex items-center gap-2 truncate">
                  <span
                    className={cn(
                      'flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border transition-colors',
                      isSelected
                        ? 'border-primary bg-primary text-primary-foreground'
                        : 'border-muted-foreground/40 bg-background/50 group-hover:border-primary/60',
                    )}
                    aria-hidden="true"
                  >
                    {isSelected && <Check className="h-2.5 w-2.5 stroke-[3]" />}
                  </span>
                  <span className="truncate">{bucket.key}</span>
                </div>
                <Badge
                  variant="secondary"
                  className={cn(
                    'font-mono text-[10px] px-1 py-0 h-4 shrink-0 ml-1',
                    isSelected && 'bg-primary/20 text-primary border-primary/30',
                  )}
                >
                  {bucket.doc_count}
                </Badge>
              </button>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <aside
      className={cn(
        'w-[260px] shrink-0 border-r border-border bg-card/40 flex flex-col h-full select-none overflow-hidden transition-all duration-150',
        className,
      )}
      aria-label="Facet Filters Sidebar"
    >
      {/* Sidebar Header */}
      <div className="flex h-10 items-center justify-between border-b border-border px-3 shrink-0">
        <div className="flex items-center gap-2 text-xs font-semibold tracking-tight text-foreground">
          <SlidersHorizontal className="h-3.5 w-3.5 text-primary" />
          <span>Filters</span>
          {isFetching && facets ? (
            <RefreshCw className="h-3 w-3 text-muted-foreground animate-spin" />
          ) : null}
          {totalActiveFilterCount > 0 ? (
            <Badge variant="outline" className="text-[10px] px-1 py-0 h-4 font-mono text-primary border-primary/40">
              {totalActiveFilterCount}
            </Badge>
          ) : null}
        </div>

        <div className="flex items-center gap-1">
          {totalActiveFilterCount > 0 ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={clearFilters}
              className="h-6 px-1.5 text-[10px] text-muted-foreground hover:text-foreground cursor-pointer"
            >
              <X className="h-3 w-3 mr-1" />
              Reset
            </Button>
          ) : null}

          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            className="h-6 w-6 text-muted-foreground hover:text-foreground cursor-pointer"
            aria-label="Collapse sidebar ([)"
          >
            <ChevronLeft className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Main Content Pane */}
      <div className="flex-1 overflow-y-auto p-3 space-y-4 text-xs">
        {/* Active Filters Display Section (TASK.md 8.6) */}
        {totalActiveFilterCount > 0 ? (
          <div
            className="rounded-md border border-border/60 bg-secondary/30 p-2.5 space-y-2"
            data-testid="active-filters-panel"
          >
            <div className="flex items-center justify-between text-[11px] font-medium text-muted-foreground">
              <span className="flex items-center gap-1.5">
                <SlidersHorizontal className="h-3 w-3 text-primary" />
                Active Filters
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={clearFilters}
                className="h-5 px-1.5 text-[10px] text-muted-foreground hover:text-destructive cursor-pointer"
              >
                Reset All
              </Button>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {activeFilterPills.map(({ key, val }) => (
                <Badge
                  key={`${key}-${val}`}
                  variant="outline"
                  className="flex items-center gap-1 bg-background/80 border-border text-[11px] py-0.5 px-1.5 font-normal text-foreground group"
                >
                  <span className="text-muted-foreground text-[10px]">{key}:</span>
                  <span className="font-mono">{val}</span>
                  <button
                    type="button"
                    onClick={() => removeFilterValue(key, val)}
                    className="ml-0.5 rounded-full p-0.5 hover:bg-muted text-muted-foreground hover:text-foreground cursor-pointer focus-visible:outline-none"
                    aria-label={`Remove filter ${key}:${val}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
            </div>
          </div>
        ) : null}

        {/* State 1: Error */}
        {isError ? (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-center space-y-2">
            <AlertCircle className="h-5 w-5 text-destructive mx-auto" />
            <div className="flex items-center justify-center gap-1">
              <span className="text-xs font-semibold text-foreground">Gagal Memuat Facet</span>
              <span
                data-testid="facet-error-code"
                className="font-mono text-[9px] bg-destructive/20 text-destructive-foreground px-1 rounded"
              >
                ERR_FACETS_UNAVAILABLE
              </span>
            </div>
            <p className="text-[11px] text-muted-foreground">
              {error?.message || 'Tidak dapat mengambil data agregasi filter.'}
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void refetch()}
              className="h-6 text-[11px] border-destructive/30 cursor-pointer"
            >
              <RefreshCw className="h-3 w-3 mr-1" />
              Coba Lagi
            </Button>
          </div>
        ) : null}

        {/* State 2: Loading Skeletons (hanya saat initial fetch ketika belum ada data) */}
        {!isError && (isLoading && !facets) ? (
          <div className="space-y-4" data-testid="facet-skeleton">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="space-y-2 animate-pulse">
                <div className="h-3 w-16 bg-muted/60 rounded" />
                <div className="space-y-1">
                  <div className="h-6 w-full bg-muted/30 rounded" />
                  <div className="h-6 w-full bg-muted/20 rounded" />
                  <div className="h-6 w-3/4 bg-muted/20 rounded" />
                </div>
              </div>
            ))}
          </div>
        ) : null}

        {/* State 3: Empty - Not Searching */}
        {!isError && !isLoading && !isSearching ? (
          <div className="space-y-2 text-center py-4 px-1">
            <SlidersHorizontal className="h-6 w-6 text-muted-foreground/60 mx-auto" />
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              Ketik pencarian untuk melihat agregasi kategori berkas, bahasa, dan tag.
            </p>
          </div>
        ) : null}

        {/* State 3: Empty - Searching but 0 facet results */}
        {!isError && !isLoading && isSearching && !hasFacets ? (
          <div className="rounded-md border border-border/50 bg-secondary/30 p-3 text-center space-y-2">
            <SearchX className="h-5 w-5 text-muted-foreground mx-auto" />
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              Tidak ada facet agregasi yang cocok dengan kriteria saat ini.
            </p>
            {totalActiveFilterCount > 0 ? (
              <Button
                variant="outline"
                size="sm"
                onClick={clearFilters}
                className="h-6 text-[11px] cursor-pointer"
              >
                Bersihkan Filter
              </Button>
            ) : null}
          </div>
        ) : null}

        {/* State 4: Success - Render Active Facet Groups */}
        {!isError && !isLoading && hasFacets && facets ? (
          <div className="space-y-4">
            {renderFacetGroup(
              'Extensions',
              <File className="h-3.5 w-3.5 text-primary/70" />,
              'extension',
              facets.extensions,
            )}
            {renderFacetGroup(
              'Type',
              <Layers className="h-3.5 w-3.5 text-primary/70" />,
              'type',
              facets.types,
            )}
            {renderFacetGroup(
              'Language',
              <FileCode className="h-3.5 w-3.5 text-primary/70" />,
              'language',
              facets.languages,
            )}
            {renderFacetGroup(
              'Tags',
              <Tag className="h-3.5 w-3.5 text-primary/70" />,
              'tag',
              facets.tags,
            )}
            {renderFacetGroup(
              'Projects',
              <FolderGit2 className="h-3.5 w-3.5 text-primary/70" />,
              'project',
              facets.projects,
            )}
          </div>
        ) : null}
      </div>

      {/* Sidebar Footer Hint */}
      <div className="border-t border-border p-2.5 text-center text-[10px] font-mono text-muted-foreground shrink-0">
        Press <kbd className="rounded border border-border bg-secondary px-1 py-0.5">[</kbd> to toggle
      </div>
    </aside>
  );
}
