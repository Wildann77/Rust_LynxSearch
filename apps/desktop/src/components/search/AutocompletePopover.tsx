import * as React from 'react';
import { Search } from 'lucide-react';
import { cn } from '../../lib/utils';

export interface AutocompletePopoverProps {
  isOpen: boolean;
  suggestions: string[];
  query: string;
  highlightedIndex: number;
  onSelect: (suggestion: string) => void;
  className?: string;
}

function SuggestionItemText({
  text,
  query,
  isSelected,
}: {
  text: string;
  query: string;
  isSelected: boolean;
}) {
  const q = query.trim().toLowerCase();
  const idx = text.toLowerCase().indexOf(q);

  if (idx === -1 || !q) {
    return <span className={cn('truncate', isSelected ? 'text-foreground' : 'text-muted-foreground')}>{text}</span>;
  }

  const before = text.slice(0, idx);
  const match = text.slice(idx, idx + q.length);
  const after = text.slice(idx + q.length);

  return (
    <span className={cn('truncate', isSelected ? 'text-foreground font-medium' : 'text-muted-foreground')}>
      {before}
      <span
        className={cn(
          'font-semibold underline decoration-primary/50 underline-offset-2',
          isSelected ? 'text-primary' : 'text-foreground',
        )}
      >
        {match}
      </span>
      {after}
    </span>
  );
}

export const AutocompletePopover: React.FC<AutocompletePopoverProps> = ({
  isOpen,
  suggestions,
  query,
  highlightedIndex,
  onSelect,
  className,
}) => {
  if (!isOpen || suggestions.length === 0) {
    return null;
  }

  return (
    <div
      role="listbox"
      id="search-autocomplete-listbox"
      aria-label="Saran Autocomplete"
      data-testid="autocomplete-popover"
      className={cn(
        'absolute left-0 right-0 top-[calc(100%+6px)] z-50 overflow-hidden rounded-lg border border-border/80 bg-card/95 backdrop-blur-md shadow-2xl py-1 animate-in fade-in-50 zoom-in-95 duration-150',
        className,
      )}
    >
      <div className="px-2.5 py-1 text-[11px] font-medium tracking-wider uppercase text-muted-foreground/70 select-none">
        Saran Pencarian
      </div>
      <ul className="flex flex-col gap-0.5 p-1 m-0 list-none">
        {suggestions.map((suggestion, index) => {
          const isSelected = index === highlightedIndex;
          return (
            <li
              key={`${suggestion}-${index}`}
              id={`autocomplete-item-${index}`}
              role="option"
              aria-selected={isSelected}
              data-testid={`autocomplete-item-${index}`}
              onMouseDown={(e) => {
                // Prevent input blur before click fires
                e.preventDefault();
                onSelect(suggestion);
              }}
              className={cn(
                'group flex items-center justify-between rounded-md px-2.5 py-1.5 text-xs cursor-pointer transition-colors select-none',
                isSelected
                  ? 'bg-secondary/90 text-foreground ring-1 ring-primary/50'
                  : 'text-muted-foreground hover:text-foreground hover:bg-secondary/50',
              )}
            >
              <div className="flex items-center gap-2 min-w-0">
                <Search
                  className={cn(
                    'h-3.5 w-3.5 shrink-0 transition-colors',
                    isSelected ? 'text-primary' : 'text-muted-foreground/70',
                  )}
                />
                <SuggestionItemText text={suggestion} query={query} isSelected={isSelected} />
              </div>
              <span
                className={cn(
                  'text-[10px] font-mono px-1.5 py-0.5 rounded border transition-all shrink-0 ml-2',
                  isSelected
                    ? 'opacity-100 bg-background/70 text-primary border-primary/40 font-medium'
                    : 'opacity-0 group-hover:opacity-100 text-muted-foreground border-border/40 bg-muted/40',
                )}
              >
                ↵ pilih
              </span>
            </li>
          );
        })}
      </ul>
      <div className="flex items-center justify-between px-3 py-1 text-[10px] text-muted-foreground/60 border-t border-border/40 font-mono bg-muted/20 select-none">
        <span>↑↓ navigasi</span>
        <span>↵ pilih</span>
        <span>esc tutup</span>
      </div>
    </div>
  );
};
