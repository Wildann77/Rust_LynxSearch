import { create } from 'zustand';
import {
  parseQueryFilters,
  setQueryFilter,
  toggleQueryFilter,
  removeQueryFilter,
  clearAllQueryFilters,
  type FilterKey,
} from '../lib/querySync';
import type { SortOption } from '../types/search';

export interface ActiveFilters {
  extension?: string;
  type?: string;
  language?: string;
  tag?: string;
  project?: string;
}

export interface SearchState {
  rawQuery: string;
  activeFilters: ActiveFilters;
  page: number;
  pageSize: number;
  sort: SortOption;
  selectedDocId: string | null;
  selectedLineNumber: number | null;

  setRawQuery: (rawQuery: string) => void;
  setFilter: (key: keyof ActiveFilters, value?: string | null) => void;
  toggleFilterValue: (key: keyof ActiveFilters, value: string) => void;
  removeFilterValue: (key: keyof ActiveFilters, value: string) => void;
  clearFilters: () => void;
  setSort: (sort: SortOption) => void;
  setPage: (page: number) => void;
  setPageSize: (pageSize: number) => void;
  setSelectedDocId: (selectedDocId: string | null) => void;
  setSelectedLineNumber: (selectedLineNumber: number | null) => void;
  resetSearch: () => void;
}

const initialState = {
  rawQuery: '',
  activeFilters: {},
  page: 1,
  pageSize: 20,
  sort: 'relevance' as SortOption,
  selectedDocId: null,
  selectedLineNumber: null,
};

export const useSearchStore = create<SearchState>((set) => ({
  ...initialState,

  setRawQuery: (rawQuery) =>
    set(() => {
      const { activeFilters } = parseQueryFilters(rawQuery);
      return {
        rawQuery,
        activeFilters,
        page: 1,
      };
    }),

  setFilter: (key, value) =>
    set((state) => {
      const { newRawQuery, newActiveFilters } = setQueryFilter(
        state.rawQuery,
        key as FilterKey,
        value,
        state.activeFilters,
      );
      return {
        rawQuery: newRawQuery,
        activeFilters: newActiveFilters,
        page: 1,
      };
    }),

  toggleFilterValue: (key, value) =>
    set((state) => {
      const { newRawQuery, newActiveFilters } = toggleQueryFilter(
        state.rawQuery,
        key as FilterKey,
        value,
        state.activeFilters,
      );
      return {
        rawQuery: newRawQuery,
        activeFilters: newActiveFilters,
        page: 1,
      };
    }),

  removeFilterValue: (key, value) =>
    set((state) => {
      const { newRawQuery, newActiveFilters } = removeQueryFilter(
        state.rawQuery,
        key as FilterKey,
        value,
        state.activeFilters,
      );
      return {
        rawQuery: newRawQuery,
        activeFilters: newActiveFilters,
        page: 1,
      };
    }),

  clearFilters: () =>
    set((state) => {
      const { newRawQuery, newActiveFilters } = clearAllQueryFilters(state.rawQuery);
      return {
        rawQuery: newRawQuery,
        activeFilters: newActiveFilters,
        page: 1,
      };
    }),

  setSort: (sort) => set({ sort, page: 1 }),

  setPage: (page) => set({ page }),

  setPageSize: (pageSize) => set({ pageSize, page: 1 }),

  setSelectedDocId: (selectedDocId) => set({ selectedDocId }),
  setSelectedLineNumber: (selectedLineNumber) => set({ selectedLineNumber }),

  resetSearch: () => set(initialState),
}));

