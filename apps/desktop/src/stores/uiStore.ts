import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type AppTheme = 'dark' | 'light' | 'system';
export type AppTab = 'search' | 'folders' | 'settings';

export interface UIState {
  sidebarCollapsed: boolean;
  previewCollapsed: boolean;
  previewWidth: number;
  theme: AppTheme;
  activeTab: AppTab;
  indexingJobId: string | null;
  folderModalOpen: boolean;
  settingsModalOpen: boolean;

  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  togglePreview: () => void;
  setPreviewCollapsed: (collapsed: boolean) => void;
  setPreviewWidth: (width: number) => void;
  setTheme: (theme: AppTheme) => void;
  setActiveTab: (activeTab: AppTab) => void;
  setIndexingJobId: (indexingJobId: string | null) => void;
  setFolderModalOpen: (open: boolean) => void;
  setSettingsModalOpen: (open: boolean) => void;
}

export const MIN_PREVIEW_WIDTH = 420;
export const MAX_PREVIEW_WIDTH = 640;
export const DEFAULT_PREVIEW_WIDTH = 480;

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      previewCollapsed: false,
      previewWidth: DEFAULT_PREVIEW_WIDTH,
      theme: 'dark',
      activeTab: 'search',
      indexingJobId: null,
      folderModalOpen: false,
      settingsModalOpen: false,

      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
      togglePreview: () => set((state) => ({ previewCollapsed: !state.previewCollapsed })),
      setPreviewCollapsed: (previewCollapsed) => set({ previewCollapsed }),
      setPreviewWidth: (width) =>
        set({
          previewWidth: Math.max(MIN_PREVIEW_WIDTH, Math.min(MAX_PREVIEW_WIDTH, Math.round(width))),
        }),
      setTheme: (theme) => set({ theme }),
      setActiveTab: (activeTab) => set({ activeTab }),
      setIndexingJobId: (indexingJobId) => set({ indexingJobId }),
      setFolderModalOpen: (folderModalOpen) => set({ folderModalOpen }),
      setSettingsModalOpen: (settingsModalOpen) => set({ settingsModalOpen }),
    }),
    {
      name: 'lynxsearch-ui-storage',
      partialize: (state) => ({
        theme: state.theme,
        sidebarCollapsed: state.sidebarCollapsed,
        previewCollapsed: state.previewCollapsed,
        previewWidth: state.previewWidth,
      }),
    },
  ),
);
