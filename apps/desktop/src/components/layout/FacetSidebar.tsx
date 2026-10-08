import * as React from 'react';
import {
  SlidersHorizontal,
  ChevronLeft,
  Tag,
  FileCode,
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
  const clearFilters = useSearchStore((state) => state.clearFilters);
  const page = useSearchStore((state) => state.page);
  const pageSize = useSearchStore((state) => state.pageSize);

  const debouncedQuery = useDebounce(rawQuery, 200);
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

  if (sidebarCollapsed) {
    return null;
  }

  const facets = data?.facets;
  const hasFacets = Boolean(
    facets &&
      (facets.types.length > 0 ||
        facets.languages.length > 0 ||
        facets.tags.length > 0 ||
        facets.projects.length > 0),
  );

  const activeFilterCount = Object.values(activeFilters).filter(Boolean).length;

  const handleToggleFacet = (key: keyof ActiveFilters, value: string) => {
    if (activeFilters[key] === value) {
      setFilter(key, null);
    } else {
      setFilter(key, value);
    }
  };

  const renderFacetGroup = (
    title: string,
    icon: React.ReactNode,
    filterKey: keyof ActiveFilters,
    buckets: FacetBucket[],
  ) => {
    if (buckets.length === 0) return null;
    const currentActive = activeFilters[filterKey];

    return (
      <div className="space-y-1.5" data-testid={`facet-group-${filterKey}`}>
        <div className="flex items-center justify-between text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
          <div className="flex items-center gap-1.5">
            {icon}
            <span>{title}</span>
          </div>
          {currentActive ? (
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
            const isSelected = currentActive === bucket.key;
            return (
              <button
                key={bucket.key}
                type="button"
                onClick={() => handleToggleFacet(filterKey, bucket.key)}
                className={cn(
                  'flex items-center justify-between rounded px-2 py-1 text-xs transition-colors cursor-pointer text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary',
                  isSelected
                    ? 'bg-primary/15 text-primary font-medium border border-primary/30'
                    : 'text-neutral-300 hover:bg-secondary/60 hover:text-foreground border border-transparent',
                )}
                aria-pressed={isSelected}
              >
                <div className="flex items-center gap-1.5 truncate">
                  {isSelected ? <Check className="h-3 w-3 shrink-0" /> : null}
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
          {activeFilterCount > 0 ? (
            <Badge variant="outline" className="text-[10px] px-1 py-0 h-4 font-mono text-primary border-primary/40">
              {activeFilterCount}
            </Badge>
          ) : null}
        </div>

        <div className="flex items-center gap-1">
          {activeFilterCount > 0 ? (
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

        {/* State 2: Loading Skeletons */}
        {!isError && (isLoading || (isFetching && !facets)) ? (
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
            {activeFilterCount > 0 ? (
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
