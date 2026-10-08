import * as React from 'react';
import {
  Code2,
  FileCode,
  Terminal,
  ExternalLink,
  Check,
  Sparkles,
} from 'lucide-react';
import { toast } from 'sonner';
import { useUIStore } from '../../stores/uiStore';
import { openFileInEditor } from '../../lib/desktop-bridge';
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
import { Checkbox } from '../ui/checkbox';
import { cn } from '../../lib/utils';

export interface EditorChoice {
  id: string;
  name: string;
  desc: string;
  cmd: string;
  icon: React.ComponentType<{ className?: string }>;
  recommended?: boolean;
}

const PRESET_EDITORS: EditorChoice[] = [
  {
    id: 'antigravity-ide',
    name: 'Antigravity IDE',
    desc: 'IDE utama pengembang (VS Code base)',
    cmd: 'antigravity-ide',
    icon: Sparkles,
    recommended: true,
  },
  {
    id: 'code',
    name: 'Visual Studio Code',
    desc: 'Buka via perintah CLI "code"',
    cmd: 'code',
    icon: Code2,
  },
  {
    id: 'gnome-text-editor',
    name: 'GNOME Text Editor',
    desc: 'Editor teks bawaan sistem desktop Linux',
    cmd: 'gnome-text-editor',
    icon: FileCode,
  },
  {
    id: 'default',
    name: 'Aplikasi Bawaan OS',
    desc: 'Buka via asosiasi file sistem operasi (xdg-open)',
    cmd: 'default',
    icon: ExternalLink,
  },
  {
    id: 'custom',
    name: 'Perintah Kustom...',
    desc: 'Tuliskan nama binary atau executable lain (contoh: cursor, nvim)',
    cmd: 'custom',
    icon: Terminal,
  },
];

interface OpenWithFormProps {
  targetPath: string;
  preferredEditor: string | null;
  onClose: () => void;
  onSetPreferredEditor: (editor: string | null) => void;
}

