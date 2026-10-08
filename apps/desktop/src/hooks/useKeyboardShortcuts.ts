import { useEffect, useRef, type RefObject } from 'react';
import { useUIStore } from '../stores/uiStore';
import { useSearchStore } from '../stores/searchStore';

export interface UseKeyboardShortcutsOptions {
  searchInputRef?: RefObject<HTMLInputElement | null>;
  onOpenSelectedFile?: () => void;
  onCopySelectedPath?: () => void;
}

export function useKeyboardShortcuts(options: UseKeyboardShortcutsOptions = {}) {
  const optionsRef = useRef(options);

  useEffect(() => {
    optionsRef.current = options;
  }, [options]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      const currentOpts = optionsRef.current;
      const activeEl = document.activeElement;
      const isInputActive =
        activeEl instanceof HTMLInputElement ||
        activeEl instanceof HTMLTextAreaElement ||
        (activeEl instanceof HTMLElement && activeEl.isContentEditable);

      const isCmd = event.metaKey || event.ctrlKey;
      const ui = useUIStore.getState();
      const search = useSearchStore.getState();

      // 1. Contextual Escape key hierarchy
      if (event.key === 'Escape') {
        if (ui.folderModalOpen) {
          event.preventDefault();
          ui.setFolderModalOpen(false);
          return;
        }
        if (ui.settingsModalOpen) {
          event.preventDefault();
          ui.setSettingsModalOpen(false);
          return;
        }
        if (ui.statsModalOpen) {
          event.preventDefault();
          ui.setStatsModalOpen(false);
          return;
        }
        if (ui.jobDrawerExpanded) {
          event.preventDefault();
          ui.setJobDrawerExpanded(false);
          return;
        }
        if (isInputActive && activeEl instanceof HTMLElement) {
          event.preventDefault();
          if (search.rawQuery) {
            search.setRawQuery('');
          } else {
            activeEl.blur();
          }
          return;
        }
        if (!ui.previewCollapsed) {
          event.preventDefault();
          ui.setPreviewCollapsed(true);
          return;
        }
        if (search.selectedDocId) {
          event.preventDefault();
          search.setSelectedDocId(null);
          return;
        }
      }

      // 2. Cmd/Ctrl + K or / to focus search input
      const isCmdK = isCmd && event.key.toLowerCase() === 'k';
      const isSlash = event.key === '/' && !isInputActive && !isCmd && !event.altKey;

      if (isCmdK || isSlash) {
        event.preventDefault();
        currentOpts.searchInputRef?.current?.focus();
        currentOpts.searchInputRef?.current?.select();
        return;
      }

      // 3. Document Action Shortcuts (Cmd+O: Open Editor, Cmd+Shift+C: Copy Path)
      if (isCmd && event.shiftKey && event.key.toLowerCase() === 'c') {
        if (currentOpts.onCopySelectedPath) {
          event.preventDefault();
          currentOpts.onCopySelectedPath();
          return;
        }
      }

      if (isCmd && !event.shiftKey && event.key.toLowerCase() === 'o') {
        if (currentOpts.onOpenSelectedFile) {
          event.preventDefault();
          currentOpts.onOpenSelectedFile();
          return;
        }
      }

      // 4. Single-character shortcuts blocked when typing in text input/textarea
      if (isInputActive) {
        return;
      }

      // [ toggles sidebar facet pane
      if (event.key === '[') {
        event.preventDefault();
        ui.toggleSidebar();
        return;
      }

      // ] toggles preview pane
      if (event.key === ']') {
        event.preventDefault();
        ui.togglePreview();
        return;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
}
