import { useQuery } from '@tanstack/react-query';
import { fetchHealth, fetchHealthLive, fetchHealthReady } from '../api';
import { queryKeys } from './queryKeys';
import type { HealthSummaryResponse, LivenessResponse, ReadinessResponse } from '../types/health';

export function useHealthQuery(options: { refetchInterval?: number | false } = {}) {
  return useQuery<HealthSummaryResponse>({
    queryKey: queryKeys.health.summary(),
    queryFn: ({ signal }) => fetchHealth(signal),
    refetchInterval: options.refetchInterval ?? 10_000,
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
