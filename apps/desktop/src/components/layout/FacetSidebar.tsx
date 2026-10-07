import { SlidersHorizontal, ChevronLeft, Tag, FileCode, Layers, FileType } from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface FacetSidebarProps {
  className?: string;
}

export function FacetSidebar({ className }: FacetSidebarProps) {
  const { sidebarCollapsed, toggleSidebar } = useUIStore();

  if (sidebarCollapsed) {
    return null;
  }

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
        </div>
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

      {/* Facets Placeholder Content */}
      <div className="flex-1 overflow-y-auto p-3 space-y-5 text-xs">
        {/* Extension Filter Placeholder */}
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 font-medium text-muted-foreground text-[11px] uppercase tracking-wider">
            <FileType className="h-3.5 w-3.5 text-primary/70" />
            <span>Extension</span>
          </div>
          <div className="rounded-md border border-border/50 bg-secondary/30 p-2.5 text-[11px] text-muted-foreground/80 leading-relaxed">
            No facets yet. Perform a search to aggregate file extensions.
          </div>
        </div>

        {/* Language Filter Placeholder */}
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 font-medium text-muted-foreground text-[11px] uppercase tracking-wider">
            <FileCode className="h-3.5 w-3.5 text-primary/70" />
            <span>Language</span>
          </div>
          <div className="rounded-md border border-border/50 bg-secondary/30 p-2.5 text-[11px] text-muted-foreground/80 leading-relaxed">
            Language facets (.rs, .ts, .md, .py) will appear here.
          </div>
        </div>

        {/* Document Type Placeholder */}
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 font-medium text-muted-foreground text-[11px] uppercase tracking-wider">
            <Layers className="h-3.5 w-3.5 text-primary/70" />
            <span>Type</span>
          </div>
          <div className="rounded-md border border-border/50 bg-secondary/30 p-2.5 text-[11px] text-muted-foreground/80 leading-relaxed">
            Code vs. Documentation grouping.
          </div>
        </div>

        {/* Tags Placeholder */}
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 font-medium text-muted-foreground text-[11px] uppercase tracking-wider">
            <Tag className="h-3.5 w-3.5 text-primary/70" />
            <span>Tags</span>
          </div>
          <div className="rounded-md border border-border/50 bg-secondary/30 p-2.5 text-[11px] text-muted-foreground/80 leading-relaxed">
            Front-matter tags (#backend, #auth, etc.).
          </div>
        </div>
      </div>

      {/* Sidebar Footer Hint */}
      <div className="border-t border-border p-2.5 text-center text-[10px] font-mono text-muted-foreground shrink-0">
        Press <kbd className="rounded border border-border bg-secondary px-1 py-0.5">[</kbd> to toggle
      </div>
    </aside>
  );
}
