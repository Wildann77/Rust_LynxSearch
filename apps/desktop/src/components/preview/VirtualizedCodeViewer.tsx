import * as React from 'react';
import { useVirtualizer, observeElementRect } from '@tanstack/react-virtual';
import { type ThemedToken } from '@shikijs/core';
import { highlightCode, splitPlainLines, type TokenLine } from '../../lib/shiki';
import { cn } from '../../lib/utils';

export interface VirtualizedCodeViewerProps {
  code: string;
  language?: string | null;
  highlightLine?: number | null;
  searchTerms?: string[];
  className?: string;
  onLineClick?: (lineNumber: number) => void;
}

const ESTIMATED_ROW_HEIGHT = 22;
const OVERSCAN_COUNT = 20;

function renderToken(
  token: ThemedToken,
  tokenIdx: number,
  searchTerms: string[],
): React.ReactNode {
  if (!token.content) return null;

  if (searchTerms.length === 0) {
    return (
      <span
        key={tokenIdx}
        style={{
          color: token.color,
          fontStyle: token.fontStyle && token.fontStyle & 1 ? 'italic' : undefined,
        }}
      >
        {token.content}
      </span>
    );
  }

  const escapedTerms = searchTerms
    .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .join('|');
  const regex = new RegExp(`(${escapedTerms})`, 'gi');
  const parts = token.content.split(regex);

  return (
    <span
      key={tokenIdx}
      style={{
        color: token.color,
        fontStyle: token.fontStyle && token.fontStyle & 1 ? 'italic' : undefined,
      }}
    >
      {parts.map((part, partIdx) => {
        if (searchTerms.includes(part.toLowerCase())) {
          return (
            <mark
              key={partIdx}
              className="bg-emerald-500/30 text-emerald-300 font-semibold rounded-xs px-0.5"
            >
              {part}
            </mark>
          );
        }
        return part;
      })}
    </span>
  );
}

export function VirtualizedCodeViewer({
  code,
  language,
  highlightLine,
  searchTerms = [],
  className,
  onLineClick,
}: VirtualizedCodeViewerProps) {
  const parentRef = React.useRef<HTMLDivElement>(null);

  // Synchronous plain-lines fallback for instant initial render
  const [tokenLines, setTokenLines] = React.useState<TokenLine[]>(() =>
    splitPlainLines(code),
  );

  React.useEffect(() => {
    let cancelled = false;

    // Set synchronous fallback first when code changes
    setTokenLines(splitPlainLines(code));

    // Highlight asynchronously via Shiki
    void highlightCode(code, language).then((highlighted) => {
      if (!cancelled) {
        setTokenLines(highlighted);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [code, language]);

  // eslint-disable-next-line react-hooks/incompatible-library
  const rowVirtualizer = useVirtualizer({
    count: tokenLines.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ESTIMATED_ROW_HEIGHT,
    overscan: OVERSCAN_COUNT,
    initialRect: { width: 800, height: 600 },
    observeElementRect: (instance, cb) => {
      return observeElementRect(instance, (rect) => {
        const height =
          rect.height > 0
            ? rect.height
            : instance.scrollElement?.clientHeight || 600;
        const width =
          rect.width > 0
            ? rect.width
            : instance.scrollElement?.clientWidth || 800;
        cb({ width, height });
      });
    },
  });

  // Auto-scroll to target line when highlightLine is provided
  React.useEffect(() => {
    if (
      highlightLine != null &&
      highlightLine > 0 &&
      highlightLine <= tokenLines.length
    ) {
      rowVirtualizer.scrollToIndex(highlightLine - 1, {
        align: 'center',
        behavior: 'smooth',
      });
    }
  }, [highlightLine, tokenLines.length, rowVirtualizer]);

  const cleanSearchTerms = React.useMemo(() => {
    return searchTerms
      .map((t) => t.trim().toLowerCase())
      .filter((t) => t.length > 0 && !t.includes(':'));
  }, [searchTerms]);

  return (
    <div
      ref={parentRef}
      role="region"
      aria-label="Source code viewer"
      tabIndex={0}
      className={cn(
        'h-full w-full overflow-auto bg-black/60 font-mono text-xs leading-snug select-text outline-hidden',
        className,
      )}
    >
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const lineNumber = virtualRow.index + 1;
          const isHighlighted = highlightLine === lineNumber;
          const lineTokens = tokenLines[virtualRow.index] ?? [{ content: '', offset: 0 }];

          return (
            <div
              key={virtualRow.index}
              data-index={virtualRow.index}
              data-line-number={lineNumber}
              data-testid="code-line-row"
              onClick={() => onLineClick?.(lineNumber)}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              className={cn(
                'flex items-center px-3 hover:bg-neutral-900/60 transition-colors',
                isHighlighted &&
                  'bg-emerald-500/15 border-l-2 border-emerald-500',
              )}
            >
              {/* Line Number Gutter */}
              <span className="w-12 shrink-0 pr-3 text-right text-[11px] text-neutral-500 select-none font-mono">
                {lineNumber}
              </span>

              {/* Code Tokens Content */}
              <pre className="font-mono text-neutral-200 overflow-visible whitespace-pre m-0 p-0 text-[11px] leading-5 flex-1 min-w-0">
                {lineTokens.map((token, idx) =>
                  renderToken(token, idx, cleanSearchTerms),
                )}
              </pre>
            </div>
          );
        })}
      </div>
    </div>
  );
}
