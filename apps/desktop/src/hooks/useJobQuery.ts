import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchJob, cancelJob, rebuildIndex } from '../api';
import { queryKeys } from './queryKeys';
import type { JobStatusResponse, CancelJobResponse, RebuildIndexResponse } from '../types/job';

export function useJobQuery(jobId: string | null | undefined) {
  return useQuery<JobStatusResponse>({
    queryKey: queryKeys.jobs.detail(jobId),
    queryFn: ({ signal }) => {
      if (!jobId) throw new Error('Job ID is required');
      return fetchJob(jobId, signal);
    },
    enabled: Boolean(jobId),
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      if (status === 'RUNNING' || status === 'PENDING') {
        return 1000;
      }
      return false;
    },
  });
}

export function useCancelJobMutation() {
  const queryClient = useQueryClient();

  return useMutation<CancelJobResponse, Error, string>({
    mutationFn: (jobId) => cancelJob(jobId),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.jobs.detail(data.job_id) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
    },
  });
}

export function useRebuildIndexMutation() {
  const queryClient = useQueryClient();

  return useMutation<RebuildIndexResponse, Error, void>({
    mutationFn: () => rebuildIndex(),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.jobs.detail(data.job_id) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
    },
  });
}
