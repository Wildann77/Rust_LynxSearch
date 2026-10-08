import * as React from 'react';
import { WifiOff, RefreshCw } from 'lucide-react';
import { toast } from 'sonner';
import { useHealthQuery } from '../../hooks/useHealthQuery';
import { BACKEND_URL } from '../../api/config';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface ConnectionBannerProps {
  className?: string;
  onRetry?: () => void;
  retry?: number | boolean;
}

export function ConnectionBanner({ className, onRetry, retry }: ConnectionBannerProps) {
  const { isError, isLoading, isSuccess, isRefetching, refetch } = useHealthQuery({ retry });

  const isOffline = isError || (!isLoading && !isSuccess);
  const prevOfflineRef = React.useRef(false);

  React.useEffect(() => {
    if (isOffline && !prevOfflineRef.current) {
      toast.error('Gagal terhubung ke service backend', {
        id: 'backend-connection-error',
      });
    }
    prevOfflineRef.current = isOffline;
  }, [isOffline]);

  if (!isOffline) {
    return null;
  }

  const handleRetry = () => {
    void refetch().then((res) => {
      if (res.isError) {
        toast.error('Gagal terhubung ke service backend', {
          id: 'backend-connection-error',
        });
      }
    });
    if (onRetry) {
      onRetry();
    }
  };

  return (
    <div
      role="alert"
      data-testid="connection-banner"
      className={cn(
        'w-full bg-destructive/15 border-b border-destructive/30 px-4 py-2 flex items-center justify-between text-xs text-foreground select-none transition-all duration-200',
        className,
      )}
    >
      <div className="flex items-center gap-2.5 min-w-0">
        <div className="p-1 rounded-full bg-destructive/20 text-destructive shrink-0">
          <WifiOff className="h-3.5 w-3.5" />
        </div>
        <div className="truncate">
          <span className="font-semibold text-destructive mr-1.5 font-mono text-[11px]">
            [OFFLINE]
          </span>
          <span className="text-muted-foreground text-xs">
            Koneksi ke backend LynxSearch ({BACKEND_URL.replace(/^https?:\/\//, '')}) terputus.
            Pencarian dan pengindeksan dijeda sementara.
          </span>
        </div>
      </div>

      <Button
        variant="outline"
        size="sm"
        onClick={handleRetry}
        disabled={isRefetching}
        className="h-6 px-2.5 text-[11px] font-mono border-destructive/40 hover:bg-destructive/20 text-foreground cursor-pointer shrink-0 ml-3"
      >
        <RefreshCw className={`h-3 w-3 mr-1.5 ${isRefetching ? 'animate-spin' : ''}`} />
        Hubungkan Ulang
      </Button>
    </div>
  );
}
