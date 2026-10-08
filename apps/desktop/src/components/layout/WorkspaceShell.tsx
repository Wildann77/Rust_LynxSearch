import * as React from 'react';
import { toast } from 'sonner';
import { TopBar } from './TopBar';
import { FacetSidebar } from './FacetSidebar';
import { ResultsPane } from './ResultsPane';
import { PreviewPane } from './PreviewPane';
import { Resizer } from './Resizer';
import { BottomStatusBar } from './BottomStatusBar';
import { JobProgressIndicator } from '../jobs/JobProgressIndicator';
import { FolderManagerModal } from '../modals/FolderManagerModal';
import { SettingsModal } from '../modals/SettingsModal';
import { StatsModal } from '../modals/StatsModal';
import { OpenWithModal } from '../modals/OpenWithModal';
import { ConnectionBanner } from '../common/ConnectionBanner';
import { LynxToaster } from '../ui/LynxToaster';
import { TooltipProvider } from '../ui/tooltip';
import { useKeyboardShortcuts } from '../../hooks/useKeyboardShortcuts';
import { useDocumentDetailQuery } from '../../hooks/useDocumentQuery';
import { useUIStore } from '../../stores/uiStore';
import { useSearchStore } from '../../stores/searchStore';
import { computeFullPath } from '../../lib/path';
import { openFileInEditor, copyTextToClipboard } from '../../lib/desktop-bridge';
import { cn } from '../../lib/utils';

export interface WorkspaceShellProps {
  className?: string;
}

export function WorkspaceShell({ className }: WorkspaceShellProps) {
  const searchInputRef = React.useRef<HTMLInputElement | null>(null);
  const previewCollapsed = useUIStore((state) => state.previewCollapsed);
  const selectedDocId = useSearchStore((state) => state.selectedDocId);

  const { data: activeDoc } = useDocumentDetailQuery(selectedDocId);

  const handleOpenSelectedFile = React.useCallback(async () => {
    if (!activeDoc) return;
    const fullPath = computeFullPath(activeDoc.folder_root_path, activeDoc.relative_path);
    const preferredEditor = useUIStore.getState().preferredEditor;
    if (preferredEditor) {
      try {
        await openFileInEditor(fullPath, preferredEditor);
        toast.info(`Membuka berkas di ${preferredEditor}...`);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : 'Gagal membuka berkas';
        toast.error(`Gagal membuka berkas: ${msg}`);
      }
    } else {
      useUIStore.getState().openWith(fullPath);
    }
  }, [activeDoc]);

  const handleCopySelectedPath = React.useCallback(async () => {
    if (!activeDoc) return;
    const fullPath = computeFullPath(activeDoc.folder_root_path, activeDoc.relative_path);
    try {
      await copyTextToClipboard(fullPath);
      toast.success('Path berkas disalin ke clipboard');
    } catch {
      toast.error('Gagal menyalin path ke clipboard');
    }
  }, [activeDoc]);

  // Hook global keyboard navigation (Cmd+K, /, [, ], Escape, Cmd+O, Cmd+Shift+C)
  useKeyboardShortcuts({
    searchInputRef,
    onOpenSelectedFile: handleOpenSelectedFile,
    onCopySelectedPath: handleCopySelectedPath,
  });

  return (
    <TooltipProvider delayDuration={300}>
      <div
        className={cn(
          'flex h-screen w-screen flex-col bg-background text-foreground font-sans antialiased overflow-hidden select-none',
          className,
        )}
      >
        {/* Top Application Bar */}
        <TopBar searchInputRef={searchInputRef} />

        {/* Persistent Connection Outage Banner */}
        <ConnectionBanner />

        {/* 3-Pane Main Workspace */}
        <main className="flex flex-1 min-h-0 w-full overflow-hidden">
          {/* Left: Facet Filters Sidebar */}
          <FacetSidebar />

          {/* Center: Search Results Pane */}
          <ResultsPane />

          {/* Resizer Divider between Results & Preview */}
          {!previewCollapsed ? <Resizer /> : null}

          {/* Right: Document Preview Pane */}
          <PreviewPane />
        </main>

        {/* Job Progress Indicator Banner / Drawer */}
        <JobProgressIndicator />

        {/* Bottom Status & Hotkeys Bar */}
        <BottomStatusBar />

        {/* Dialog Overlays */}
        <FolderManagerModal />
        <SettingsModal />
        <StatsModal />
        <OpenWithModal />

        {/* Global Notifications Toast */}
        <LynxToaster />
      </div>
    </TooltipProvider>
  );
}
