import { useQuery } from '@tanstack/react-query';
import { fetchHealth, fetchHealthLive, fetchHealthReady } from '../api';
import { queryKeys } from './queryKeys';
import type { HealthSummaryResponse, LivenessResponse, ReadinessResponse } from '../types/health';

export function useHealthQuery(options: { refetchInterval?: number | false } = {}) {
  return useQuery<HealthSummaryResponse>({
    queryKey: queryKeys.health.summary(),
    queryFn: ({ signal }) => fetchHealth(signal),
    refetchInterval: (query) => {
      if (options.refetchInterval === false) return false;
      // When in error/offline, poll aggressively every 2000ms so reconnection is near-instant
      const isOffline = query.state.status === 'error';
      return isOffline ? 2_000 : (options.refetchInterval ?? 10_000);
    },
    refetchOnWindowFocus: true,
    refetchIntervalInBackground: true,
    retry: 1,
  });
}

export function useHealthLiveQuery() {
  return useQuery<LivenessResponse>({
    queryKey: queryKeys.health.live(),
    queryFn: ({ signal }) => fetchHealthLive(signal),
    refetchInterval: 30_000,
  });
}

export function useHealthReadyQuery() {
  return useQuery<ReadinessResponse>({
    queryKey: queryKeys.health.ready(),
    queryFn: ({ signal }) => fetchHealthReady(signal),
    refetchInterval: 15_000,
  });
}
