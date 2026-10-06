import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchSettings, updateSettings } from '../api';
import { queryKeys } from './queryKeys';
import type { AppSettings, UpdateSettingsRequest } from '../types/settings';

export function useSettingsQuery() {
  return useQuery<AppSettings>({
    queryKey: queryKeys.settings.all,
    queryFn: ({ signal }) => fetchSettings(signal),
    staleTime: 60_000,
  });
}

export function useUpdateSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation<AppSettings, Error, UpdateSettingsRequest>({
    mutationFn: (data) => updateSettings(data),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.settings.all, updated);
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings.all });
    },
  });
}
