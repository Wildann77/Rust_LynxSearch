import { useEffect, type RefObject } from 'react';
import { useUIStore } from '../stores/uiStore';

export interface UseKeyboardShortcutsOptions {
  searchInputRef?: RefObject<HTMLInputElement | null>;
}

export function useKeyboardShortcuts(options: UseKeyboardShortcutsOptions = {}) {
  const { toggleSidebar, togglePreview, folderModalOpen, settingsModalOpen, setFolderModalOpen, setSettingsModalOpen } = useUIStore();

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      const activeEl = document.activeElement;
      const isInputActive =
        activeEl instanceof HTMLInputElement ||
        activeEl instanceof HTMLTextAreaElement ||
        (activeEl instanceof HTMLElement && activeEl.isContentEditable);

      // Escape key behavior
      if (event.key === 'Escape') {
        if (folderModalOpen) {
          setFolderModalOpen(false);
          return;
        }
        if (settingsModalOpen) {
          setSettingsModalOpen(false);
          return;
        }
        if (isInputActive && activeEl instanceof HTMLElement) {
          activeEl.blur();
          return;
        }
      }

      // Cmd/Ctrl + K or / to focus search
      const isCmdK = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k';
      const isSlash = event.key === '/' && !isInputActive && !event.metaKey && !event.ctrlKey && !event.altKey;

      if (isCmdK || isSlash) {
        event.preventDefault();
        options.searchInputRef?.current?.focus();
        options.searchInputRef?.current?.select();
        return;
      }

      // If typing in input, ignore single-character shortcuts
      if (isInputActive) {
        return;
      }

      // [ toggles sidebar facet pane
      if (event.key === '[') {
        event.preventDefault();
        toggleSidebar();
        return;
      }

      // ] toggles preview pane
      if (event.key === ']') {
        event.preventDefault();
        togglePreview();
        return;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    options.searchInputRef,
    toggleSidebar,
    togglePreview,
    folderModalOpen,
    settingsModalOpen,
    setFolderModalOpen,
    setSettingsModalOpen,
  ]);
}
