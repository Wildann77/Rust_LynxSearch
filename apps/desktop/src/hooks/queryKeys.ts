import type { SearchRequest, SuggestRequest } from '../types/search';

export const queryKeys = {
  health: {
    all: ['health'] as const,
    summary: () => [...queryKeys.health.all, 'summary'] as const,
    live: () => [...queryKeys.health.all, 'live'] as const,
    ready: () => [...queryKeys.health.all, 'ready'] as const,
  },
  folders: {
    all: ['folders'] as const,
    list: () => [...queryKeys.folders.all, 'list'] as const,
  },
  jobs: {
    all: ['jobs'] as const,
    detail: (id: string | null | undefined) => [...queryKeys.jobs.all, id] as const,
  },
  documents: {
    all: ['documents'] as const,
    detail: (id: string | null | undefined) => [...queryKeys.documents.all, id] as const,
  },
  search: {
    all: ['search'] as const,
    list: (params: SearchRequest) => [...queryKeys.search.all, params] as const,
  },
  suggest: {
    all: ['suggest'] as const,
    list: (params: SuggestRequest) => [...queryKeys.suggest.all, params] as const,
  },
  stats: {
    all: ['stats'] as const,
  },
  settings: {
    all: ['settings'] as const,
  },
};
