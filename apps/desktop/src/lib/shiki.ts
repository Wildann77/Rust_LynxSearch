import {
  createBundledHighlighter,
  type HighlighterGeneric,
  type ThemedToken,
} from '@shikijs/core';
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript';

export type TokenLine = ThemedToken[];

export const DEFAULT_SHIKI_THEME = 'github-dark-default';

export const CORE_SHIKI_LANGUAGES = [
  'rust',
  'typescript',
  'javascript',
  'python',
  'go',
  'c',
  'cpp',
  'csharp',
  'java',
  'kotlin',
  'ruby',
  'php',
  'swift',
  'bash',
  'sql',
  'html',
  'css',
  'scss',
  'json',
  'yaml',
  'toml',
  'markdown',
  'xml',
  'lua',
] as const;

const bundledLanguages = {
  rust: () => import('@shikijs/langs/rust'),
  typescript: () => import('@shikijs/langs/typescript'),
  javascript: () => import('@shikijs/langs/javascript'),
  tsx: () => import('@shikijs/langs/typescript'),
  jsx: () => import('@shikijs/langs/javascript'),
  python: () => import('@shikijs/langs/python'),
  go: () => import('@shikijs/langs/go'),
  c: () => import('@shikijs/langs/c'),
  cpp: () => import('@shikijs/langs/c'),
  csharp: () => import('@shikijs/langs/java'),
  java: () => import('@shikijs/langs/java'),
  kotlin: () => import('@shikijs/langs/kotlin'),
  ruby: () => import('@shikijs/langs/python'),
  php: () => import('@shikijs/langs/c'),
  swift: () => import('@shikijs/langs/rust'),
  bash: () => import('@shikijs/langs/bash'),
  sql: () => import('@shikijs/langs/sql'),
  html: () => import('@shikijs/langs/html'),
  css: () => import('@shikijs/langs/css'),
  scss: () => import('@shikijs/langs/css'),
  json: () => import('@shikijs/langs/json'),
  yaml: () => import('@shikijs/langs/yaml'),
  toml: () => import('@shikijs/langs/toml'),
  markdown: () => import('@shikijs/langs/markdown'),
  xml: () => import('@shikijs/langs/html'),
  lua: () => import('@shikijs/langs/lua'),
};

const bundledThemes = {
  'github-dark-default': () => import('@shikijs/themes/github-dark-default'),
};

export type BundledLang = keyof typeof bundledLanguages;
export type BundledTheme = keyof typeof bundledThemes;
export type Highlighter = HighlighterGeneric<BundledLang, BundledTheme>;

const createCustomHighlighter = createBundledHighlighter({
  langs: bundledLanguages,
  themes: bundledThemes,
  engine: () => createJavaScriptRegexEngine(),
});

/**
 * Map document language or file extension to canonical Shiki language identifier.
 * Returns null if language is unsupported or plain text.
 */
export function mapLanguageToShiki(lang?: string | null): string | null {
  if (!lang) return null;
  const l = lang.toLowerCase().trim();

  switch (l) {
    case 'rs':
    case 'rust':
      return 'rust';
    case 'ts':
    case 'typescript':
      return 'typescript';
    case 'tsx':
      return 'tsx';
    case 'js':
    case 'javascript':
    case 'mjs':
    case 'cjs':
      return 'javascript';
    case 'jsx':
      return 'jsx';
    case 'py':
    case 'python':
      return 'python';
    case 'go':
    case 'golang':
      return 'go';
    case 'c':
    case 'h':
      return 'c';
    case 'cpp':
    case 'cc':
    case 'cxx':
    case 'hpp':
      return 'cpp';
    case 'cs':
    case 'csharp':
      return 'csharp';
    case 'java':
      return 'java';
    case 'kt':
    case 'kts':
    case 'kotlin':
      return 'kotlin';
    case 'rb':
    case 'ruby':
      return 'ruby';
    case 'php':
      return 'php';
    case 'swift':
      return 'swift';
    case 'sh':
    case 'bash':
    case 'zsh':
    case 'shell':
      return 'bash';
    case 'sql':
      return 'sql';
    case 'html':
    case 'htm':
      return 'html';
    case 'css':
      return 'css';
    case 'scss':
      return 'scss';
    case 'json':
      return 'json';
    case 'yaml':
    case 'yml':
      return 'yaml';
    case 'toml':
      return 'toml';
    case 'xml':
    case 'svg':
      return 'xml';
    case 'md':
    case 'markdown':
      return 'markdown';
    case 'lua':
      return 'lua';
    default:
      return null;
  }
}

/**
 * Split plain text code into line-by-line single tokens as fast synchronous fallback.
 * Preserves exact whitespace and line integrity.
 */
export function splitPlainLines(code: string): TokenLine[] {
  const lines = code.split('\n');
  return lines.map((line, lineIdx) => [
    {
      content: line,
      offset: lineIdx,
    },
  ]);
}

let highlighterInstance: Promise<Highlighter> | null = null;

/**
 * Lazy singleton instance getter for Shiki highlighter.
 */
export async function getHighlighter(): Promise<Highlighter> {
  if (!highlighterInstance) {
    highlighterInstance = createCustomHighlighter({
      themes: [DEFAULT_SHIKI_THEME],
      langs: ['rust', 'typescript', 'javascript'],
    });
  }
  return highlighterInstance;
}

// In-memory cache for tokenized lines to avoid re-highlighting on scroll or re-render
const tokenCache = new Map<string, TokenLine[]>();
const MAX_CACHE_SIZE = 100;

function computeCacheKey(code: string, mappedLang: string | null): string {
  const head = code.slice(0, 32);
  const tail = code.slice(-32);
  return `${mappedLang ?? 'plain'}:${code.length}:${head}:${tail}`;
}

/**
 * Highlights full source code using Shiki with caching and safe plain-text fallback.
 */
export async function highlightCode(
  code: string,
  lang?: string | null,
): Promise<TokenLine[]> {
  const mappedLang = mapLanguageToShiki(lang);
  if (!mappedLang) {
    return splitPlainLines(code);
  }

  const cacheKey = computeCacheKey(code, mappedLang);
  const cached = tokenCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  try {
    const highlighter = await getHighlighter();
    const loadedLangs = highlighter.getLoadedLanguages();

    if (!loadedLangs.includes(mappedLang)) {
      try {
        await highlighter.loadLanguage(mappedLang as BundledLang);
      } catch {
        return splitPlainLines(code);
      }
    }

    const result = highlighter.codeToTokens(code, {
      lang: mappedLang as BundledLang,
      theme: DEFAULT_SHIKI_THEME,
    });

    const tokens: TokenLine[] = result.tokens.map((line) =>
      line.length > 0 ? line : [{ content: '', offset: 0 }],
    );

    if (tokenCache.size >= MAX_CACHE_SIZE) {
      const firstKey = tokenCache.keys().next().value;
      if (firstKey) {
        tokenCache.delete(firstKey);
      }
    }
    tokenCache.set(cacheKey, tokens);

    return tokens;
  } catch {
    return splitPlainLines(code);
  }
}

/**
 * Clear cached tokenized documents. Useful for testing or memory flush.
 */
export function clearTokenCache(): void {
  tokenCache.clear();
}
