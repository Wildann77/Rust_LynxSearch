import * as React from 'react';
import { TopBar } from './TopBar';
import { FacetSidebar } from './FacetSidebar';
import { ResultsPane } from './ResultsPane';
import { PreviewPane } from './PreviewPane';
import { Resizer } from './Resizer';
import { BottomStatusBar } from './BottomStatusBar';
import { JobProgressIndicator } from '../jobs/JobProgressIndicator';
import { FolderManagerModal } from '../modals/FolderManagerModal';
import { SettingsModal } from '../modals/SettingsModal';
import { Toaster } from 'sonner';
import { TooltipProvider } from '../ui/tooltip';
import { useKeyboardShortcuts } from '../../hooks/useKeyboardShortcuts';
import { useUIStore } from '../../stores/uiStore';
import { cn } from '../../lib/utils';

export interface WorkspaceShellProps {
  className?: string;
}

export function WorkspaceShell({ className }: WorkspaceShellProps) {
  const searchInputRef = React.useRef<HTMLInputElement | null>(null);
  const previewCollapsed = useUIStore((state) => state.previewCollapsed);

  // Hook global keyboard navigation (Cmd+K, /, [, ], Escape)
  useKeyboardShortcuts({ searchInputRef });

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

        {/* Global Notifications Toast */}
        <Toaster theme="dark" position="bottom-right" richColors />
      </div>
    </TooltipProvider>
  );
}
