import { WifiOff, RefreshCw, Terminal, CheckCircle2 } from 'lucide-react';
import { Button } from '../ui/button';
import { BACKEND_URL } from '../../api/config';
import { cn } from '../../lib/utils';

export interface OfflineFallbackProps {
  className?: string;
  onRetry?: () => void;
  isRetrying?: boolean;
}

export function OfflineFallback({ className, onRetry, isRetrying = false }: OfflineFallbackProps) {
  return (
    <div
      data-testid="offline-fallback"
      className={cn(
        'flex flex-1 flex-col items-center justify-center p-6 text-center select-none',
        className,
      )}
    >
      <div className="max-w-md w-full flex flex-col items-center gap-4">
        {/* Offline Icon */}
        <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-destructive/10 border border-destructive/30 text-destructive shadow-sm">
          <WifiOff className="h-7 w-7" />
        </div>

        {/* Heading & Explanation */}
        <div className="space-y-1.5">
          <h3 className="text-base font-semibold text-foreground tracking-tight">
            Backend Service Tidak Tersedia
          </h3>
          <p className="text-xs text-muted-foreground leading-relaxed">
            Tidak dapat berkomunikasi dengan backend lokal di{' '}
            <code className="text-foreground bg-secondary px-1.5 py-0.5 rounded text-[11px] font-mono">
              {BACKEND_URL}
            </code>
            . Seluruh fungsi pencarian, navigasi berkas, dan indexing dijeda.
          </p>
        </div>

        {/* Troubleshooting Instructions Card */}
        <div className="w-full text-left rounded-lg border border-border bg-card/60 p-3.5 space-y-2.5 font-mono text-xs">
          <div className="flex items-center gap-1.5 text-muted-foreground font-sans font-medium text-[11px] uppercase tracking-wider">
            <Terminal className="h-3.5 w-3.5 text-primary" />
            <span>Langkah Pemulihan Cepat:</span>
          </div>

          <div className="space-y-2 text-[11px] text-muted-foreground">
            <div className="flex items-start gap-2">
              <span className="text-primary font-bold">1.</span>
              <div>
                <span>Pastikan Docker berjalan:</span>
                <div className="mt-1 rounded bg-secondary/80 px-2 py-1 text-foreground select-all">
                  docker compose -f docker/docker-compose.yml up -d
                </div>
              </div>
            </div>

            <div className="flex items-start gap-2">
              <span className="text-primary font-bold">2.</span>
              <div>
                <span>Jalankan backend LynxSearch:</span>
                <div className="mt-1 rounded bg-secondary/80 px-2 py-1 text-foreground select-all">
                  cargo run -p backend
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Retry Button */}
        {onRetry ? (
          <Button
            onClick={onRetry}
            disabled={isRetrying}
            className="cursor-pointer text-xs mt-1"
          >
            <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${isRetrying ? 'animate-spin' : ''}`} />
            Hubungkan Ulang Sekarang
          </Button>
        ) : null}

        <div className="flex items-center gap-1 text-[11px] text-muted-foreground/70">
          <CheckCircle2 className="h-3 w-3 text-muted-foreground/50" />
          <span>Aplikasi akan otomatis terhubung kembali saat backend aktif</span>
        </div>
      </div>
    </div>
  );
}
