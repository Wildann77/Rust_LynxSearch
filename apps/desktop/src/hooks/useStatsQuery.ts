import { useQuery } from '@tanstack/react-query';
import { fetchStats } from '../api';
import { queryKeys } from './queryKeys';
import type { StatsResponse } from '../types/stats';

export function useStatsQuery() {
  return useQuery<StatsResponse>({
    queryKey: queryKeys.stats.all,
    queryFn: ({ signal }) => fetchStats(signal),
    staleTime: 30_000,
  });
}
