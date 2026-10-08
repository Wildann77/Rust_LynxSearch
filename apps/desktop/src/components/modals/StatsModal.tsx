import * as React from 'react';
import {
  BarChart3,
  FileText,
  HardDrive,
  FolderTree,
  AlertCircle,
  RefreshCw,
  FolderPlus,
  Layers,
  Code2,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useStatsQuery } from '../../hooks/useStatsQuery';
import { formatBytes } from '../../lib/format';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Progress } from '../ui/progress';
import { Skeleton } from '../ui/skeleton';

export function StatsModal() {
  const statsModalOpen = useUIStore((state) => state.statsModalOpen);
  const setStatsModalOpen = useUIStore((state) => state.setStatsModalOpen);
  const setFolderModalOpen = useUIStore((state) => state.setFolderModalOpen);

  const { data: stats, isLoading, isError, error, refetch, isFetching } = useStatsQuery();

  // Convert types and languages records into sorted arrays for clear visual representation
  const typesRecord = stats?.types;
  const languagesRecord = stats?.languages;

  const sortedTypes = React.useMemo(() => {
    if (!typesRecord) return [];
    return Object.entries(typesRecord)
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count);
  }, [typesRecord]);

  const sortedLanguages = React.useMemo(() => {
    if (!languagesRecord) return [];
    return Object.entries(languagesRecord)
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count);
  }, [languagesRecord]);

  const totalDocs = stats?.total_documents ?? 0;
  const isEmpty = !isLoading && !isError && totalDocs === 0;

  return (
    <Dialog open={statsModalOpen} onOpenChange={setStatsModalOpen}>
      <DialogContent
        className="sm:max-w-xl max-h-[85vh] flex flex-col p-6 overflow-hidden gap-0"
        aria-describedby="stats-dialog-desc"
      >
        {/* Header */}
        <DialogHeader className="pb-4 border-b border-border shrink-0">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-md bg-primary/10 text-primary border border-primary/20 shrink-0">
              <BarChart3 className="h-5 w-5" />
            </div>
            <div>
              <DialogTitle className="text-base font-semibold">Index Statistics</DialogTitle>
              <DialogDescription id="stats-dialog-desc" className="text-xs text-muted-foreground mt-0.5">
                Ringkasan metrik volume dokumen lokal, penggunaan ukuran indeks, dan sebaran bahasa.
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        {/* Content Body */}
        <div className="flex-1 overflow-y-auto py-4 space-y-6 min-h-0 pr-1">
          {/* Loading State */}
          {isLoading && (
            <div className="space-y-4" data-testid="stats-loading">
              <div className="grid grid-cols-3 gap-3">
                <Skeleton className="h-20 rounded-lg" />
                <Skeleton className="h-20 rounded-lg" />
                <Skeleton className="h-20 rounded-lg" />
              </div>
              <Skeleton className="h-32 w-full rounded-lg" />
              <Skeleton className="h-32 w-full rounded-lg" />
            </div>
          )}

          {/* Error State */}
          {isError && (
            <div
              className="py-8 flex flex-col items-center justify-center text-center space-y-3"
              data-testid="stats-error"
            >
              <div className="p-3 rounded-full bg-destructive/10 text-destructive border border-destructive/20">
                <AlertCircle className="h-6 w-6" />
              </div>
              <div className="max-w-sm">
                <p className="text-sm font-semibold text-destructive">Gagal Mengambil Statistik</p>
                <p className="text-xs text-muted-foreground mt-1">
                  {error instanceof Error ? error.message : 'Backend service tidak merespons.'}
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void refetch()}
                className="text-xs cursor-pointer mt-2"
              >
                <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                Coba Lagi
              </Button>
            </div>
          )}

          {/* Empty State */}
          {isEmpty && (
            <div
              className="py-10 flex flex-col items-center justify-center text-center space-y-3"
              data-testid="stats-empty"
            >
              <div className="p-3.5 rounded-2xl bg-secondary/70 border border-border text-muted-foreground">
                <FolderPlus className="h-7 w-7 text-primary" />
              </div>
              <div className="max-w-sm space-y-1">
                <h4 className="text-sm font-semibold text-foreground">Belum Ada Dokumen Terindeks</h4>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Daftarkan folder direktori proyek atau catatan markdown di Folder Manager untuk memulai pengindeksan.
                </p>
              </div>
              <Button
                size="sm"
                onClick={() => {
                  setStatsModalOpen(false);
                  setFolderModalOpen(true);
                }}
                className="text-xs cursor-pointer mt-2"
              >
                Buka Folder Manager
              </Button>
            </div>
          )}

          {/* Success State */}
          {!isLoading && !isError && !isEmpty && stats && (
            <div className="space-y-6" data-testid="stats-content">
              {/* Top Key Metrics Cards */}
              <div className="grid grid-cols-3 gap-3">
                <div className="rounded-lg border border-border bg-card/60 p-3.5 flex flex-col justify-between">
                  <div className="flex items-center justify-between text-muted-foreground">
                    <span className="text-[11px] font-medium uppercase tracking-wider">Dokumen</span>
                    <FileText className="h-4 w-4 text-primary" />
                  </div>
                  <div className="mt-2">
                    <div className="text-xl font-bold font-mono text-foreground tracking-tight">
                      {stats.total_documents.toLocaleString()}
                    </div>
                    <span className="text-[10px] text-muted-foreground">Total item aktif</span>
                  </div>
                </div>

                <div className="rounded-lg border border-border bg-card/60 p-3.5 flex flex-col justify-between">
                  <div className="flex items-center justify-between text-muted-foreground">
                    <span className="text-[11px] font-medium uppercase tracking-wider">Ukuran Indeks</span>
                    <HardDrive className="h-4 w-4 text-primary" />
                  </div>
                  <div className="mt-2">
                    <div className="text-xl font-bold font-mono text-foreground tracking-tight">
                      {formatBytes(stats.total_size_bytes)}
                    </div>
                    <span className="text-[10px] text-muted-foreground">Konten berkas</span>
                  </div>
                </div>

                <div className="rounded-lg border border-border bg-card/60 p-3.5 flex flex-col justify-between">
                  <div className="flex items-center justify-between text-muted-foreground">
                    <span className="text-[11px] font-medium uppercase tracking-wider">Folder</span>
                    <FolderTree className="h-4 w-4 text-primary" />
                  </div>
                  <div className="mt-2">
                    <div className="text-xl font-bold font-mono text-foreground tracking-tight">
                      {stats.indexed_folders}
                    </div>
                    <span className="text-[10px] text-muted-foreground">Sumber lokal terdaftar</span>
                  </div>
                </div>
              </div>

              {/* Type Distribution */}
              <div className="space-y-3 rounded-lg border border-border bg-card/40 p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Layers className="h-4 w-4 text-primary" />
                    <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                      Distribusi Tipe Dokumen
                    </h4>
                  </div>
                  <span className="text-[11px] font-mono text-muted-foreground">
                    {sortedTypes.length} tipe
                  </span>
                </div>

                {sortedTypes.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">Tidak ada rincian tipe.</p>
                ) : (
                  <div className="space-y-2.5 pt-1">
                    {sortedTypes.map(({ name, count }) => {
                      const pct = totalDocs > 0 ? Math.round((count / totalDocs) * 100) : 0;
                      return (
                        <div key={name} className="space-y-1">
                          <div className="flex items-center justify-between text-xs">
                            <div className="flex items-center gap-1.5">
                              <Badge variant="secondary" className="text-[10px] uppercase font-mono px-1.5 py-0">
                                {name}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-2 font-mono text-[11px] text-muted-foreground">
                              <span>{count.toLocaleString()} file</span>
                              <span className="text-border">|</span>
                              <span className="w-9 text-right text-foreground font-semibold">{pct}%</span>
                            </div>
                          </div>
                          <Progress value={count} max={totalDocs} className="h-1.5" />
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Language Distribution */}
              <div className="space-y-3 rounded-lg border border-border bg-card/40 p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Code2 className="h-4 w-4 text-primary" />
                    <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                      Distribusi Bahasa Pemrograman
                    </h4>
                  </div>
                  <span className="text-[11px] font-mono text-muted-foreground">
                    {sortedLanguages.length} bahasa
                  </span>
                </div>

                {sortedLanguages.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">Tidak ada rincian bahasa.</p>
                ) : (
                  <div className="space-y-2.5 pt-1">
                    {sortedLanguages.map(({ name, count }) => {
                      const pct = totalDocs > 0 ? Math.round((count / totalDocs) * 100) : 0;
                      return (
                        <div key={name} className="space-y-1">
                          <div className="flex items-center justify-between text-xs">
                            <div className="flex items-center gap-1.5">
                              <span className="font-mono text-xs font-medium text-foreground">
                                {name}
                              </span>
                            </div>
                            <div className="flex items-center gap-2 font-mono text-[11px] text-muted-foreground">
                              <span>{count.toLocaleString()} file</span>
                              <span className="text-border">|</span>
                              <span className="w-9 text-right text-foreground font-semibold">{pct}%</span>
                            </div>
                          </div>
                          <Progress
                            value={count}
                            max={totalDocs}
                            className="h-1.5"
                            indicatorClassName="bg-emerald-500"
                          />
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="pt-3 border-t border-border flex items-center justify-between shrink-0">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void refetch()}
            disabled={isLoading || isFetching}
            className="text-xs text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${isFetching ? 'animate-spin' : ''}`} />
            Perbarui Data
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setStatsModalOpen(false)}
            className="text-xs cursor-pointer"
          >
            Tutup
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
