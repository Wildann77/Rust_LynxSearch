import * as React from 'react';
import {
  Folder,
  Settings,
  BarChart3,
  PanelLeft,
  PanelRight,
} from 'lucide-react';
import { SearchBar } from '../search/SearchBar';
import { Button } from '../ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { HealthDot } from '../common/HealthDot';
import { useUIStore } from '../../stores/uiStore';
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
    setStatsModalOpen,
  } = useUIStore();

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

      {/* Right Region: Health Dot, Folder, Stats, Settings, Preview Toggle */}
      <div className="flex items-center gap-1.5">
        {/* Backend Health Dot */}
        <HealthDot />

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

        {/* Index Statistics Button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setStatsModalOpen(true)}
              className="h-8 w-8 text-muted-foreground hover:text-foreground cursor-pointer"
              aria-label="Open Index Statistics"
            >
              <BarChart3 className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Index Statistics</TooltipContent>
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

