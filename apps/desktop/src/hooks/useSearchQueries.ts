import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { searchDocuments, suggestQueries } from '../api';
import { queryKeys } from './queryKeys';
import type { SearchRequest, SearchResponse, SuggestRequest, SuggestResponse } from '../types/search';

export function useSearchQuery(
  params: SearchRequest,
  options: { enabled?: boolean } = {},
) {
  return useQuery<SearchResponse>({
    queryKey: queryKeys.search.list(params),
    queryFn: ({ signal }) => searchDocuments(params, signal),
    enabled:
      options.enabled ??
      Boolean(
        params.q?.trim() ||
          params.extension ||
          params.type ||
          params.language ||
          params.tag ||
          params.project,
      ),
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  });
}

export function useSuggestQuery(
  params: SuggestRequest,
  options: { enabled?: boolean } = {},
) {
  return useQuery<SuggestResponse>({
    queryKey: queryKeys.suggest.list(params),
    queryFn: ({ signal }) => suggestQueries(params, signal),
    enabled: options.enabled ?? Boolean(params.q?.trim() && params.q.trim().length >= 1),
    staleTime: 60_000,
  });
}
