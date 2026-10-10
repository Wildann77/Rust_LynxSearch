import * as React from 'react';
import { FileText, X, ChevronRight } from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useSearchStore } from '../../stores/searchStore';
import { Button } from '../ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { Skeleton } from '../ui/skeleton';
import { cn } from '../../lib/utils';

// Lazy load heavy DocumentPreview (and its Shiki / markdown dependencies)
const DocumentPreview = React.lazy(() =>
  import('../preview/DocumentPreview').then((m) => ({ default: m.DocumentPreview }))
);

function PreviewSkeletonFallback({ onClose }: { onClose: () => void }) {
  return (
    <div className="flex flex-col h-full bg-card/20 select-none animate-pulse" data-testid="preview-lazy-skeleton">
      <div className="flex h-12 items-center justify-between border-b border-border px-3 shrink-0">
        <div className="flex items-center gap-2">
          <Skeleton className="h-4 w-4 rounded" />
          <Skeleton className="h-4 w-32" />
        </div>
        <div className="flex items-center gap-1">
          <Skeleton className="h-6 w-6 rounded" />
          <Skeleton className="h-6 w-6 rounded" />
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="h-6 w-6 text-muted-foreground hover:text-foreground cursor-pointer"
            aria-label="Close preview panel (])"
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
      <div className="p-4 space-y-3 flex-1 overflow-hidden">
        <Skeleton className="h-6 w-3/4" />
        <Skeleton className="h-4 w-1/2" />
        <div className="pt-4 space-y-2">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-5/6" />
          <Skeleton className="h-4 w-4/6" />
          <Skeleton className="h-4 w-full" />
        </div>
      </div>
    </div>
  );
}

export interface PreviewPaneProps {
  className?: string;
}

export function PreviewPane({ className }: PreviewPaneProps) {
  const { previewCollapsed, previewWidth, togglePreview } = useUIStore();
  const selectedDocId = useSearchStore((state) => state.selectedDocId);
  const selectedLineNumber = useSearchStore((state) => state.selectedLineNumber);

  if (previewCollapsed) {
    return null;
  }

  return (
    <aside
      style={{ width: `${previewWidth}px` }}
      className={cn(
        'shrink-0 border-l border-border bg-card/30 flex flex-col h-full select-none overflow-hidden transition-[width] duration-75',
        className,
      )}
      aria-label="Document Preview Panel"
    >
      {selectedDocId ? (
        <React.Suspense fallback={<PreviewSkeletonFallback onClose={togglePreview} />}>
          <DocumentPreview
            documentId={selectedDocId}
            targetLine={selectedLineNumber}
            onClose={togglePreview}
          />
        </React.Suspense>
      ) : (
        <>
          {/* Preview Header (Empty State) */}
          <div className="flex h-10 items-center justify-between border-b border-border px-3 shrink-0 bg-card/40">
            <div className="flex items-center gap-2 text-xs font-semibold tracking-tight text-foreground">
              <FileText className="h-3.5 w-3.5 text-primary" />
              <span>Preview</span>
              <span className="font-mono text-[10px] text-muted-foreground">({previewWidth}px)</span>
            </div>

            <div className="flex items-center gap-1">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={togglePreview}
                    className="h-6 w-6 text-muted-foreground hover:text-foreground cursor-pointer"
                    aria-label="Close preview panel (])"
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Close Preview (])</TooltipContent>
              </Tooltip>
            </div>
          </div>

          {/* Preview Empty State Content */}
          <div className="flex-1 overflow-y-auto p-6 flex flex-col items-center justify-center text-center">
            <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-secondary/80 border border-border text-primary/80 mb-3 shadow-inner">
              <FileText className="h-6 w-6" />
            </div>

            <h3 className="text-sm font-semibold text-foreground tracking-tight mb-1">
              Pilih dokumen untuk melihat preview
            </h3>
            <p className="text-xs text-muted-foreground max-w-xs leading-relaxed mb-4">
              Pilih salah satu item hasil pencarian untuk melihat format Markdown atau penomoran baris source code virtual.
            </p>

            <div className="flex items-center gap-1.5 text-[11px] font-mono text-muted-foreground/80 border border-border/60 bg-secondary/30 rounded-md px-2.5 py-1">
              <span>Tutup preview:</span>
              <kbd className="rounded border border-border bg-card px-1 py-0.5 text-[10px]">]</kbd>
            </div>
          </div>
        </>
      )}

      {/* Preview Footer Width Hint */}
      <div className="border-t border-border px-3 py-1.5 flex items-center justify-between text-[10px] font-mono text-muted-foreground shrink-0 bg-card/20">
        <span>Drag divider to resize (420–640px)</span>
        <Button
          variant="ghost"
          size="sm"
          onClick={togglePreview}
          className="h-5 px-1.5 text-[10px] text-muted-foreground hover:text-foreground cursor-pointer"
        >
          <span>Collapse</span>
          <ChevronRight className="h-3 w-3 ml-0.5" />
        </Button>
      </div>
    </aside>
  );
}
