import * as React from 'react';
import { Search, X, Loader2 } from 'lucide-react';
import { useIsFetching } from '@tanstack/react-query';
import { useSearchStore, type ActiveFilters } from '../../stores/searchStore';
import { useDebounce } from '../../hooks/useDebounce';
import { useSuggestQuery } from '../../hooks/useSearchQueries';
import { queryKeys } from '../../hooks/queryKeys';
import { parseQueryFilters } from '../../lib/querySync';
import { AutocompletePopover } from './AutocompletePopover';
import { cn } from '../../lib/utils';

export interface SearchBarProps {
  className?: string;
  isLoading?: boolean;
  debounceMs?: number;
}

export const SearchBar = React.forwardRef<HTMLInputElement, SearchBarProps>(
  ({ className, isLoading = false, debounceMs = 200 }, ref) => {
    const rawQuery = useSearchStore((state) => state.rawQuery);
    const setRawQuery = useSearchStore((state) => state.setRawQuery);
    const activeFilters = useSearchStore((state) => state.activeFilters);
    const setFilter = useSearchStore((state) => state.setFilter);

    const inputRef = React.useRef<HTMLInputElement | null>(null);
    const containerRef = React.useRef<HTMLDivElement | null>(null);

    const [isFocused, setIsFocused] = React.useState(false);
    const [isDismissed, setIsDismissed] = React.useState(false);
    const [highlightedIndex, setHighlightedIndex] = React.useState(-1);

    React.useImperativeHandle(ref, () => inputRef.current as HTMLInputElement);

    // Track debounce state for search
    const debouncedQuery = useDebounce(rawQuery, debounceMs);
    const isDebouncing = rawQuery !== debouncedQuery;

    // Track prefix suggestion with 150ms debounce
    const { freeText } = parseQueryFilters(rawQuery);
    const debouncedSuggestTerm = useDebounce(freeText, 150);
    const isQueryLongEnough = debouncedSuggestTerm.trim().length >= 2;

    const shouldFetchSuggest = Boolean(isFocused && !isDismissed && isQueryLongEnough);
    const { data: suggestData, isFetching: isFetchingSuggest } = useSuggestQuery(
      { q: debouncedSuggestTerm.trim(), limit: 5 },
      { enabled: shouldFetchSuggest },
    );
    const suggestions = suggestData?.suggestions ?? [];
    const isPopoverOpen = Boolean(
      isFocused && !isDismissed && isQueryLongEnough && suggestions.length > 0,
    );

    // Check if background TanStack query is fetching search results
    const isFetchingSearch = useIsFetching({ queryKey: queryKeys.search.all }) > 0;
    const showLoader = Boolean(
      isLoading ||
        isFetchingSearch ||
        (isDebouncing && rawQuery.trim().length > 0) ||
        isFetchingSuggest,
    );

    // Close popover when clicking outside
    React.useEffect(() => {
      const handleClickOutside = (event: MouseEvent) => {
        if (
          containerRef.current &&
          !containerRef.current.contains(event.target as Node)
        ) {
          setIsDismissed(true);
        }
      };

      document.addEventListener('mousedown', handleClickOutside);
      return () => {
        document.removeEventListener('mousedown', handleClickOutside);
      };
    }, []);

    const handleClear = () => {
      setRawQuery('');
      setIsDismissed(true);
      setHighlightedIndex(-1);
      inputRef.current?.focus();
    };

    const handleRemoveFilter = (key: keyof ActiveFilters) => {
      setFilter(key, null);
      inputRef.current?.focus();
    };

    const applySuggestion = (suggestion: string) => {
      let newQuery: string;
      const trimmedFree = freeText.trim();
      if (!trimmedFree || trimmedFree === rawQuery.trim()) {
        newQuery = suggestion;
      } else {
        const idx = rawQuery.lastIndexOf(trimmedFree);
        if (idx !== -1) {
          newQuery = `${rawQuery.slice(0, idx)}${suggestion}${rawQuery.slice(idx + trimmedFree.length)}`;
        } else {
          newQuery = `${rawQuery.trim()} ${suggestion}`;
        }
      }

      setRawQuery(newQuery);
      setIsDismissed(true);
      setHighlightedIndex(-1);
    };

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (suggestions.length > 0) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          if (isDismissed) {
            setIsDismissed(false);
            setHighlightedIndex(0);
            return;
          }
          setHighlightedIndex((prev) => (prev + 1) % suggestions.length);
          return;
        }
        if (e.key === 'ArrowUp' && isPopoverOpen) {
          e.preventDefault();
          setHighlightedIndex((prev) =>
            prev <= 0 ? suggestions.length - 1 : prev - 1,
          );
          return;
        }
        if (e.key === 'Enter' && isPopoverOpen) {
          if (highlightedIndex >= 0 && suggestions[highlightedIndex]) {
            e.preventDefault();
            applySuggestion(suggestions[highlightedIndex]);
            return;
          }
        }
        if (e.key === 'Escape' && isPopoverOpen) {
          e.preventDefault();
          setIsDismissed(true);
          setHighlightedIndex(-1);
          return;
        }
      }

      if (e.key === 'Escape') {
        e.preventDefault();
        if (rawQuery) {
          setRawQuery('');
        } else {
          inputRef.current?.blur();
        }
      }
    };

    const filterEntries = Object.entries(activeFilters).filter(
      ([, value]) => Boolean(value),
    ) as [keyof ActiveFilters, string][];

    return (
      <div
        ref={containerRef}
        className={cn(
          'relative flex items-center min-h-9 h-auto w-full max-w-xl rounded-md border border-border bg-card/90 px-2.5 py-1 shadow-xs transition-colors focus-within:border-primary/60 focus-within:ring-1 focus-within:ring-primary/40 gap-1.5 flex-wrap sm:flex-nowrap',
          className,
        )}
        onClick={() => inputRef.current?.focus()}
      >
        {/* Leading Search or Loader indicator */}
        <div className="flex items-center shrink-0">
          {showLoader ? (
            <Loader2
              data-testid="search-loader"
              className="h-4 w-4 shrink-0 text-primary animate-spin"
            />
          ) : (
            <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
          )}
        </div>

        {/* Active Filter Chips Scaffold */}
        {filterEntries.length > 0 ? (
          <div className="flex items-center gap-1 shrink-0 overflow-x-auto max-w-[200px] scrollbar-none py-0.5">
            {filterEntries.map(([key, value]) => (
              <span
                key={key}
                data-testid={`filter-chip-${key}`}
                className="inline-flex items-center gap-1 rounded bg-secondary/80 border border-border/80 px-1.5 py-0.5 text-[11px] font-mono text-secondary-foreground shrink-0 select-none animate-in fade-in duration-100"
              >
                <span className="text-muted-foreground">{key}:</span>
                <span className="font-semibold text-foreground">{value}</span>
                <button
                  type="button"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    handleRemoveFilter(key);
                  }}
                  className="rounded hover:bg-muted text-muted-foreground hover:text-foreground cursor-pointer p-0.5 transition-colors"
                  aria-label={`Remove filter ${key}`}
                >
                  <X className="h-2.5 w-2.5" />
                </button>
              </span>
            ))}
          </div>
        ) : null}

        {/* Global Query Input */}
        <input
          ref={inputRef}
          type="text"
          value={rawQuery}
          onChange={(e) => {
            setRawQuery(e.target.value);
            setIsDismissed(false);
            setHighlightedIndex(-1);
          }}
          onFocus={() => {
            setIsFocused(true);
          }}
          onBlur={(e) => {
            if (!containerRef.current?.contains(e.relatedTarget as Node)) {
              setIsFocused(false);
            }
          }}
          onKeyDown={handleKeyDown}
          placeholder="Search code, docs, tags... (Press / or ⌘K)"
          aria-label="Pencarian dokumen, kode, dan tag"
          aria-autocomplete="list"
          aria-controls={isPopoverOpen ? 'search-autocomplete-listbox' : undefined}
          aria-expanded={isPopoverOpen}
          aria-activedescendant={
            highlightedIndex >= 0 ? `autocomplete-item-${highlightedIndex}` : undefined
          }
          className="flex-1 min-w-[120px] bg-transparent text-sm text-foreground placeholder:text-muted-foreground outline-hidden focus:outline-hidden"
          spellCheck={false}
          autoComplete="off"
        />

        {/* Clear query action */}
        {rawQuery ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleClear();
            }}
            className="flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground transition-colors mr-0.5 cursor-pointer shrink-0"
            aria-label="Clear search input"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        ) : null}

        {/* Keyboard shortcut hint */}
        <div className="flex items-center gap-1 shrink-0">
          <kbd className="pointer-events-none hidden sm:inline-flex h-5 select-none items-center gap-0.5 rounded border border-border bg-secondary px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
            <span className="text-xs">⌘</span>K
          </kbd>
        </div>

        {/* Autocomplete Popover Dropdown */}
        <AutocompletePopover
          isOpen={isPopoverOpen}
          suggestions={suggestions}
          query={debouncedSuggestTerm}
          highlightedIndex={highlightedIndex}
          onSelect={applySuggestion}
        />
      </div>
    );
  },
);

SearchBar.displayName = 'SearchBar';
