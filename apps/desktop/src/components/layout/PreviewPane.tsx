import { FileText, X, ChevronRight } from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useSearchStore } from '../../stores/searchStore';
import { DocumentPreview } from '../preview/DocumentPreview';
import { Button } from '../ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { cn } from '../../lib/utils';

export interface PreviewPaneProps {
  className?: string;
}

export function PreviewPane({ className }: PreviewPaneProps) {
  const { previewCollapsed, previewWidth, togglePreview } = useUIStore();
  const selectedDocId = useSearchStore((state) => state.selectedDocId);

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
        <DocumentPreview documentId={selectedDocId} onClose={togglePreview} />
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
