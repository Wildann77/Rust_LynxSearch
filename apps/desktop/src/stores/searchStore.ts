import { create } from 'zustand';

export interface ActiveFilters {
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
  selectedDocId: string | null;

  setRawQuery: (rawQuery: string) => void;
  setFilter: (key: keyof ActiveFilters, value?: string | null) => void;
  clearFilters: () => void;
  setPage: (page: number) => void;
  setPageSize: (pageSize: number) => void;
  setSelectedDocId: (selectedDocId: string | null) => void;
  resetSearch: () => void;
}

const initialState = {
  rawQuery: '',
  activeFilters: {},
  page: 1,
  pageSize: 20,
  selectedDocId: null,
};

export const useSearchStore = create<SearchState>((set) => ({
  ...initialState,

  setRawQuery: (rawQuery) =>
    set({
      rawQuery,
      page: 1, // Reset ke halaman pertama setiap kali input query berubah
    }),

  setFilter: (key, value) =>
    set((state) => {
      const updated = { ...state.activeFilters };
      if (!value) {
        delete updated[key];
      } else {
        updated[key] = value;
      }
      return { activeFilters: updated, page: 1 };
    }),

  clearFilters: () => set({ activeFilters: {}, page: 1 }),

  setPage: (page) => set({ page }),

  setPageSize: (pageSize) => set({ pageSize, page: 1 }),

  setSelectedDocId: (selectedDocId) => set({ selectedDocId }),

  resetSearch: () => set(initialState),
}));
