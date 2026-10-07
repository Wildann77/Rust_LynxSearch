import * as React from 'react';
import {
  Folder,
  Plus,
  RefreshCw,
  Trash2,
  AlertTriangle,
  FolderPlus,
  CheckCircle2,
  Loader2,
  HardDrive,
} from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useUIStore } from '../../stores/uiStore';
import {
  useFoldersQuery,
  useRegisterFolderMutation,
  useDeleteFolderMutation,
} from '../../hooks/useFolderQueries';
import { useRebuildIndexMutation } from '../../hooks/useJobQuery';
import { queryKeys } from '../../hooks/queryKeys';
import { registerOrRescanFolder } from '../../api';
import { JobProgressIndicator } from '../jobs/JobProgressIndicator';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Skeleton } from '../ui/skeleton';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { formatRelativeTime } from '../../lib/date';
import { pickDirectory } from '../../lib/desktop-bridge';
import type { FolderResponse } from '../../types/folder';
import { cn } from '../../lib/utils';

export function FolderManagerModal() {
  const queryClient = useQueryClient();
  const folderModalOpen = useUIStore((state) => state.folderModalOpen);
  const setFolderModalOpen = useUIStore((state) => state.setFolderModalOpen);
  const setIndexingJobId = useUIStore((state) => state.setIndexingJobId);
  const addIndexingJobId = useUIStore((state) => state.addIndexingJobId);

  // Queries & Mutations
  const { data: folders = [], isLoading, isError, error, refetch } = useFoldersQuery();
  const registerMutation = useRegisterFolderMutation();
  const deleteMutation = useDeleteFolderMutation();
  const rebuildMutation = useRebuildIndexMutation();

  // Track folders currently in scanning/rescan state (optimistic set to guarantee minimum visual feedback)
  const [activeScanningFolderIds, setActiveScanningFolderIds] = React.useState<Set<string>>(new Set());
  const [isRescanningAll, setIsRescanningAll] = React.useState(false);

  // Dialog states for confirmations
  const [folderToDelete, setFolderToDelete] = React.useState<FolderResponse | null>(null);
  const [rebuildConfirmOpen, setRebuildConfirmOpen] = React.useState(false);

  // Handle Pick and Register Folder
  const handleAddFolder = async () => {
    try {
      const selectedPath = await pickDirectory();
      if (!selectedPath) return;

      const trimmed = selectedPath.trim();
      if (!trimmed) return;

      const res = await registerMutation.mutateAsync({ root_path: trimmed });
      setIndexingJobId(res.job_id);
      toast.success('Folder berhasil didaftarkan. Pemindaian dimulai.');
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Gagal menambahkan folder';
      toast.error(message);
    }
  };

  // Handle Re-scan individual folder with minimum visual feedback
  const handleRescanFolder = async (folder: FolderResponse) => {
    setActiveScanningFolderIds((prev) => new Set(prev).add(folder.id));
    // eslint-disable-next-line react-hooks/purity
    const startTime = performance.now();
    try {
      const res = await registerOrRescanFolder({
        folder_id: folder.id,
        root_path: folder.root_path,
      });
      setIndexingJobId(res.job_id);
      toast.success(`Pemindaian ulang folder dimulai: ${folder.root_path}`);
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Gagal memindai ulang folder';
      toast.error(message);
    } finally {
      const elapsed = performance.now() - startTime;
      const remaining = Math.max(0, 1500 - elapsed);
      setTimeout(() => {
        setActiveScanningFolderIds((prev) => {
          const next = new Set(prev);
          next.delete(folder.id);
          return next;
        });
        void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      }, remaining);
    }
  };

  // Handle Re-scan all registered folders concurrently
  const handleRescanAll = async () => {
    if (folders.length === 0 || isRescanningAll) return;
    setIsRescanningAll(true);
    const folderIds = folders.map((f) => f.id);
    setActiveScanningFolderIds(new Set(folderIds));
    const startTime = performance.now();

    try {
      const results = await Promise.allSettled(
        folders.map((f) =>
          registerOrRescanFolder({
            folder_id: f.id,
            root_path: f.root_path,
          }),
        ),
      );

      const successful = results.filter((r) => r.status === 'fulfilled');
      if (successful.length > 0) {
        toast.success(`Memulai pemindaian ulang ${successful.length} folder...`);
      }
      for (const res of results) {
        if (res.status === 'fulfilled') {
          addIndexingJobId(res.value.job_id);
        }
      }
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Gagal memindai ulang folder';
      toast.error(message);
    } finally {
      const elapsed = performance.now() - startTime;
      const remaining = Math.max(0, 1500 - elapsed);
      setTimeout(() => {
        setIsRescanningAll(false);
        setActiveScanningFolderIds(new Set());
        void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      }, remaining);
    }
  };

  // Handle Confirm Delete Folder
  const handleConfirmDelete = async () => {
    if (!folderToDelete) return;
    try {
      await deleteMutation.mutateAsync(folderToDelete.id);
      toast.success('Folder dan dokumen berhasil dihapus dari indeks.');
      setFolderToDelete(null);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Gagal menghapus folder';
      toast.error(message);
    }
  };

  // Handle Confirm Rebuild Index
  const handleConfirmRebuild = async () => {
    try {
      const res = await rebuildMutation.mutateAsync();
      setIndexingJobId(res.job_id);
      toast.success('Pembangunan ulang indeks Elasticsearch dimulai.');
      setRebuildConfirmOpen(false);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Gagal membangun ulang indeks';
      toast.error(message);
    }
  };

  const isAnyScanning = folders.some((f) => f.status === 'SCANNING') || activeScanningFolderIds.size > 0;

  return (
    <>
      {/* Primary Folder Manager Modal */}
      <Dialog open={folderModalOpen} onOpenChange={setFolderModalOpen}>
        <DialogContent className="sm:max-w-3xl max-h-[85vh] flex flex-col p-6 overflow-hidden">
          <DialogHeader className="shrink-0 pb-3 border-b border-border/80">
            <div className="flex items-center justify-between pr-8">
              <div className="flex items-center gap-2.5">
                <div className="p-2 rounded-lg bg-primary/10 border border-primary/20 text-primary">
                  <Folder className="h-5 w-5" />
                </div>
                <div>
                  <DialogTitle className="text-base font-semibold">Folder Manager</DialogTitle>
                  <DialogDescription className="text-xs text-muted-foreground mt-0.5">
                    Daftarkan direktori lokal untuk pengindeksan catatan Markdown dan source code.
                  </DialogDescription>
                </div>
              </div>

              <div className="flex items-center gap-2">
                {folders.length > 0 && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleRescanAll}
                    disabled={isRescanningAll || isAnyScanning}
                    className="gap-1.5 cursor-pointer text-xs"
                    aria-label="Pindai Semua Folder"
                  >
                    <RefreshCw
                      className={cn(
                        'h-3.5 w-3.5',
                        (isRescanningAll || isAnyScanning) && 'animate-spin text-amber-400',
                      )}
                    />
                    <span>Pindai Semua</span>
                  </Button>
                )}

                <Button
                  variant="default"
                  size="sm"
                  onClick={handleAddFolder}
                  disabled={registerMutation.isPending}
                  className="gap-1.5 cursor-pointer text-xs"
                >
                  {registerMutation.isPending ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Plus className="h-3.5 w-3.5" />
                  )}
                  <span>Tambah Folder</span>
                </Button>
              </div>
            </div>
          </DialogHeader>

          {/* Body Content / Table */}
          <div className="flex-1 min-h-0 overflow-y-auto py-3 space-y-3">
            {isLoading ? (
              <div className="space-y-2.5">
                <Skeleton className="h-14 w-full rounded-md" />
                <Skeleton className="h-14 w-full rounded-md" />
                <Skeleton className="h-14 w-full rounded-md" />
              </div>
            ) : isError ? (
              <div className="p-6 text-center border border-destructive/30 rounded-lg bg-destructive/10">
                <p className="text-xs text-destructive mb-3">
                  {error?.message || 'Gagal memuat daftar folder.'}
                </p>
                <Button variant="outline" size="sm" onClick={() => void refetch()} className="gap-1.5">
                  <RefreshCw className="h-3 w-3" />
                  <span>Coba Lagi</span>
                </Button>
              </div>
            ) : folders.length === 0 ? (
              <div className="py-12 flex flex-col items-center justify-center border border-dashed border-border rounded-xl bg-secondary/15 text-center px-4">
                <div className="h-12 w-12 rounded-full bg-secondary/60 flex items-center justify-center text-muted-foreground mb-3">
                  <FolderPlus className="h-6 w-6" />
                </div>
                <h4 className="text-sm font-semibold text-foreground mb-1">
                  Belum ada folder yang terdaftar
                </h4>
                <p className="text-xs text-muted-foreground max-w-sm mb-4 leading-relaxed">
                  Tambahkan folder pertama Anda untuk mulai mengindeks dokumen catatan dan source code lokal.
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleAddFolder}
                  disabled={registerMutation.isPending}
                  className="gap-1.5 cursor-pointer"
                >
                  <Plus className="h-3.5 w-3.5" />
                  <span>Pilih Folder Lokal</span>
                </Button>
              </div>
            ) : (
              <div className="border border-border rounded-lg overflow-hidden">
                <table className="w-full text-xs text-left border-collapse">
                  <thead>
                    <tr className="border-b border-border bg-secondary/40 text-muted-foreground font-mono text-[11px]">
                      <th className="py-2.5 px-3 font-medium">Folder Path</th>
                      <th className="py-2.5 px-3 font-medium">Scan Terakhir</th>
                      <th className="py-2.5 px-3 font-medium text-right">Dokumen</th>
                      <th className="py-2.5 px-3 font-medium text-center">Status</th>
                      <th className="py-2.5 px-3 font-medium text-right">Aksi</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/60">
                    {folders.map((folder) => {
                      const isScanning = folder.status === 'SCANNING' || activeScanningFolderIds.has(folder.id);
                      const isErrorStatus = folder.status === 'ERROR' && !activeScanningFolderIds.has(folder.id);

                      return (
                        <tr
                          key={folder.id}
                          className="hover:bg-secondary/20 transition-colors"
                        >
                          {/* Folder Path */}
                          <td className="py-2.5 px-3 max-w-[260px]">
                            <div className="flex items-center gap-2">
                              <HardDrive className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                              <span
                                className="font-mono text-xs text-foreground truncate select-text cursor-default"
                                title={folder.root_path}
                              >
                                {folder.root_path}
                              </span>
                            </div>
                          </td>

                          {/* Last Scan Relative Time */}
                          <td className="py-2.5 px-3 font-mono text-muted-foreground whitespace-nowrap">
                            {formatRelativeTime(folder.last_scanned_at)}
                          </td>

                          {/* Indexed Document Count */}
                          <td className="py-2.5 px-3 font-mono text-right text-foreground font-medium whitespace-nowrap">
                            {folder.document_count.toLocaleString()}
                          </td>

                          {/* Status Badge */}
                          <td className="py-2.5 px-3 text-center whitespace-nowrap">
                            {isScanning ? (
                              <Badge className="font-mono text-[10px] bg-amber-500/15 text-amber-300 border-amber-500/30 animate-pulse inline-flex items-center gap-1">
                                <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
                                <span>SCANNING</span>
                              </Badge>
                            ) : isErrorStatus ? (
                              <Badge variant="destructive" className="font-mono text-[10px]">
                                ERROR
                              </Badge>
                            ) : (
                              <Badge variant="secondary" className="font-mono text-[10px] text-muted-foreground inline-flex items-center gap-1">
                                <CheckCircle2 className="h-2.5 w-2.5 text-primary" />
                                <span>IDLE</span>
                              </Badge>
                            )}
                          </td>

                          {/* Action Buttons */}
                          <td className="py-2.5 px-3 text-right whitespace-nowrap">
                            <div className="flex items-center justify-end gap-1">
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    onClick={() => void handleRescanFolder(folder)}
                                    disabled={isScanning}
                                    className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
                                    aria-label={`Re-scan ${folder.root_path}`}
                                  >
                                    <RefreshCw
                                      className={cn(
                                        'h-3.5 w-3.5',
                                        isScanning && 'animate-spin text-amber-400',
                                      )}
                                    />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent side="bottom">Re-scan Folder</TooltipContent>
                              </Tooltip>

                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    onClick={() => setFolderToDelete(folder)}
                                    disabled={isScanning || deleteMutation.isPending}
                                    className="h-7 w-7 text-muted-foreground hover:text-destructive cursor-pointer"
                                    aria-label={`Hapus ${folder.root_path}`}
                                  >
                                    <Trash2 className="h-3.5 w-3.5" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent side="bottom">Hapus dari Indeks</TooltipContent>
                              </Tooltip>
                            </div>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            {/* Active Job Progress Indicator inside modal */}
            <JobProgressIndicator compact className="my-2.5" />

            {/* Danger Zone: Rebuild Index */}
            <div className="pt-2">
              <div className="flex items-center justify-between p-3.5 rounded-lg border border-destructive/20 bg-destructive/5">
                <div className="flex items-start gap-2.5">
                  <AlertTriangle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                  <div>
                    <h5 className="text-xs font-semibold text-foreground">
                      Bangun Ulang Indeks (Rebuild Index)
                    </h5>
                    <p className="text-[11px] text-muted-foreground leading-relaxed">
                      Hapus dan bangun kembali seluruh indeks Elasticsearch secara idempoten dari database dan file disk.
                    </p>
                  </div>
                </div>

                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setRebuildConfirmOpen(true)}
                  disabled={rebuildMutation.isPending}
                  className="border-destructive/40 text-destructive hover:bg-destructive/10 shrink-0 text-xs cursor-pointer ml-3"
                >
                  {rebuildMutation.isPending ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin mr-1.5" />
                  ) : null}
                  <span>Rebuild Index</span>
                </Button>
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Delete Folder Confirmation Dialog */}
      <Dialog open={Boolean(folderToDelete)} onOpenChange={(open) => !open && setFolderToDelete(null)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <div className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              <DialogTitle>Hapus Folder dari Indeks?</DialogTitle>
            </div>
            <DialogDescription className="text-xs leading-relaxed pt-2 text-muted-foreground">
              Folder <span className="font-mono text-foreground font-semibold break-all">{folderToDelete?.root_path}</span> beserta{' '}
              <span className="font-semibold text-foreground">{folderToDelete?.document_count}</span> dokumennya akan dihapus dari indeks pencarian.
              <br /><br />
              <span className="text-foreground font-medium">Catatan:</span> Berkas fisik pada disk komputer Anda tetap aman dan tidak akan disentuh.
            </DialogDescription>
          </DialogHeader>

          <DialogFooter className="gap-2 sm:gap-0 mt-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setFolderToDelete(null)}
              disabled={deleteMutation.isPending}
            >
              Batal
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleConfirmDelete}
              disabled={deleteMutation.isPending}
              className="gap-1.5"
            >
              {deleteMutation.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Trash2 className="h-3.5 w-3.5" />
              )}
              <span>Hapus Folder</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Strong Warning: Rebuild Index Confirmation Dialog */}
      <Dialog open={rebuildConfirmOpen} onOpenChange={setRebuildConfirmOpen}>
        <DialogContent className="sm:max-w-lg border-destructive/50">
          <DialogHeader>
            <div className="flex items-center gap-2.5 text-amber-500">
              <div className="p-2 rounded-md bg-amber-500/10 border border-amber-500/20">
                <AlertTriangle className="h-5 w-5" />
              </div>
              <DialogTitle className="text-base text-foreground">
                Peringatan Keras: Bangun Ulang Seluruh Indeks?
              </DialogTitle>
            </div>
            <DialogDescription asChild>
              <div className="text-xs leading-relaxed pt-3 text-muted-foreground">
                Tindakan ini akan membuat ulang seluruh indeks pencarian Elasticsearch dari awal.
                <br /><br />
                Selama proses berjalan di latar belakang:
                <ul className="list-disc list-inside mt-1.5 space-y-1 font-mono text-[11px] text-foreground/80">
                  <li>Seluruh dokumen akan dipindai ulang secara idempoten.</li>
                  <li>Pencarian tetap berjalan menggunakan alias indeks lama.</li>
                  <li>Setelah selesai, alias otomatis dialihkan ke indeks baru tanpa downtime.</li>
                </ul>
                <br />
                Apakah Anda yakin ingin memicu pembangunan ulang seluruh indeks sekarang?
              </div>
            </DialogDescription>
          </DialogHeader>

          <DialogFooter className="gap-2 sm:gap-0 mt-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setRebuildConfirmOpen(false)}
              disabled={rebuildMutation.isPending}
            >
              Batal
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleConfirmRebuild}
              disabled={rebuildMutation.isPending}
              className="gap-1.5 bg-destructive hover:bg-destructive/90"
            >
              {rebuildMutation.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <RefreshCw className="h-3.5 w-3.5" />
              )}
              <span>Ya, Bangun Ulang Seluruh Indeks</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
