import * as React from 'react';
import {
  Settings,
  Sliders,
  RotateCcw,
  Save,
  Plus,
  X,
  Loader2,
  AlertCircle,
  RefreshCw,
} from 'lucide-react';
import { toast } from 'sonner';
import { useUIStore } from '../../stores/uiStore';
import {
  useSettingsQuery,
  useUpdateSettingsMutation,
} from '../../hooks/useSettingsQuery';
import { DEFAULT_SETTINGS, type AppSettings } from '../../types/settings';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/badge';
import { Slider } from '../ui/slider';
import { Skeleton } from '../ui/skeleton';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select';

interface SettingsFormProps {
  settings: AppSettings;
  onClose: () => void;
}

function SettingsForm({ settings, onClose }: SettingsFormProps) {
  const updateMutation = useUpdateSettingsMutation();

  const [maxFileSizeMb, setMaxFileSizeMb] = React.useState<number>(() => {
    const mb = Math.round((settings.max_file_size_bytes / (1024 * 1024)) * 10) / 10;
    return mb >= 1 ? mb : 1;
  });
  const [titleWeight, setTitleWeight] = React.useState<number>(settings.weights.title);
  const [tagWeight, setTagWeight] = React.useState<number>(settings.weights.tags);
  const [contentWeight, setContentWeight] = React.useState<number>(settings.weights.content);
  const [ignorePatterns, setIgnorePatterns] = React.useState<string[]>(settings.ignore_patterns);
  const [patternInput, setPatternInput] = React.useState<string>('');
  const [patternError, setPatternError] = React.useState<string | null>(null);

  const preferredEditor = useUIStore((state) => state.preferredEditor);
  const setPreferredEditor = useUIStore((state) => state.setPreferredEditor);

  const handleAddPattern = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = patternInput.trim();
    if (!trimmed) return;

    if (ignorePatterns.includes(trimmed)) {
      setPatternError(`Pola "${trimmed}" sudah terdaftar.`);
      return;
    }

    setIgnorePatterns((prev) => [...prev, trimmed]);
    setPatternInput('');
    setPatternError(null);
  };

  const handleRemovePattern = (patternToRemove: string) => {
    setIgnorePatterns((prev) => prev.filter((p) => p !== patternToRemove));
  };

  const handleResetToDefault = () => {
    setMaxFileSizeMb(Math.round(DEFAULT_SETTINGS.max_file_size_bytes / (1024 * 1024)));
    setTitleWeight(DEFAULT_SETTINGS.weights.title);
    setTagWeight(DEFAULT_SETTINGS.weights.tags);
    setContentWeight(DEFAULT_SETTINGS.weights.content);
    setIgnorePatterns([...DEFAULT_SETTINGS.ignore_patterns]);
    setPatternInput('');
    setPatternError(null);
    setPreferredEditor(null);
    toast.info('Nilai dikembalikan ke default. Klik Simpan untuk menerapkan.');
  };

  const handleSave = async () => {
    if (maxFileSizeMb < 1 || maxFileSizeMb > 100) {
      toast.error('Batas ukuran file harus antara 1 MB dan 100 MB');
      return;
    }

    const max_file_size_bytes = Math.round(maxFileSizeMb * 1024 * 1024);

    try {
      await updateMutation.mutateAsync({
        max_file_size_bytes,
        weights: {
          title: titleWeight,
          tags: tagWeight,
          content: contentWeight,
        },
        ignore_patterns: ignorePatterns,
      });

      toast.success('Pengaturan berhasil disimpan');
      onClose();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Terjadi kesalahan sistem';
      toast.error(`Gagal menyimpan pengaturan: ${msg}`);
    }
  };

  return (
    <>
      <div className="space-y-5 py-2">
        {/* Preferred Editor Selection */}
        <div className="space-y-1.5 p-3 rounded-lg border border-border/60 bg-secondary/20">
          <label htmlFor="settings-preferred-editor" className="text-xs font-medium text-foreground block">
            Editor Berkas Bawaan (Open in Editor)
          </label>
          <div className="flex items-center gap-2">
            <Select
              value={preferredEditor ?? 'ask'}
              onValueChange={(val) => {
                setPreferredEditor(val === 'ask' ? null : val);
                toast.success(
                  val === 'ask'
                    ? 'Editor diatur ke: Selalu Tanya'
                    : `Editor bawaan diatur ke: ${val}`,
                );
              }}
            >
              <SelectTrigger
                id="settings-preferred-editor"
                className="w-full h-9 bg-card border-border/80 px-3 py-2 text-xs font-medium text-foreground hover:bg-secondary/40 focus:ring-1 focus:ring-ring transition-colors cursor-pointer"
                aria-label="Editor Berkas Bawaan"
              >
                <SelectValue placeholder="Pilih editor berkas bawaan" />
              </SelectTrigger>
              <SelectContent className="bg-popover/95 backdrop-blur-sm border-border text-popover-foreground shadow-xl">
                <SelectItem
                  value="ask"
                  className="text-xs font-medium py-2 cursor-pointer focus:bg-secondary focus:text-foreground"
                >
                  Selalu Tanya (Munculkan Pilihan)
                </SelectItem>
                <SelectItem
                  value="antigravity-ide"
                  className="text-xs font-medium py-2 cursor-pointer focus:bg-secondary focus:text-foreground"
                >
                  Antigravity IDE (Rekomendasi)
                </SelectItem>
                <SelectItem
                  value="code"
                  className="text-xs font-medium py-2 cursor-pointer focus:bg-secondary focus:text-foreground"
                >
                  Visual Studio Code (code)
                </SelectItem>
                <SelectItem
                  value="gnome-text-editor"
                  className="text-xs font-medium py-2 cursor-pointer focus:bg-secondary focus:text-foreground"
                >
                  GNOME Text Editor
                </SelectItem>
                <SelectItem
                  value="default"
                  className="text-xs font-medium py-2 cursor-pointer focus:bg-secondary focus:text-foreground"
                >
                  Aplikasi Bawaan OS (xdg-open)
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <p className="text-[11px] text-muted-foreground">
            Aplikasi yang dipanggil saat menekan tombol "Open in Editor" atau pintasan <kbd className="font-mono">⌘O</kbd>.
          </p>
        </div>

        {/* Numeric Max File Size */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <label htmlFor="settings-max-file-size" className="text-xs font-medium text-foreground">
              Batas Ukuran Berkas Maksimum (MB)
            </label>
            <span className="text-xs font-mono text-muted-foreground">
              ~{(maxFileSizeMb * 1024 * 1024).toLocaleString()} bytes
            </span>
          </div>
          <Input
            id="settings-max-file-size"
            type="number"
            min={1}
            max={100}
            step={1}
            value={maxFileSizeMb}
            onChange={(e) => setMaxFileSizeMb(Math.max(1, Math.min(100, Number(e.target.value) || 1)))}
            className="font-mono text-sm"
            aria-label="Batas Ukuran Berkas Maksimum dalam Megabyte"
          />
          <p className="text-[11px] text-muted-foreground">
            Berkas lebih besar dari batas ini akan dilewati selama indexing. Rentang valid: 1 MB – 100 MB.
          </p>
        </div>

        {/* Ignore Patterns Tag Input */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-foreground">
            Pola yang Diabaikan (Ignore Patterns)
          </label>
          <div className="flex flex-wrap gap-1.5 p-2 rounded-md border border-border bg-secondary/30 min-h-12 items-center">
            {ignorePatterns.length === 0 ? (
              <span className="text-xs text-muted-foreground/60 italic">
                Tidak ada pola ignore khusus.
              </span>
            ) : (
              ignorePatterns.map((pattern) => (
                <Badge
                  key={pattern}
                  variant="secondary"
                  className="gap-1.5 py-1 px-2.5 font-mono text-xs border border-border/60 bg-secondary/80"
                >
                  <span>{pattern}</span>
                  <button
                    type="button"
                    onClick={() => handleRemovePattern(pattern)}
                    className="rounded-full hover:bg-muted p-0.5 text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                    aria-label={`Hapus pola ${pattern}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))
            )}
          </div>
          <div className="flex gap-2">
            <Input
              value={patternInput}
              onChange={(e) => {
                setPatternInput(e.target.value);
                if (patternError) setPatternError(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  handleAddPattern();
                }
              }}
              placeholder="Tambah pola baru (misal: *.log, dist, temp)..."
              className="text-xs font-mono"
              aria-label="Input pola baru"
            />
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => handleAddPattern()}
              className="shrink-0 cursor-pointer text-xs"
            >
              <Plus className="h-3.5 w-3.5 mr-1" />
              Tambah
            </Button>
          </div>
          {patternError && (
            <p className="text-[11px] text-destructive">{patternError}</p>
          )}
        </div>

        {/* BM25 Field Weight Sliders */}
        <div className="space-y-4 pt-2 border-t border-border/60">
          <div className="flex items-center gap-1.5">
            <Sliders className="h-3.5 w-3.5 text-primary" />
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Bobot Relevansi BM25
            </h4>
          </div>

          {/* Title Weight Slider */}
          <div className="space-y-1.5">
            <div className="flex justify-between items-center text-xs">
              <span className="font-medium text-foreground">Bobot Judul & Nama Berkas</span>
              <span className="font-mono font-semibold text-primary">{titleWeight.toFixed(1)}x</span>
            </div>
            <Slider
              value={[titleWeight]}
              min={0.1}
              max={10.0}
              step={0.1}
              onValueChange={([val]) => setTitleWeight(Number(val.toFixed(1)))}
              aria-label="Bobot Judul BM25"
            />
            <p className="text-[11px] text-muted-foreground">
              Memprioritaskan kecocokan kata kunci pada judul dokumen dan nama berkas.
            </p>
          </div>

          {/* Tag Weight Slider */}
          <div className="space-y-1.5">
            <div className="flex justify-between items-center text-xs">
              <span className="font-medium text-foreground">Bobot Tag & Metadata</span>
              <span className="font-mono font-semibold text-primary">{tagWeight.toFixed(1)}x</span>
            </div>
            <Slider
              value={[tagWeight]}
              min={0.1}
              max={10.0}
              step={0.1}
              onValueChange={([val]) => setTagWeight(Number(val.toFixed(1)))}
              aria-label="Bobot Tag BM25"
            />
            <p className="text-[11px] text-muted-foreground">
              Memprioritaskan kecocokan pada kata kunci front-matter YAML dan tag.
            </p>
          </div>

          {/* Content Weight Slider */}
          <div className="space-y-1.5">
            <div className="flex justify-between items-center text-xs">
              <span className="font-medium text-foreground">Bobot Isi Dokumen</span>
              <span className="font-mono font-semibold text-primary">{contentWeight.toFixed(1)}x</span>
            </div>
            <Slider
              value={[contentWeight]}
              min={0.1}
              max={10.0}
              step={0.1}
              onValueChange={([val]) => setContentWeight(Number(val.toFixed(1)))}
              aria-label="Bobot Isi Dokumen BM25"
            />
            <p className="text-[11px] text-muted-foreground">
              Memprioritaskan kecocokan pada isi teks utama atau baris source code.
            </p>
          </div>
        </div>
      </div>

      <DialogFooter className="flex items-center justify-between gap-2 sm:justify-between pt-3 border-t border-border">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={handleResetToDefault}
          disabled={updateMutation.isPending}
          className="text-xs cursor-pointer"
        >
          <RotateCcw className="h-3.5 w-3.5 mr-1.5" />
          Reset ke Default
        </Button>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onClose}
            disabled={updateMutation.isPending}
            className="text-xs cursor-pointer"
          >
            Batal
          </Button>
          <Button
            type="button"
            variant="default"
            size="sm"
            onClick={handleSave}
            disabled={updateMutation.isPending}
            className="text-xs cursor-pointer"
          >
            {updateMutation.isPending ? (
              <>
                <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                Menyimpan...
              </>
            ) : (
              <>
                <Save className="h-3.5 w-3.5 mr-1.5" />
                Simpan Pengaturan
              </>
            )}
          </Button>
        </div>
      </DialogFooter>
    </>
  );
}

export function SettingsModal() {
  const settingsModalOpen = useUIStore((state) => state.settingsModalOpen);
  const setSettingsModalOpen = useUIStore((state) => state.setSettingsModalOpen);

  const { data: settings, isLoading, isError, error, refetch } = useSettingsQuery();

  return (
    <Dialog open={settingsModalOpen} onOpenChange={setSettingsModalOpen}>
      <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-md bg-primary/10 text-primary border border-primary/20">
              <Settings className="h-5 w-5" />
            </div>
            <div>
              <DialogTitle className="text-base font-semibold">Search Settings</DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground mt-0.5">
                Konfigurasi batas berkas, pola yang diabaikan, dan bobot relevansi algoritma BM25.
              </DialogDescription>
            </div>

          </div>
        </DialogHeader>

        {isLoading ? (
          <div className="py-6 space-y-4">
            <Skeleton className="h-10 w-full rounded-md" />
            <Skeleton className="h-20 w-full rounded-md" />
            <Skeleton className="h-28 w-full rounded-md" />
          </div>
        ) : isError ? (
          <div className="py-8 flex flex-col items-center justify-center text-center space-y-3">
            <div className="p-2.5 rounded-full bg-destructive/10 text-destructive">
              <AlertCircle className="h-6 w-6" />
            </div>
            <div>
              <p className="text-sm font-medium text-destructive">Gagal Memuat Pengaturan</p>
              <p className="text-xs text-muted-foreground mt-0.5">
                {error?.message || 'Tidak dapat terhubung ke backend service.'}
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void refetch()}
              className="text-xs cursor-pointer"
            >
              <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
              Coba Lagi
            </Button>
          </div>
        ) : settings ? (
          <SettingsForm
            key={settings.max_file_size_bytes + '-' + settings.weights.title}
            settings={settings}
            onClose={() => setSettingsModalOpen(false)}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
