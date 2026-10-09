import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchSettings, updateSettings, resetSettings } from '../api';
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
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
    },
  });
}

export function useResetSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation<AppSettings, Error, void>({
    mutationFn: () => resetSettings(),
    onSuccess: (resetData) => {
      queryClient.setQueryData(queryKeys.settings.all, resetData);
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
    },
  });
}
