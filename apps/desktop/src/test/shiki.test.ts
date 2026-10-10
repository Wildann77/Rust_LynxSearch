import { describe, it, expect, beforeEach } from 'vitest';
import {
  mapLanguageToShiki,
  splitPlainLines,
  highlightCode,
  clearTokenCache,
  CORE_SHIKI_LANGUAGES,
} from '../lib/shiki';

describe('Shiki Integration Service (Phase 9.5)', () => {
  beforeEach(() => {
    clearTokenCache();
  });

  describe('mapLanguageToShiki', () => {
    it('maps language aliases and extensions correctly', () => {
      expect(mapLanguageToShiki('rs')).toBe('rust');
      expect(mapLanguageToShiki('rust')).toBe('rust');
      expect(mapLanguageToShiki('ts')).toBe('typescript');
      expect(mapLanguageToShiki('tsx')).toBe('tsx');
      expect(mapLanguageToShiki('js')).toBe('javascript');
      expect(mapLanguageToShiki('py')).toBe('python');
      expect(mapLanguageToShiki('go')).toBe('go');
      expect(mapLanguageToShiki('kt')).toBe('kotlin');
      expect(mapLanguageToShiki('cs')).toBe('csharp');
      expect(mapLanguageToShiki('cpp')).toBe('cpp');
      expect(mapLanguageToShiki('c')).toBe('c');
      expect(mapLanguageToShiki('sh')).toBe('bash');
      expect(mapLanguageToShiki('shell')).toBe('bash');
      expect(mapLanguageToShiki('yml')).toBe('yaml');
      expect(mapLanguageToShiki('yaml')).toBe('yaml');
      expect(mapLanguageToShiki('json')).toBe('json');
      expect(mapLanguageToShiki('toml')).toBe('toml');
      expect(mapLanguageToShiki('md')).toBe('markdown');
    });

    it('returns null for unsupported language or empty/null values', () => {
      expect(mapLanguageToShiki(null)).toBeNull();
      expect(mapLanguageToShiki(undefined)).toBeNull();
      expect(mapLanguageToShiki('')).toBeNull();
      expect(mapLanguageToShiki('unsupported_binary_xyz')).toBeNull();
    });

    it('all core languages are represented in mapping', () => {
      for (const lang of CORE_SHIKI_LANGUAGES) {
        expect(mapLanguageToShiki(lang)).toBe(lang);
      }
    });
  });

  describe('splitPlainLines', () => {
    it('splits multiline string and preserves exact whitespace and indentations', () => {
      const code = 'fn main() {\n    let x = 42;\n\n}';
      const lines = splitPlainLines(code);

      expect(lines.length).toBe(4);
      expect(lines[0][0].content).toBe('fn main() {');
      expect(lines[1][0].content).toBe('    let x = 42;');
      expect(lines[2][0].content).toBe('');
      expect(lines[3][0].content).toBe('}');
    });

    it('handles single line string', () => {
      const lines = splitPlainLines('const a = 1;');
      expect(lines.length).toBe(1);
      expect(lines[0][0].content).toBe('const a = 1;');
    });
  });

  describe('highlightCode', () => {
    it('highlights supported language (Rust) with syntax tokens', async () => {
      const code = 'fn main() {\n    println!("hello");\n}';
      const tokenLines = await highlightCode(code, 'rust');

      expect(tokenLines.length).toBe(3);
      // First line should contain tokens for 'fn', space, 'main', etc.
      const firstLineTokens = tokenLines[0];
      expect(firstLineTokens.length).toBeGreaterThan(1);
      expect(firstLineTokens.some((t) => t.content === 'fn')).toBe(true);
      expect(firstLineTokens.some((t) => t.color != null)).toBe(true);
    });

    it('falls back to plain lines when language is unsupported', async () => {
      const code = 'line 1\nline 2 with   spaces';
      const tokenLines = await highlightCode(code, 'unknown_custom_lang');

      expect(tokenLines.length).toBe(2);
      expect(tokenLines[0][0].content).toBe('line 1');
      expect(tokenLines[1][0].content).toBe('line 2 with   spaces');
    });

    it('reuses cached tokens on identical input', async () => {
      const code = 'let x = 10;';
      const firstRun = await highlightCode(code, 'typescript');
      const secondRun = await highlightCode(code, 'typescript');

      expect(secondRun).toBe(firstRun);
    });
  });
});
