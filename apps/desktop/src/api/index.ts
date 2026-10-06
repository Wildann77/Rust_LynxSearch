import { z } from 'zod';
import { apiClient } from '../lib/api-client';
import {
  HealthSummaryResponseSchema,
  LivenessResponseSchema,
  ReadinessResponseSchema,
  type HealthSummaryResponse,
  type LivenessResponse,
  type ReadinessResponse,
} from '../types/health';
import {
  StatsResponseSchema,
  type StatsResponse,
} from '../types/stats';
import {
  FolderResponseSchema,
  IndexFolderResponseSchema,
  DeleteFolderResponseSchema,
  type FolderResponse,
  type IndexFolderResponse,
  type DeleteFolderResponse,
  type RegisterFolderRequest,
} from '../types/folder';
import {
  JobStatusResponseSchema,
  RebuildIndexResponseSchema,
  IndexDocumentResponseSchema,
  type JobStatusResponse,
  type RebuildIndexResponse,
  type IndexDocumentResponse,
  type IndexDocumentRequest,
} from '../types/job';
import {
  DocumentDetailResponseSchema,
  DeleteDocumentResponseSchema,
  type DocumentDetailResponse,
  type DeleteDocumentResponse,
} from '../types/document';
import {
  SearchResponseSchema,
  SuggestResponseSchema,
  type SearchRequest,
  type SearchResponse,
  type SuggestRequest,
  type SuggestResponse,
} from '../types/search';
import {
  AppSettingsSchema,
  type AppSettings,
  type UpdateSettingsRequest,
} from '../types/settings';

export * from './config';
export * from '../lib/api-client';
export * from '../types';

export async function fetchHealth(signal?: AbortSignal): Promise<HealthSummaryResponse> {
  return apiClient.get('/api/health', {
    schema: HealthSummaryResponseSchema,
    signal,
  });
}

export async function fetchHealthLive(signal?: AbortSignal): Promise<LivenessResponse> {
  return apiClient.get('/api/health/live', {
    schema: LivenessResponseSchema,
    signal,
  });
}

export async function fetchHealthReady(signal?: AbortSignal): Promise<ReadinessResponse> {
  return apiClient.get('/api/health/ready', {
    schema: ReadinessResponseSchema,
    signal,
  });
}

export async function fetchStats(signal?: AbortSignal): Promise<StatsResponse> {
  return apiClient.get('/api/stats', {
    schema: StatsResponseSchema,
    signal,
  });
}

export async function fetchFolders(signal?: AbortSignal): Promise<FolderResponse[]> {
  return apiClient.get('/api/folders', {
    schema: z.array(FolderResponseSchema),
    signal,
  });
}

export async function registerOrRescanFolder(
  data: RegisterFolderRequest,
  signal?: AbortSignal,
): Promise<IndexFolderResponse> {
  return apiClient.post('/api/index/folder', data, {
    schema: IndexFolderResponseSchema,
    signal,
  });
}

export async function deleteFolder(
  folderId: string,
  signal?: AbortSignal,
): Promise<DeleteFolderResponse> {
  return apiClient.delete(`/api/folders/${folderId}`, {
    schema: DeleteFolderResponseSchema,
    signal,
  });
}

export async function indexDocument(
  data: IndexDocumentRequest,
  signal?: AbortSignal,
): Promise<IndexDocumentResponse> {
  return apiClient.post('/api/index', data, {
    schema: IndexDocumentResponseSchema,
    signal,
  });
}

export async function fetchJob(
  jobId: string,
  signal?: AbortSignal,
): Promise<JobStatusResponse> {
  return apiClient.get(`/api/index/jobs/${jobId}`, {
    schema: JobStatusResponseSchema,
    signal,
  });
}

export async function cancelJob(
  jobId: string,
  signal?: AbortSignal,
): Promise<JobStatusResponse> {
  return apiClient.post(`/api/index/jobs/${jobId}/cancel`, undefined, {
    schema: JobStatusResponseSchema,
    signal,
  });
}

export async function rebuildIndex(signal?: AbortSignal): Promise<RebuildIndexResponse> {
  return apiClient.post('/api/index/rebuild', undefined, {
    schema: RebuildIndexResponseSchema,
    signal,
  });
}

export async function fetchDocument(
  documentId: string,
  signal?: AbortSignal,
): Promise<DocumentDetailResponse> {
  return apiClient.get(`/api/documents/${documentId}`, {
    schema: DocumentDetailResponseSchema,
    signal,
  });
}

export async function deleteDocument(
  documentId: string,
  signal?: AbortSignal,
): Promise<DeleteDocumentResponse> {
  return apiClient.delete(`/api/documents/${documentId}`, {
    schema: DeleteDocumentResponseSchema,
    signal,
  });
}

export async function searchDocuments(
  params: SearchRequest,
  signal?: AbortSignal,
): Promise<SearchResponse> {
  return apiClient.get('/api/search', {
    params: {
      q: params.q,
      page: params.page,
      size: params.size,
      type: params.type,
      language: params.language,
      tag: params.tag,
      project: params.project,
      sort: params.sort,
    },
    schema: SearchResponseSchema,
    signal,
  });
}

export async function suggestQueries(
  params: SuggestRequest,
  signal?: AbortSignal,
): Promise<SuggestResponse> {
  return apiClient.get('/api/suggest', {
    params: {
      q: params.q,
      limit: params.limit,
    },
    schema: SuggestResponseSchema,
    signal,
  });
}

export async function fetchSettings(signal?: AbortSignal): Promise<AppSettings> {
  return apiClient.get('/api/settings', {
    schema: AppSettingsSchema,
    signal,
  });
}

export async function updateSettings(
  data: UpdateSettingsRequest,
  signal?: AbortSignal,
): Promise<AppSettings> {
  return apiClient.put('/api/settings', data, {
    schema: AppSettingsSchema,
    signal,
  });
}
