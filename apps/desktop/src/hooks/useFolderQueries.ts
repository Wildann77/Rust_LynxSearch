import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  fetchFolders,
  registerOrRescanFolder,
  deleteFolder,
  indexDocument,
} from '../api';
import { queryKeys } from './queryKeys';
import type {
  FolderResponse,
  IndexFolderResponse,
  DeleteFolderResponse,
  RegisterFolderRequest,
} from '../types/folder';
import type { IndexDocumentRequest, IndexDocumentResponse } from '../types/job';

export function useFoldersQuery() {
  return useQuery<FolderResponse[]>({
    queryKey: queryKeys.folders.list(),
    queryFn: ({ signal }) => fetchFolders(signal),
    staleTime: 10_000,
    refetchInterval: (query) => {
      const folders = query.state.data;
      const isScanning = folders?.some((f) => f.status === 'SCANNING');
      return isScanning ? 1500 : false;
    },
  });
}

export function useRegisterFolderMutation() {
  const queryClient = useQueryClient();

  return useMutation<IndexFolderResponse, Error, RegisterFolderRequest>({
    mutationFn: (data) => registerOrRescanFolder(data),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
    },
  });
}

export function useDeleteFolderMutation() {
  const queryClient = useQueryClient();

  return useMutation<DeleteFolderResponse, Error, string>({
    mutationFn: (folderId) => deleteFolder(folderId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
    },
  });
}

export function useIndexDocumentMutation() {
  const queryClient = useQueryClient();

  return useMutation<IndexDocumentResponse, Error, IndexDocumentRequest>({
    mutationFn: (data) => indexDocument(data),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
    },
  });
}
