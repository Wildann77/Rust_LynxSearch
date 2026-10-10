import * as React from 'react';
import { FileCode, FileText, SlidersHorizontal, Hash } from 'lucide-react';
import type { SearchResultItem } from '../../types/search';
import { SafeHighlight } from './SafeHighlight';
import { formatBytes, formatScore } from '../../lib/format';
import { formatRelativeTime } from '../../lib/date';
import { cn } from '../../lib/utils';

export interface ResultCardProps {
  item: SearchResultItem;
  isSelected?: boolean;
  onSelect: (item: SearchResultItem, lineNumber?: number) => void;
  className?: string;
}

export const ResultCard = React.forwardRef<HTMLDivElement, ResultCardProps>(
  ({ item, isSelected = false, onSelect, className }, ref) => {
    // Choose icon by document type
    const renderTypeIcon = () => {
      switch (item.type) {
        case 'code':
          return <FileCode className="h-4 w-4 text-emerald-400 shrink-0" />;
        case 'config':
          return <SlidersHorizontal className="h-4 w-4 text-amber-400 shrink-0" />;
        case 'doc':
        default:
          return <FileText className="h-4 w-4 text-blue-400 shrink-0" />;
      }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onSelect(item);
      }
    };

    return (
      <div
        ref={ref}
        role="button"
        tabIndex={0}
        aria-selected={isSelected}
        onClick={() => onSelect(item)}
        onKeyDown={handleKeyDown}
        className={cn(
          'group relative flex flex-col gap-2 rounded-md border p-3 text-left transition-all cursor-pointer select-none outline-hidden focus-visible:ring-2 focus-visible:ring-primary/50',
          isSelected
            ? 'border-primary/60 bg-secondary/70 ring-1 ring-primary/40 shadow-xs'
            : 'border-border/70 bg-card/50 hover:bg-card/90 hover:border-border',
          className,
        )}
      >
        {/* Header: Type icon, Title, Language Badge, Score Badge */}
        <div className="flex items-center justify-between gap-2 min-w-0">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            {renderTypeIcon()}
            <h3 className="font-semibold text-sm text-foreground tracking-tight truncate group-hover:text-primary transition-colors">
              {item.title}
            </h3>
            {item.language ? (
              <span className="shrink-0 rounded bg-secondary/80 border border-border/60 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground uppercase">
                {item.language}
              </span>
            ) : null}
            {item.project ? (
              <span className="shrink-0 rounded bg-muted/70 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground truncate max-w-[120px]">
                {item.project}
              </span>
            ) : null}
          </div>

          <div className="flex items-center gap-1.5 shrink-0">
            <div
              data-testid="relevance-indicator"
              className="flex items-center gap-1 rounded bg-secondary/60 border border-border/50 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground"
              title={`BM25 Relevance Score: ${formatScore(item.score)}`}
            >
              <span
                className={cn(
                  'h-1.5 w-1.5 rounded-full',
                  item.score > 5
                    ? 'bg-emerald-400'
                    : item.score > 2
                      ? 'bg-blue-400'
                      : 'bg-neutral-400',
                )}
              />
              <span data-testid="result-score">Score {formatScore(item.score)}</span>
            </div>
          </div>
        </div>

        {/* Relative Path */}
        <div className="font-mono text-xs text-muted-foreground/80 truncate">
          {item.relative_path}
        </div>

        {/* Snippet & Highlights */}
        {item.highlights && item.highlights.length > 0 ? (
          <div className="flex flex-col gap-1.5 bg-background/60 rounded p-2 border border-border/40">
            {item.highlights.slice(0, 3).map((h, idx) => (
              <div
                key={idx}
                className={cn(
                  'flex items-start gap-2 text-xs font-mono rounded p-0.5 transition-colors',
                  h.line_number != null && 'cursor-pointer hover:bg-primary/10',
                )}
                onClick={(e) => {
                  if (h.line_number != null) {
                    e.stopPropagation();
                    onSelect(item, h.line_number);
                  }
                }}
              >
                {h.line_number != null ? (
                  <span
                    data-testid="result-line-number"
                    className="shrink-0 rounded bg-primary/10 border border-primary/25 px-1 py-0.2 text-[10px] font-semibold text-primary font-mono select-none"
                  >
                    L{h.line_number}
                  </span>
                ) : null}
                <SafeHighlight
                  snippet={h.snippet}
                  className="text-xs text-neutral-300 leading-relaxed font-sans line-clamp-2"
                />
              </div>
            ))}
          </div>
        ) : null}

        {/* Footer: Tags and File Metadata */}
        <div className="flex items-center justify-between gap-2 text-[11px] font-mono text-muted-foreground pt-1 border-t border-border/30">
          <div className="flex items-center gap-1.5 overflow-x-auto scrollbar-none py-0.5">
            {item.tags && item.tags.length > 0 ? (
              item.tags.map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-0.5 text-muted-foreground/90 hover:text-foreground transition-colors"
                >
                  <Hash className="h-2.5 w-2.5 opacity-60" />
                  {tag}
                </span>
              ))
            ) : (
              <span className="text-muted-foreground/50 text-[10px]">-</span>
            )}
          </div>

          <div className="flex items-center gap-2.5 shrink-0 text-[10px] text-muted-foreground/80">
            <span>{formatBytes(item.file_size)}</span>
            {item.updated_at ? <span>{formatRelativeTime(item.updated_at)}</span> : null}
          </div>
        </div>
      </div>
    );
  },
);

ResultCard.displayName = 'ResultCard';
