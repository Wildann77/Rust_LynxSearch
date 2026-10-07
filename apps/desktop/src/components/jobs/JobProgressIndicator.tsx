import * as React from 'react';
import {
  CheckCircle2,
  AlertTriangle,
  Ban,
  ChevronUp,
  ChevronDown,
  X,
  Clock,
  RotateCw,
} from 'lucide-react';
import { toast } from 'sonner';
import { Progress } from '../ui/progress';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/tooltip';
import { useUIStore } from '../../stores/uiStore';
import { useJobQuery, useCancelJobMutation } from '../../hooks/useJobQuery';
import { useFoldersQuery } from '../../hooks/useFolderQueries';
import { formatDateTime, formatDuration } from '../../lib/date';
import { cn } from '../../lib/utils';

export interface JobProgressIndicatorProps {
  className?: string;
  autoDismissMs?: number;
  compact?: boolean;
}

export function JobProgressIndicator({
  className,
  autoDismissMs = 6000,
  compact = false,
}: JobProgressIndicatorProps) {
  const indexingJobId = useUIStore((state) => state.indexingJobId);
  const indexingJobIds = useUIStore((state) => state.indexingJobIds);
  const setIndexingJobId = useUIStore((state) => state.setIndexingJobId);
  const removeIndexingJobId = useUIStore((state) => state.removeIndexingJobId);
  const jobDrawerExpanded = useUIStore((state) => state.jobDrawerExpanded);
  const toggleJobDrawer = useUIStore((state) => state.toggleJobDrawer);
  const setJobDrawerExpanded = useUIStore((state) => state.setJobDrawerExpanded);

  const { data: job, isLoading } = useJobQuery(indexingJobId);
  const { data: folders } = useFoldersQuery();
  const cancelMutation = useCancelJobMutation();

  const [isHovered, setIsHovered] = React.useState(false);
  const notifiedJobIdRef = React.useRef<string | null>(null);

  const isTerminal =
    job?.status === 'COMPLETED' ||
    job?.status === 'FAILED' ||
    job?.status === 'CANCELLED';

  const isRunning = job?.status === 'RUNNING' || job?.status === 'PENDING';

  // Handle toast notification on terminal state transitions
  React.useEffect(() => {
    if (!job || !isTerminal) return;
    if (notifiedJobIdRef.current === job.job_id) return;

    notifiedJobIdRef.current = job.job_id;

    if (job.status === 'COMPLETED') {
      toast.success(
        `Indeks Selesai: ${job.indexed_files} terindeks, ${job.skipped_files} dilewati, ${job.failed_files} gagal.`,
      );
    } else if (job.status === 'FAILED') {
      toast.error(`Pekerjaan indeks gagal: ${job.error || 'Terjadi kesalahan sistem'}`);
    } else if (job.status === 'CANCELLED') {
      toast.warning('Pekerjaan indeks telah dibatalkan.');
    }
  }, [job, isTerminal]);

  // Handle auto-dismiss timer on terminal state
  React.useEffect(() => {
    if (!indexingJobId || !isTerminal || isHovered || jobDrawerExpanded) {
      return;
    }

    const timer = setTimeout(() => {
      if (job?.job_id) {
        removeIndexingJobId(job.job_id);
      } else {
        setIndexingJobId(null);
      }
      setJobDrawerExpanded(false);
    }, autoDismissMs);

    return () => clearTimeout(timer);
  }, [
    indexingJobId,
    job?.job_id,
    isTerminal,
    isHovered,
    jobDrawerExpanded,
    autoDismissMs,
    removeIndexingJobId,
    setIndexingJobId,
    setJobDrawerExpanded,
  ]);

  // Do not render if there's no active job id tracked
  if (!indexingJobId) {
    return null;
  }

  // Handle cancel request
  const handleCancel = () => {
    if (!job) return;
    cancelMutation.mutate(job.job_id, {
      onSuccess: () => {
        toast.info('Permintaan pembatalan indeks dikirim.');
      },
      onError: (err) => {
        toast.error(`Gagal membatalkan job: ${err.message}`);
      },
    });
  };

  // Handle manual dismiss
  const handleDismiss = () => {
    if (job?.job_id) {
      removeIndexingJobId(job.job_id);
    } else if (indexingJobId) {
      removeIndexingJobId(indexingJobId);
    }
    setJobDrawerExpanded(false);
  };

  const processed = job?.processed_files ?? 0;
  const total = job?.total_files ?? 0;
  const indexed = job?.indexed_files ?? 0;
  const skipped = job?.skipped_files ?? 0;
  const failed = job?.failed_files ?? 0;

  const percentage =
    total > 0 ? Math.min(100, Math.max(0, Math.round((processed / total) * 100))) : isTerminal ? 100 : 0;

  const currentFolder = folders?.find((f) => f.id === job?.folder_id);
  const currentFolderName = currentFolder
    ? currentFolder.root_path.split('/').filter(Boolean).pop()
    : null;

  return (
    <div
      role="region"
      aria-label="Status Progres Pemindaian Indeks"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={cn(
        'w-full border-t border-border bg-card/95 backdrop-blur text-card-foreground shadow-lg transition-all duration-200 select-none z-30 shrink-0',
        compact && 'rounded-lg border shadow-sm my-2 overflow-hidden',
        className,
      )}
    >
      {/* Multi-Job Tab Switcher */}
      {indexingJobIds.length > 1 ? (
        <div className="flex items-center gap-1.5 px-3 py-1.5 border-b border-border/50 bg-secondary/20 overflow-x-auto text-xs">
          <span className="text-[10px] text-muted-foreground uppercase font-mono mr-1 shrink-0">
            Pekerjaan ({indexingJobIds.length}):
          </span>
          {indexingJobIds.map((id, idx) => {
            const isCurrent = id === indexingJobId;
            return (
              <button
                key={id}
                type="button"
                onClick={() => setIndexingJobId(id)}
                className={cn(
                  'px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer flex items-center gap-1 shrink-0',
                  isCurrent
                    ? 'bg-primary text-black font-semibold shadow-xs'
                    : 'bg-secondary/70 text-muted-foreground hover:text-foreground',
                )}
                aria-label={`Pilih pekerjaan ${id}`}
              >
                <span>Job #{idx + 1}</span>
                <span className="text-[9px] opacity-75">({id.slice(0, 4)})</span>
              </button>
            );
          })}
        </div>
      ) : null}

      {/* Expanded Details Drawer */}
      {jobDrawerExpanded && job ? (
        <div className="border-b border-border/80 bg-secondary/30 px-4 py-3 text-xs animate-in slide-in-from-bottom-2 duration-200">
          <div className="flex items-center justify-between pb-2 mb-2 border-b border-border/40">
            <span className="font-semibold text-foreground text-xs uppercase tracking-wider">
              Rincian Pekerjaan Indeks {currentFolderName ? `(${currentFolderName})` : ''}
            </span>
            <div className="flex items-center gap-2 text-muted-foreground font-mono text-[11px]">
              <span className="truncate max-w-[200px]" title={job.job_id}>
                ID: {job.job_id.slice(0, 8)}...
              </span>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 py-1">
            <div className="p-2 rounded-md bg-card/80 border border-border/60">
              <span className="text-muted-foreground text-[10px] block uppercase font-mono">
                Total Berkas
              </span>
              <span className="text-sm font-semibold font-mono text-foreground">
                {total.toLocaleString()}
              </span>
            </div>

            <div className="p-2 rounded-md bg-card/80 border border-border/60">
              <span className="text-muted-foreground text-[10px] block uppercase font-mono">
                Berhasil Diindeks
              </span>
              <span className="text-sm font-semibold font-mono text-[#10a37f]">
                {indexed.toLocaleString()}
              </span>
            </div>

            <div className="p-2 rounded-md bg-card/80 border border-border/60">
              <span className="text-muted-foreground text-[10px] block uppercase font-mono">
                Dilewati (Skip)
              </span>
              <span className="text-sm font-semibold font-mono text-muted-foreground">
                {skipped.toLocaleString()}
              </span>
            </div>

            <div className="p-2 rounded-md bg-card/80 border border-border/60">
              <span className="text-muted-foreground text-[10px] block uppercase font-mono">
                Gagal
              </span>
              <span
                className={cn(
                  'text-sm font-semibold font-mono',
                  failed > 0 ? 'text-destructive font-bold' : 'text-muted-foreground',
                )}
              >
                {failed.toLocaleString()}
              </span>
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-2 pt-2 mt-1 text-[11px] font-mono text-muted-foreground">
            <div className="flex items-center gap-4">
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3 text-muted-foreground" />
                <span>Mulai: {formatDateTime(job.started_at)}</span>
              </span>
              {job.finished_at ? (
                <span>Selesai: {formatDateTime(job.finished_at)}</span>
              ) : null}
              <span>Durasi: {formatDuration(job.started_at, job.finished_at)}</span>
            </div>

            {isTerminal ? (
              <span className="text-[10px] text-muted-foreground/75 italic">
                Auto-tutup dalam {Math.round(autoDismissMs / 1000)} detik saat tidak aktif
              </span>
            ) : null}
          </div>

          {job.error ? (
            <div className="mt-2.5 p-2 rounded-md bg-destructive/15 border border-destructive/30 text-destructive text-xs font-mono flex items-start gap-2">
              <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
              <div className="flex-1 overflow-hidden">
                <span className="font-semibold block">Kesalahan Indeks:</span>
                <span className="break-all">{job.error}</span>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}

      {/* Main Banner Bar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2 px-3 py-2">
        {/* Left region: Status badge + info */}
        <div className="flex items-center gap-2.5 min-w-0">
          {isLoading && !job ? (
            <RotateCw className="h-4 w-4 animate-spin text-muted-foreground shrink-0" />
          ) : isRunning ? (
            <RotateCw className="h-4 w-4 animate-spin text-amber-400 shrink-0" />
          ) : job?.status === 'COMPLETED' ? (
            <CheckCircle2 className="h-4 w-4 text-[#10a37f] shrink-0" />
          ) : job?.status === 'FAILED' ? (
            <AlertTriangle className="h-4 w-4 text-destructive shrink-0" />
          ) : (
            <Ban className="h-4 w-4 text-amber-500 shrink-0" />
          )}

          <div className="flex items-center gap-2 min-w-0 truncate">
            <span className="text-xs font-medium text-foreground truncate">
              {isRunning
                ? currentFolderName
                  ? `Memindai ${currentFolderName}...`
                  : 'Memproses Indeks Dokumen...'
                : job?.status === 'COMPLETED'
                  ? 'Pemindaian Indeks Selesai'
                  : job?.status === 'FAILED'
                    ? 'Pemindaian Indeks Gagal'
                    : 'Pekerjaan Dibatalkan'}
            </span>

            {job?.status ? (
              <Badge
                variant={
                  isRunning
                    ? 'secondary'
                    : job.status === 'COMPLETED'
                      ? 'default'
                      : job.status === 'FAILED'
                        ? 'destructive'
                        : 'outline'
                }
                className={cn(
                  'text-[10px] font-mono px-1.5 py-0 uppercase',
                  isRunning && 'bg-amber-500/15 text-amber-300 border-amber-500/30 animate-pulse',
                  job.status === 'COMPLETED' && 'bg-[#10a37f]/20 text-[#10a37f] border-[#10a37f]/40',
                )}
              >
                {job.status}
              </Badge>
            ) : null}
          </div>
        </div>

        {/* Center region: Dynamic Progress Bar & Metrics */}
        <div className="flex flex-1 items-center gap-3 max-w-md mx-2">
          <div className="flex-1 min-w-[120px]">
            <Progress
              value={percentage}
              max={100}
              className="h-2 bg-secondary"
              indicatorClassName={cn(
                isRunning && 'bg-amber-400 animate-pulse',
                job?.status === 'COMPLETED' && 'bg-[#10a37f]',
                job?.status === 'FAILED' && 'bg-destructive',
                job?.status === 'CANCELLED' && 'bg-muted-foreground',
              )}
            />
          </div>

          <div className="flex items-center gap-2 text-[11px] font-mono shrink-0 text-muted-foreground">
            <span className="font-semibold text-foreground" title="Processed / Total">
              {processed}/{total}
            </span>
            <span className="text-border">•</span>
            <span title="Skipped">Skip: {skipped}</span>
            <span className="text-border">•</span>
            <span
              className={cn(failed > 0 && 'text-destructive font-semibold')}
              title="Failed"
            >
              Fail: {failed}
            </span>
          </div>
        </div>

        {/* Right region: Actions (Cancel, Toggle Drawer, Dismiss) */}
        <div className="flex items-center gap-1.5 shrink-0 justify-end">
          {isRunning ? (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleCancel}
              disabled={cancelMutation.isPending}
              className="h-7 px-2.5 text-xs gap-1 cursor-pointer"
              aria-label="Batalkan pekerjaan indeks"
            >
              <Ban className="h-3 w-3" />
              <span>{cancelMutation.isPending ? 'Membatalkan...' : 'Batalkan'}</span>
            </Button>
          ) : null}

          {/* Toggle Details Drawer */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={toggleJobDrawer}
                className={cn(
                  'h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer',
                  jobDrawerExpanded && 'bg-secondary text-foreground',
                )}
                aria-label={jobDrawerExpanded ? 'Tutup rincian pekerjaan' : 'Buka rincian pekerjaan'}
              >
                {jobDrawerExpanded ? (
                  <ChevronDown className="h-3.5 w-3.5" />
                ) : (
                  <ChevronUp className="h-3.5 w-3.5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              {jobDrawerExpanded ? 'Tutup Rincian' : 'Rincian Lengkap'}
            </TooltipContent>
          </Tooltip>

          {/* Dismiss Summary Button */}
          {isTerminal ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleDismiss}
                  className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
                  aria-label="Tutup ringkasan pekerjaan"
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">Tutup</TooltipContent>
            </Tooltip>
          ) : null}
        </div>
      </div>
    </div>
  );
}
