import { useHealthQuery } from '../../hooks/useHealthQuery';
import { useJobQuery } from '../../hooks/useJobQuery';
import { useUIStore } from '../../stores/uiStore';
import { BACKEND_URL } from '../../api/config';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { cn } from '../../lib/utils';

export interface HealthDotProps {
  className?: string;
  showLabel?: boolean;
  retry?: number | boolean;
}

export type HealthDotStatus = 'healthy' | 'indexing' | 'degraded' | 'offline';

export function HealthDot({ className, showLabel = true, retry }: HealthDotProps) {
  const indexingJobId = useUIStore((state) => state.indexingJobId);
  const { data: health, isLoading, isSuccess, isError, isRefetching, refetch } = useHealthQuery({
    refetchInterval: 10_000,
    retry,
  });
  const { data: job } = useJobQuery(indexingJobId);

  // Status mapping as required by 7.16:
  // Green = backend healthy.
  // Yellow = indexing state as defined by UI (or connecting / degraded).
  // Red = backend unavailable.
  const isJobRunning = job?.status === 'RUNNING';
  const isConnecting = (isLoading && !health) || (!isSuccess && isRefetching);
  const isOffline = isError || (!isLoading && !isSuccess);

  let status: HealthDotStatus = 'healthy';
  let dotColor = 'bg-[#10a37f]'; // Green
  let labelText = 'Ready';
  let tooltipText = `Backend Ready (${health?.version ?? 'v1.0.0'})`;

  if (isOffline) {
    status = 'offline';
    dotColor = 'bg-destructive shadow-[0_0_8px_rgba(239,68,68,0.5)]'; // Red
    labelText = 'Offline';
    tooltipText = `Backend Offline (${BACKEND_URL.replace(/^https?:\/\//, '')})`;
  } else if (isJobRunning) {
    status = 'indexing';
    dotColor = 'bg-amber-400 animate-pulse shadow-[0_0_8px_rgba(251,191,36,0.5)]'; // Yellow
    labelText = 'Indexing...';
    tooltipText = `Sedang mengindeks berkas (${job?.processed_files ?? 0}/${job?.total_files ?? 0})`;
  } else if (isConnecting) {
    status = 'indexing';
    dotColor = 'bg-amber-400 animate-pulse'; // Yellow
    labelText = 'Connecting...';
    tooltipText = 'Menghubungkan ke backend...';
  } else if (health?.status !== 'ok') {
    status = 'degraded';
    dotColor = 'bg-amber-500'; // Degraded yellow/amber
    labelText = 'Degraded';
    tooltipText = 'Backend Degraded (Storage tidak sepenuhnya siap)';
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={() => void refetch()}
          className={cn(
            'flex items-center gap-1.5 px-2 py-1 rounded-md hover:bg-secondary/50 cursor-pointer transition-all duration-200 focus-visible:ring-1 focus-visible:ring-primary focus-visible:outline-none select-none',
            className,
          )}
          aria-label={`Backend Status: ${labelText}`}
          data-testid="health-dot-button"
          data-status={status}
        >
          <span
            className={cn('h-2.5 w-2.5 rounded-full shrink-0 transition-colors duration-300', dotColor)}
            data-testid="health-dot-indicator"
          />
          {showLabel && (
            <span className="hidden xl:inline-block text-[11px] font-mono text-muted-foreground truncate w-[72px] text-left shrink-0 transition-opacity duration-200">
              {labelText}
            </span>
          )}
          {isRefetching && !isConnecting && (
            <span
              className="h-1.5 w-1.5 rounded-full bg-primary/40 animate-pulse shrink-0"
              title="Sinkronisasi status..."
            />
          )}
        </button>
      </TooltipTrigger>
      <TooltipContent side="bottom" className="text-xs font-mono">
        {tooltipText} (Klik untuk hubungkan ulang)
      </TooltipContent>
    </Tooltip>
  );
}