function OpenWithForm({
  targetPath,
  preferredEditor,
  onClose,
  onSetPreferredEditor,
}: OpenWithFormProps) {
  const [selectedId, setSelectedId] = React.useState<string>(() => {
    if (preferredEditor && PRESET_EDITORS.some((e) => e.cmd === preferredEditor)) {
      return preferredEditor;
    }
    if (preferredEditor) {
      return 'custom';
    }
    return 'antigravity-ide';
  });

  const [customCmd, setCustomCmd] = React.useState<string>(() => {
    return preferredEditor && !PRESET_EDITORS.some((e) => e.cmd === preferredEditor) ? preferredEditor : '';
  });

  const [rememberChoice, setRememberChoice] = React.useState<boolean>(true);
  const [isOpening, setIsOpening] = React.useState<boolean>(false);

  const fileName = targetPath.split(/[\\/]/).pop() || targetPath;

  const handleOpen = async () => {
    let finalCmd = selectedId;
    if (selectedId === 'custom') {
      const trimmed = customCmd.trim();
      if (!trimmed) {
        toast.error('Tuliskan perintah executable editor kustom');
        return;
      }
      finalCmd = trimmed;
    }

    setIsOpening(true);
    try {
      if (rememberChoice) {
        onSetPreferredEditor(finalCmd);
      }
      await openFileInEditor(targetPath, finalCmd);
      const appName = selectedId === 'custom' ? finalCmd : PRESET_EDITORS.find((e) => e.id === selectedId)?.name || finalCmd;
      toast.success(`Membuka "${fileName}" di ${appName}`);
      onClose();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      toast.error(`Gagal membuka berkas: ${msg}`);
    } finally {
      setIsOpening(false);
    }
  };

  return (
    <DialogContent className="max-w-md w-full bg-card border-border/80 shadow-2xl p-6">
      <DialogHeader className="space-y-1">
        <DialogTitle className="text-base font-semibold text-foreground flex items-center gap-2">
          <Code2 className="h-4 w-4 text-primary" />
          <span>Buka Berkas Dengan...</span>
        </DialogTitle>
        <DialogDescription className="text-xs text-muted-foreground truncate" title={targetPath}>
          Pilih editor untuk membuka: <span className="font-mono text-foreground">{fileName}</span>
        </DialogDescription>
      </DialogHeader>

      <div className="py-3 space-y-2">
        {PRESET_EDITORS.map((editor) => {
          const Icon = editor.icon;
          const isSelected = selectedId === editor.id;

          return (
            <div
              key={editor.id}
              role="button"
              tabIndex={0}
              onClick={() => setSelectedId(editor.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setSelectedId(editor.id);
                }
              }}
              className={cn(
                'flex items-center justify-between p-2.5 rounded-lg border text-left cursor-pointer transition-colors outline-none focus-visible:ring-1 focus-visible:ring-primary',
                isSelected
                  ? 'border-primary/60 bg-primary/10 text-foreground'
                  : 'border-border/60 bg-secondary/30 hover:bg-secondary/60 text-muted-foreground hover:text-foreground',
              )}
            >
              <div className="flex items-center gap-3 min-w-0">
                <div
                  className={cn(
                    'p-2 rounded-md shrink-0',
                    isSelected ? 'bg-primary/20 text-primary' : 'bg-muted/40 text-muted-foreground',
                  )}
                >
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-medium text-foreground">{editor.name}</span>
                    {editor.recommended && (
                      <span className="text-[10px] bg-primary/20 text-primary px-1.5 py-0.2 rounded font-mono">
                        Rekomendasi
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-muted-foreground truncate">{editor.desc}</p>
                </div>
              </div>

              {isSelected && <Check className="h-4 w-4 text-primary shrink-0 ml-2" />}
            </div>
          );
        })}

        {selectedId === 'custom' && (
          <div className="pt-2">
            <label htmlFor="custom-editor-input" className="text-[11px] font-mono text-muted-foreground block mb-1">
              Executable Command:
            </label>
            <Input
              id="custom-editor-input"
              value={customCmd}
              onChange={(e) => setCustomCmd(e.target.value)}
              placeholder="misal: cursor, nvim, gedit"
              className="h-8 text-xs font-mono"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  void handleOpen();
                }
              }}
            />
          </div>
        )}
      </div>

      <div className="flex items-center space-x-2 pt-1 border-t border-border/40">
        <Checkbox
          id="remember-editor"
          checked={rememberChoice}
          onCheckedChange={(checked) => setRememberChoice(Boolean(checked))}
        />
        <label
          htmlFor="remember-editor"
          className="text-xs text-muted-foreground cursor-pointer select-none leading-none"
        >
          Ingat pilihan ini (jadikan editor default)
        </label>
      </div>

      <DialogFooter className="mt-4 gap-2 sm:gap-0">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onClose}
          className="text-xs text-muted-foreground hover:text-foreground cursor-pointer"
        >
          Batal
        </Button>
        <Button
          type="button"
          variant="default"
          size="sm"
          onClick={handleOpen}
          disabled={isOpening}
          className="text-xs cursor-pointer font-medium"
        >
          {isOpening ? 'Membuka...' : 'Buka Berkas'}
        </Button>
      </DialogFooter>
    </DialogContent>
  );
}

export function OpenWithModal() {
  const openWithModalOpen = useUIStore((state) => state.openWithModalOpen);
  const openWithTargetPath = useUIStore((state) => state.openWithTargetPath);
  const preferredEditor = useUIStore((state) => state.preferredEditor);
  const setPreferredEditor = useUIStore((state) => state.setPreferredEditor);
  const closeOpenWith = useUIStore((state) => state.closeOpenWith);

  if (!openWithModalOpen || !openWithTargetPath) {
    return null;
  }

  return (
    <Dialog open={openWithModalOpen} onOpenChange={(open) => !open && closeOpenWith()}>
      <OpenWithForm
        key={openWithTargetPath}
        targetPath={openWithTargetPath}
        preferredEditor={preferredEditor}
        onClose={closeOpenWith}
        onSetPreferredEditor={setPreferredEditor}
      />
    </Dialog>
  );
}
