import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export function isTauriEnvironment(): boolean {
  return (
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
  );
}

export async function pickDirectory(): Promise<string | null> {
  if (isTauriEnvironment()) {
    try {
      const result = await invoke<string | null>('pick_folder');
      // If invoke executed (returns folder path string or null if user canceled dialog),
      // return it directly. Do NOT fall back to plugin open when user intentionally clicked cancel!
      return result;
    } catch (err) {
      console.warn('[LynxSearch] invoke pick_folder failed, falling back to plugin open:', err);
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Pilih Folder untuk Di-index',
      });
      if (Array.isArray(selected)) {
        return selected[0] ?? null;
      }
      return selected;
    } catch (openErr) {
      console.error('[LynxSearch] pickDirectory dialog failed:', openErr);
      throw openErr;
    }
  }

  // Graceful browser/test fallback
  if (typeof window !== 'undefined' && typeof window.prompt === 'function') {
    return window.prompt('Masukkan absolute path direktori lokal:');
  }

  return null;
}

export async function openFileInEditor(path: string): Promise<void> {
  if (isTauriEnvironment()) {
    try {
      await invoke('open_file_in_editor', { path });
      return;
    } catch (invokeErr) {
      console.warn('[LynxSearch] invoke open_file_in_editor failed, falling back to openPath:', invokeErr);
      try {
        await openPath(path);
        return;
      } catch (openerErr) {
        console.error('[LynxSearch] openFileInEditor failed via both methods:', openerErr);
        throw openerErr;
      }
    }
  }

  // Graceful web fallback
  if (typeof window !== 'undefined') {
    window.open(`file://${path}`, '_blank');
  }
}

export async function copyTextToClipboard(text: string): Promise<void> {
  if (isTauriEnvironment()) {
    await writeText(text);
    return;
  }

  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  throw new Error('Clipboard API is not available');
}

export async function focusWindow(): Promise<void> {
  if (isTauriEnvironment()) {
    try {
      await invoke('focus_window');
      return;
    } catch {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        await win.setFocus();
      } catch {
        // Ignore in non-tauri or test environments
      }
    }
  }
}

export async function setDesktopZoom(scale: number): Promise<void> {
  if (isTauriEnvironment()) {
    try {
      await invoke('set_desktop_zoom', { scale });
    } catch (err) {
      console.warn('[LynxSearch] invoke set_desktop_zoom failed:', err);
    }
  }
}
