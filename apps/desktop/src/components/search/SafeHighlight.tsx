import React, { useMemo } from 'react';
import { cn } from '@/lib/utils';

export interface HighlightSegment {
  text: string;
  isMatch: boolean;
}

export interface SafeHighlightProps extends React.HTMLAttributes<HTMLElement> {
  snippet: string;
  as?: React.ElementType;
  markClassName?: string;
}

export function parseHighlightMarkers(snippet: string): HighlightSegment[] {
  if (!snippet) {
    return [];
  }

  const parts = snippet.split(/(<em>[\s\S]*?<\/em>)/g);
  const segments: HighlightSegment[] = [];

  for (const part of parts) {
    if (!part) continue;

    if (part.startsWith('<em>') && part.endsWith('</em>')) {
      const matchText = part.slice(4, -5);
      if (matchText) {
        segments.push({ text: matchText, isMatch: true });
      }
    } else {
      segments.push({ text: part, isMatch: false });
    }
  }

  return segments;
}

export function SafeHighlight({
  snippet,
  as: Component = 'span',
  className,
  markClassName,
  ...rest
}: SafeHighlightProps) {
  const segments = useMemo(() => parseHighlightMarkers(snippet), [snippet]);

  if (!snippet) {
    return null;
  }

  return (
    <Component className={cn('text-sm text-neutral-300 leading-relaxed', className)} {...rest}>
      {segments.map((seg, idx) =>
        seg.isMatch ? (
          <mark
            key={idx}
            className={cn(
              'bg-amber-500/20 text-amber-200 font-medium rounded-xs px-0.5',
              markClassName
            )}
          >
            {seg.text}
          </mark>
        ) : (
          <React.Fragment key={idx}>{seg.text}</React.Fragment>
        )
      )}
    </Component>
  );
}
