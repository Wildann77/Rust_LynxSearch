import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchDocument, deleteDocument } from '../api';
import { queryKeys } from './queryKeys';
import type { DocumentDetailResponse, DeleteDocumentResponse } from '../types/document';

export function useDocumentDetailQuery(documentId: string | null | undefined) {
  return useQuery<DocumentDetailResponse>({
    queryKey: queryKeys.documents.detail(documentId),
    queryFn: ({ signal }) => {
      if (!documentId) throw new Error('Document ID is required');
      return fetchDocument(documentId, signal);
    },
    enabled: Boolean(documentId),
    staleTime: 60_000,
  });
}

export function useDeleteDocumentMutation() {
  const queryClient = useQueryClient();

  return useMutation<DeleteDocumentResponse, Error, string>({
    mutationFn: (documentId) => deleteDocument(documentId),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.documents.detail(data.id) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.search.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.stats.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.folders.all });
    },
  });
}
