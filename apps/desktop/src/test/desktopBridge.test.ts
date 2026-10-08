import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as desktopBridge from '../lib/desktop-bridge';

describe('desktop-bridge', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('web fallback environment', () => {
    it('detects non-Tauri environment correctly', () => {
      expect(desktopBridge.isTauriEnvironment()).toBe(false);
    });

    it('falls back to window.prompt for pickDirectory when available', async () => {
      const promptMock = vi.fn().mockReturnValue('/user/custom/path');
      window.prompt = promptMock;
      const result = await desktopBridge.pickDirectory();
      expect(promptMock).toHaveBeenCalled();
      expect(result).toBe('/user/custom/path');
    });

    it('falls back to window.open for openFileInEditor when not in Tauri', async () => {
      const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
      await desktopBridge.openFileInEditor('/user/custom/file.rs');
      expect(openSpy).toHaveBeenCalledWith('file:///user/custom/file.rs', '_blank');
    });

    it('falls back to window.open for revealFileInFolder when not in Tauri', async () => {
      const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
      await desktopBridge.revealFileInFolder('/user/custom/file.rs');
      expect(openSpy).toHaveBeenCalledWith('file:///user/custom/file.rs', '_blank');
    });

    it('falls back to navigator.clipboard for copyTextToClipboard', async () => {
      const writeTextMock = vi.fn().mockResolvedValue(undefined);
      Object.defineProperty(navigator, 'clipboard', {
        value: { writeText: writeTextMock },
        configurable: true,
      });

      await desktopBridge.copyTextToClipboard('sample path');
      expect(writeTextMock).toHaveBeenCalledWith('sample path');
    });
  });
});
