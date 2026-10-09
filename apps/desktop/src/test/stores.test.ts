import { describe, it, expect, beforeEach } from 'vitest';
import { useSearchStore, useUIStore } from '../stores';

describe('Zustand Stores', () => {
  beforeEach(() => {
    useSearchStore.getState().resetSearch();
    useUIStore.setState({
      sidebarCollapsed: false,
      previewCollapsed: false,
      previewWidth: 480,
      theme: 'dark',
      activeTab: 'search',
      indexingJobId: null,
      folderModalOpen: false,
      settingsModalOpen: false,
    });
  });

  describe('useSearchStore', () => {
    it('initializes with default search state', () => {
      const state = useSearchStore.getState();
      expect(state.rawQuery).toBe('');
      expect(state.activeFilters).toEqual({});
      expect(state.page).toBe(1);
      expect(state.pageSize).toBe(20);
      expect(state.sort).toBe('relevance');
      expect(state.selectedDocId).toBeNull();
    });

    it('updates rawQuery and resets page to 1 while preserving sort', () => {
      useSearchStore.getState().setSort('modified_desc');
      useSearchStore.getState().setPage(3);
      expect(useSearchStore.getState().page).toBe(3);

      useSearchStore.getState().setRawQuery('authenticate');
      expect(useSearchStore.getState().rawQuery).toBe('authenticate');
      expect(useSearchStore.getState().page).toBe(1);
      expect(useSearchStore.getState().sort).toBe('modified_desc');
    });

    it('updates sort and resets page to 1', () => {
      useSearchStore.getState().setPage(4);
      useSearchStore.getState().setSort('name_asc');
      expect(useSearchStore.getState().sort).toBe('name_asc');
      expect(useSearchStore.getState().page).toBe(1);
    });

    it('preserves sort on page change', () => {
      useSearchStore.getState().setSort('size_desc');
      useSearchStore.getState().setPage(2);
      expect(useSearchStore.getState().sort).toBe('size_desc');
      expect(useSearchStore.getState().page).toBe(2);
    });

    it('resets sort on resetSearch', () => {
      useSearchStore.getState().setSort('name_desc');
      useSearchStore.getState().setPage(3);
      useSearchStore.getState().resetSearch();
      expect(useSearchStore.getState().sort).toBe('relevance');
      expect(useSearchStore.getState().page).toBe(1);
    });

    it('manages activeFilters individually and collectively', () => {
      useSearchStore.getState().setPage(2);
      useSearchStore.getState().setFilter('language', 'rust');
      useSearchStore.getState().setFilter('type', 'code');

      expect(useSearchStore.getState().activeFilters).toEqual({
        language: 'rust',
        type: 'code',
      });
      expect(useSearchStore.getState().page).toBe(1);

      // Removing one filter with null/undefined
      useSearchStore.getState().setFilter('language', null);
      expect(useSearchStore.getState().activeFilters).toEqual({
        type: 'code',
      });

      // Clear all
      useSearchStore.getState().clearFilters();
      expect(useSearchStore.getState().activeFilters).toEqual({});
    });

    it('supports multi-value filtering with toggleFilterValue and removeFilterValue', () => {
      useSearchStore.getState().toggleFilterValue('extension', 'rs');
      expect(useSearchStore.getState().activeFilters.extension).toBe('rs');

      // Toggle another extension
      useSearchStore.getState().toggleFilterValue('extension', 'ts');
      expect(useSearchStore.getState().activeFilters.extension).toBe('rs,ts');

      // Untoggle first extension
      useSearchStore.getState().toggleFilterValue('extension', 'rs');
      expect(useSearchStore.getState().activeFilters.extension).toBe('ts');

      // Remove specific value
      useSearchStore.getState().removeFilterValue('extension', 'ts');
      expect(useSearchStore.getState().activeFilters.extension).toBeUndefined();
    });

    it('manages selectedDocId, page, and pageSize', () => {
      const docId = '550e8400-e29b-41d4-a716-446655440000';
      useSearchStore.getState().setSelectedDocId(docId);
      expect(useSearchStore.getState().selectedDocId).toBe(docId);

      useSearchStore.getState().setPage(4);
      expect(useSearchStore.getState().page).toBe(4);

      useSearchStore.getState().setPageSize(50);
      expect(useSearchStore.getState().pageSize).toBe(50);
      expect(useSearchStore.getState().page).toBe(1);
    });

    it('resets entire search state cleanly', () => {
      useSearchStore.getState().setRawQuery('test query');
      useSearchStore.getState().setFilter('tag', 'auth');
      useSearchStore.getState().setSelectedDocId('abc');
      useSearchStore.getState().setPage(5);

      useSearchStore.getState().resetSearch();

      const state = useSearchStore.getState();
      expect(state.rawQuery).toBe('');
      expect(state.activeFilters).toEqual({});
      expect(state.page).toBe(1);
      expect(state.selectedDocId).toBeNull();
    });

    it('synchronizes facets and query bidirectionally (TASK.md 8.7)', () => {
      // 1. Preserve free-text terms and append token when clicking facet
      useSearchStore.getState().setRawQuery('tokio runtime');
      useSearchStore.getState().toggleFilterValue('language', 'rust');
      expect(useSearchStore.getState().rawQuery).toBe('tokio runtime language:rust');
      expect(useSearchStore.getState().activeFilters.language).toBe('rust');

      // 2. Add second value -> comma separated, avoid duplicate
      useSearchStore.getState().toggleFilterValue('tag', 'cli');
      useSearchStore.getState().toggleFilterValue('tag', 'web');
      expect(useSearchStore.getState().rawQuery).toBe(
        'tokio runtime language:rust tag:cli,web',
      );
      expect(useSearchStore.getState().activeFilters.tag).toBe('cli,web');

      // 3. Editing filter token in query input updates activeFilters
      useSearchStore
        .getState()
        .setRawQuery('tokio runtime language:go tag:cli,web type:code');
      expect(useSearchStore.getState().activeFilters.language).toBe('go');
      expect(useSearchStore.getState().activeFilters.type).toBe('code');
      expect(useSearchStore.getState().activeFilters.tag).toBe('cli,web');

      // 4. Removing token unchecks facet
      useSearchStore.getState().removeFilterValue('language', 'go');
      expect(useSearchStore.getState().rawQuery).toBe('tokio runtime type:code tag:cli,web');
      expect(useSearchStore.getState().activeFilters.language).toBeUndefined();

      // 5. Clear filters keeps free text intact
      useSearchStore.getState().clearFilters();
      expect(useSearchStore.getState().rawQuery).toBe('tokio runtime');
      expect(useSearchStore.getState().activeFilters).toEqual({});
    });
  });

  describe('useUIStore', () => {
    it('manages sidebar collapse toggle and explicit value', () => {
      expect(useUIStore.getState().sidebarCollapsed).toBe(false);

      useUIStore.getState().toggleSidebar();
      expect(useUIStore.getState().sidebarCollapsed).toBe(true);

      useUIStore.getState().setSidebarCollapsed(false);
      expect(useUIStore.getState().sidebarCollapsed).toBe(false);
    });

    it('manages preview collapse toggle and explicit value', () => {
      expect(useUIStore.getState().previewCollapsed).toBe(false);

      useUIStore.getState().togglePreview();
      expect(useUIStore.getState().previewCollapsed).toBe(true);

      useUIStore.getState().setPreviewCollapsed(false);
      expect(useUIStore.getState().previewCollapsed).toBe(false);
    });

    it('manages preview width with 420-640px clamping constraints', () => {
      expect(useUIStore.getState().previewWidth).toBe(480);

      // Within valid range
      useUIStore.getState().setPreviewWidth(550);
      expect(useUIStore.getState().previewWidth).toBe(550);

      // Clamped below minimum (420)
      useUIStore.getState().setPreviewWidth(300);
      expect(useUIStore.getState().previewWidth).toBe(420);

      // Clamped above maximum (640)
      useUIStore.getState().setPreviewWidth(900);
      expect(useUIStore.getState().previewWidth).toBe(640);
    });

    it('manages modal dialog visibility states', () => {
      expect(useUIStore.getState().folderModalOpen).toBe(false);
      expect(useUIStore.getState().settingsModalOpen).toBe(false);
      expect(useUIStore.getState().statsModalOpen).toBe(false);

      useUIStore.getState().setFolderModalOpen(true);
      expect(useUIStore.getState().folderModalOpen).toBe(true);

      useUIStore.getState().setSettingsModalOpen(true);
      expect(useUIStore.getState().settingsModalOpen).toBe(true);

      useUIStore.getState().setStatsModalOpen(true);
      expect(useUIStore.getState().statsModalOpen).toBe(true);
    });

    it('updates theme and activeTab', () => {
      useUIStore.getState().setTheme('light');
      expect(useUIStore.getState().theme).toBe('light');

      useUIStore.getState().setActiveTab('folders');
      expect(useUIStore.getState().activeTab).toBe('folders');
    });

    it('tracks active indexing job ID', () => {
      const jobId = '990e8400-e29b-41d4-a716-446655440001';
      useUIStore.getState().setIndexingJobId(jobId);
      expect(useUIStore.getState().indexingJobId).toBe(jobId);
      expect(useUIStore.getState().indexingJobIds).toContain(jobId);

      useUIStore.getState().setIndexingJobId(null);
      expect(useUIStore.getState().indexingJobId).toBeNull();
      expect(useUIStore.getState().indexingJobIds).toHaveLength(0);
    });

    it('tracks multiple concurrent indexing job IDs', () => {
      const id1 = '990e8400-e29b-41d4-a716-446655440001';
      const id2 = '990e8400-e29b-41d4-a716-446655440002';

      useUIStore.getState().addIndexingJobId(id1);
      useUIStore.getState().addIndexingJobId(id2);

      expect(useUIStore.getState().indexingJobIds).toEqual([id1, id2]);
      expect(useUIStore.getState().indexingJobId).toBe(id2);

      useUIStore.getState().removeIndexingJobId(id2);
      expect(useUIStore.getState().indexingJobIds).toEqual([id1]);
      expect(useUIStore.getState().indexingJobId).toBe(id1);

      useUIStore.getState().clearIndexingJobIds();
      expect(useUIStore.getState().indexingJobIds).toHaveLength(0);
      expect(useUIStore.getState().indexingJobId).toBeNull();
    });

    it('tracks and toggles jobDrawerExpanded', () => {
      expect(useUIStore.getState().jobDrawerExpanded).toBe(false);

      useUIStore.getState().toggleJobDrawer();
      expect(useUIStore.getState().jobDrawerExpanded).toBe(true);

      useUIStore.getState().setJobDrawerExpanded(false);
      expect(useUIStore.getState().jobDrawerExpanded).toBe(false);
    });
  });
});
