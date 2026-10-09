import { describe, it, expect } from 'vitest';
import {
  parseQueryFilters,
  buildQueryString,
  setQueryFilter,
  toggleQueryFilter,
  removeQueryFilter,
  clearAllQueryFilters,
} from '../lib/querySync';

describe('querySync (Phase 8.7)', () => {
  describe('parseQueryFilters', () => {
    it('parses plain free-text with no filters', () => {
      const res = parseQueryFilters('rust async tokio');
      expect(res.freeText).toBe('rust async tokio');
      expect(res.activeFilters).toEqual({});
    });

    it('parses single filter token and preserves free text', () => {
      const res = parseQueryFilters('tokio language:rust');
      expect(res.freeText).toBe('tokio');
      expect(res.activeFilters).toEqual({ language: 'rust' });
    });

    it('handles aliases: ext -> extension, lang -> language, tags -> tag', () => {
      const res = parseQueryFilters('ext:rs lang:rust tags:cli');
      expect(res.freeText).toBe('');
      expect(res.activeFilters).toEqual({
        extension: 'rs',
        language: 'rust',
        tag: 'cli',
      });
    });

    it('parses multi-value comma-separated filter tokens', () => {
      const res = parseQueryFilters('test tag:cli,web,api');
      expect(res.freeText).toBe('test');
      expect(res.activeFilters).toEqual({ tag: 'cli,web,api' });
    });

    it('consolidates repeated tokens into comma-separated values without duplicates', () => {
      const res = parseQueryFilters('tag:cli tag:web tag:cli');
      expect(res.freeText).toBe('');
      expect(res.activeFilters).toEqual({ tag: 'cli,web' });
    });

    it('parses quoted values in filter tokens', () => {
      const res = parseQueryFilters('search project:"lynx core"');
      expect(res.freeText).toBe('search');
      expect(res.activeFilters).toEqual({ project: 'lynx core' });
    });

    it('preserves quoted phrases in free-text without treating as filters', () => {
      const res = parseQueryFilters('"exact phrase search" type:code');
      expect(res.freeText).toBe('"exact phrase search"');
      expect(res.activeFilters).toEqual({ type: 'code' });
    });

    it('ignores non-filter colons such as URLs and Rust path syntax', () => {
      const res = parseQueryFilters('http://localhost::3000 std::path::PathBuf');
      expect(res.freeText).toBe('http://localhost::3000 std::path::PathBuf');
      expect(res.activeFilters).toEqual({});
    });
  });

  describe('buildQueryString', () => {
    it('serializes freeText and activeFilters correctly', () => {
      const str = buildQueryString('tokio runtime', {
        language: 'rust',
        type: 'code',
      });
      expect(str).toBe('tokio runtime type:code language:rust');
    });

    it('serializes without freeText when freeText is empty', () => {
      const str = buildQueryString('', { extension: 'rs,ts' });
      expect(str).toBe('extension:rs,ts');
    });

    it('quotes multi-word filter values when necessary', () => {
      const str = buildQueryString('query', { project: 'lynx core' });
      expect(str).toBe('query project:"lynx core"');
    });
  });

  describe('setQueryFilter', () => {
    it('adds new filter to query while preserving free text', () => {
      const { newRawQuery, newActiveFilters } = setQueryFilter(
        'tokio async',
        'language',
        'rust',
      );
      expect(newRawQuery).toBe('tokio async language:rust');
      expect(newActiveFilters).toEqual({ language: 'rust' });
    });

    it('replaces existing filter for same key', () => {
      const { newRawQuery, newActiveFilters } = setQueryFilter(
        'tokio language:go',
        'language',
        'rust',
      );
      expect(newRawQuery).toBe('tokio language:rust');
      expect(newActiveFilters).toEqual({ language: 'rust' });
    });

    it('removes filter when value is null or empty', () => {
      const { newRawQuery, newActiveFilters } = setQueryFilter(
        'tokio language:rust type:code',
        'language',
        null,
      );
      expect(newRawQuery).toBe('tokio type:code');
      expect(newActiveFilters).toEqual({ type: 'code' });
    });
  });

  describe('toggleQueryFilter', () => {
    it('appends value to existing filter key', () => {
      const { newRawQuery, newActiveFilters } = toggleQueryFilter(
        'tokio tag:cli',
        'tag',
        'web',
      );
      expect(newRawQuery).toBe('tokio tag:cli,web');
      expect(newActiveFilters).toEqual({ tag: 'cli,web' });
    });

    it('removes value when toggling existing value', () => {
      const { newRawQuery, newActiveFilters } = toggleQueryFilter(
        'tokio tag:cli,web',
        'tag',
        'cli',
      );
      expect(newRawQuery).toBe('tokio tag:web');
      expect(newActiveFilters).toEqual({ tag: 'web' });
    });

    it('removes token entirely when toggling last value', () => {
      const { newRawQuery, newActiveFilters } = toggleQueryFilter(
        'tokio tag:cli',
        'tag',
        'cli',
      );
      expect(newRawQuery).toBe('tokio');
      expect(newActiveFilters).toEqual({});
    });
  });

  describe('removeQueryFilter', () => {
    it('removes specific value and keeps remaining values', () => {
      const { newRawQuery, newActiveFilters } = removeQueryFilter(
        'search extension:rs,ts',
        'extension',
        'rs',
      );
      expect(newRawQuery).toBe('search extension:ts');
      expect(newActiveFilters).toEqual({ extension: 'ts' });
    });

    it('removes filter token completely when last value removed', () => {
      const { newRawQuery, newActiveFilters } = removeQueryFilter(
        'search extension:rs',
        'extension',
        'rs',
      );
      expect(newRawQuery).toBe('search');
      expect(newActiveFilters).toEqual({});
    });
  });

  describe('clearAllQueryFilters', () => {
    it('removes all filters from query string and keeps free text', () => {
      const { newRawQuery, newActiveFilters } = clearAllQueryFilters(
        'rust ownership tag:cli,web language:rust type:code',
      );
      expect(newRawQuery).toBe('rust ownership');
      expect(newActiveFilters).toEqual({});
    });
  });
});
