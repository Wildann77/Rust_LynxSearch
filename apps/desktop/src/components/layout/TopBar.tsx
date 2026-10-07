import * as React from 'react';
import {
  Folder,
  Settings,
  PanelLeft,
  PanelRight,
} from 'lucide-react';
import { SearchBar } from '../search/SearchBar';
import { Button } from '../ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { useUIStore } from '../../stores/uiStore';
import { useHealthQuery } from '../../hooks/useHealthQuery';
import { BACKEND_URL } from '../../api/config';
import { cn } from '../../lib/utils';

export interface TopBarProps {
  searchInputRef?: React.RefObject<HTMLInputElement | null>;
  className?: string;
}

export function TopBar({ searchInputRef, className }: TopBarProps) {
  const {
    sidebarCollapsed,
    previewCollapsed,
    toggleSidebar,
    togglePreview,
    setFolderModalOpen,
    setSettingsModalOpen,
  } = useUIStore();

  const { data: health, isLoading, isSuccess, isRefetching, refetch } = useHealthQuery({ refetchInterval: 10_000 });

  // Resolve health status color and label (hindari glitch saat background refetch otomatis)
  const isHealthy = isSuccess && health?.status === 'ok';
  const isDegraded = isSuccess && health?.status !== 'ok';
  const isConnecting = (isLoading && !health) || (!isSuccess && isRefetching);

  const healthColor = isConnecting
    ? 'bg-amber-400 animate-pulse'
    : isHealthy
      ? 'bg-[#10a37f]'
      : isDegraded
        ? 'bg-amber-500'
        : 'bg-destructive';

  const healthText = isConnecting
    ? 'Connecting to backend...'
    : isHealthy
      ? `Backend Ready (${health?.version ?? 'v1.0.0'})`
      : isDegraded
        ? 'Backend Degraded (Storage Unavailable)'
        : `Backend Offline (${BACKEND_URL.replace(/^https?:\/\//, '')})`;

  return (
    <header
      data-tauri-drag-region
      className={cn(
        'flex h-12 w-full shrink-0 items-center justify-between border-b border-border bg-card px-3 select-none',
        className,
      )}
    >
      {/* Left Region: Logo & Sidebar Toggle */}
      <div className="flex items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleSidebar}
              className={cn(
                'h-8 w-8 text-muted-foreground hover:text-foreground cursor-pointer',
                !sidebarCollapsed && 'text-foreground bg-secondary/60',
              )}
              aria-label="Toggle Facet Sidebar ([)"
            >
              <PanelLeft className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Toggle Facet Sidebar ([)</TooltipContent>
        </Tooltip>

        <div className="flex items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary text-black font-mono font-bold text-xs select-none">
            L
          </div>
          <span className="font-semibold text-sm tracking-tight hidden md:inline-block">
            LynxSearch
          </span>
        </div>
      </div>

      {/* Center Region: Global Search Input */}
      <div className="flex flex-1 items-center justify-center max-w-xl mx-4">
        <SearchBar ref={searchInputRef} />
      </div>

      {/* Right Region: Health Dot, Folder, Settings, Preview Toggle */}
      <div className="flex items-center gap-1.5">
        {/* Backend Health Dot */}
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => void refetch()}
              className="flex items-center gap-1.5 px-2 py-1 rounded-md hover:bg-secondary/50 cursor-pointer transition-all duration-200 focus:outline-none"
              aria-label={`Backend Status: ${healthText}`}
            >
              <span className={cn('h-2.5 w-2.5 rounded-full shrink-0 transition-colors duration-300', healthColor)} />
              <span className="hidden xl:inline-block text-[11px] font-mono text-muted-foreground truncate max-w-[140px] transition-opacity duration-200">
                {isConnecting ? 'Connecting...' : isHealthy ? 'Ready' : 'Offline'}
              </span>
              {isRefetching && !isConnecting && (
                <span className="h-1.5 w-1.5 rounded-full bg-primary/40 animate-pulse shrink-0" title="Sinkronisasi status..." />
              )}
            </button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs font-mono">
            {healthText} (Klik untuk hubungkan ulang)
          </TooltipContent>
        </Tooltip>

        {/* Folder Manager Button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setFolderModalOpen(true)}
              className="h-8 w-8 text-muted-foreground hover:text-foreground cursor-pointer"
              aria-label="Open Folder Manager"
            >
              <Folder className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Folder Manager</TooltipContent>
        </Tooltip>

        {/* Settings Button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setSettingsModalOpen(true)}
              className="h-8 w-8 text-muted-foreground hover:text-foreground cursor-pointer"
              aria-label="Open Settings"
            >
              <Settings className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Settings</TooltipContent>
        </Tooltip>

        {/* Preview Panel Toggle */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={togglePreview}
              className={cn(
                'h-8 w-8 text-muted-foreground hover:text-foreground cursor-pointer',
                !previewCollapsed && 'text-foreground bg-secondary/60',
              )}
              aria-label="Toggle Document Preview (])"
            >
              <PanelRight className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Toggle Preview (])</TooltipContent>
        </Tooltip>
      </div>
    </header>
  );
}

