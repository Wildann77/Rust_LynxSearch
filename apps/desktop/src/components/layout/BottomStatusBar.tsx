import { useHealthQuery } from '../../hooks/useHealthQuery';
import { useJobQuery } from '../../hooks/useJobQuery';
import { useUIStore } from '../../stores/uiStore';
import { cn } from '../../lib/utils';

export interface BottomStatusBarProps {
  className?: string;
}

export function BottomStatusBar({ className }: BottomStatusBarProps) {
  const indexingJobId = useUIStore((state) => state.indexingJobId);
  const toggleJobDrawer = useUIStore((state) => state.toggleJobDrawer);
  const { data: health } = useHealthQuery({ refetchInterval: 15_000 });
  const { data: job } = useJobQuery(indexingJobId);

  return (
    <footer
      className={cn(
        'flex h-7 w-full shrink-0 items-center justify-between border-t border-border bg-card/70 px-3 text-[11px] font-mono text-muted-foreground select-none overflow-hidden',
        className,
      )}
      aria-label="Application Status and Hotkeys Bar"
    >
      {/* Left: Keyboard Hotkey Guide */}
      <div className="flex items-center gap-3 overflow-hidden text-ellipsis whitespace-nowrap">
        <span className="flex items-center gap-1">
          <kbd className="rounded border border-border bg-secondary px-1 py-0.2 text-[10px]">⌘K</kbd>
          <span className="hidden sm:inline">Search</span>
        </span>

        <span className="text-border hidden sm:inline">•</span>

        <span className="hidden sm:flex items-center gap-1">
          <kbd className="rounded border border-border bg-secondary px-1 py-0.2 text-[10px]">j/k</kbd>
          <span>Navigate</span>
        </span>

        <span className="text-border hidden md:inline">•</span>

        <span className="hidden md:flex items-center gap-1">
          <kbd className="rounded border border-border bg-secondary px-1 py-0.2 text-[10px]">Enter</kbd>
          <span>Preview</span>
        </span>

        <span className="text-border hidden lg:inline">•</span>

        <span className="hidden lg:flex items-center gap-1">
          <kbd className="rounded border border-border bg-secondary px-1 py-0.2 text-[10px]">[</kbd>
          <span>Sidebar</span>
        </span>

        <span className="text-border hidden lg:inline">•</span>

        <span className="hidden lg:flex items-center gap-1">
          <kbd className="rounded border border-border bg-secondary px-1 py-0.2 text-[10px]">]</kbd>
          <span>Preview</span>
        </span>
      </div>

      {/* Right: Runtime Info / Job Status */}
      <div className="flex items-center gap-3 shrink-0 ml-2">
        {job?.status === 'RUNNING' ? (
          <button
            type="button"
            onClick={toggleJobDrawer}
            className="text-amber-400 font-semibold flex items-center gap-1.5 animate-pulse hover:opacity-80 cursor-pointer focus:outline-none"
            title="Klik untuk membuka/menutup rincian progres pekerjaan"
          >
            <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
            <span>Indexing: {job.processed_files}/{job.total_files} files</span>
          </button>
        ) : (
          <span className="text-[10px] text-muted-foreground/80">
            {health?.status === 'ok' ? 'ES 8.19 + Postgres Connected' : 'Ready'}
          </span>
        )}
      </div>
    </footer>
  );
}
