import { http, HttpResponse } from 'msw';
import type {
  HealthSummaryResponse,
  LivenessResponse,
  ReadinessResponse,
} from '../../types/health';
import type { StatsResponse } from '../../types/stats';
import type { FolderResponse, IndexFolderResponse, DeleteFolderResponse } from '../../types/folder';
import type { JobStatusResponse, CancelJobResponse, RebuildIndexResponse } from '../../types/job';
import type { DocumentDetailResponse, DeleteDocumentResponse } from '../../types/document';
import type { SearchResponse, SuggestResponse } from '../../types/search';
import type { AppSettings } from '../../types/settings';

export const FOLDER_UUID = 'a0000000-0000-4000-8000-000000000001';
export const JOB_UUID = 'b0000000-0000-4000-8000-000000000002';
export const DOC_UUID = 'c0000000-0000-4000-8000-000000000003';

export const mockHealth: HealthSummaryResponse = {
  status: 'ok',
  version: '0.1.0',
  timestamp: new Date().toISOString(),
  database: {
    status: 'healthy',
    latency_ms: 1,
  },
  elasticsearch: {
    status: 'healthy',
    latency_ms: 2,
  },
};

export const mockLive: LivenessResponse = {
  status: 'ok',
};

export const mockReady: ReadinessResponse = {
  status: 'ready',
  database: {
    status: 'healthy',
    latency_ms: 1,
  },
  elasticsearch: {
    status: 'healthy',
    latency_ms: 2,
  },
};

export const mockStats: StatsResponse = {
  total_documents: 42,
  total_size_bytes: 1048576,
  indexed_folders: 3,
  types: { doc: 10, code: 25, config: 7 },
  languages: { rust: 20, typescript: 5 },
};

export const mockFolders: FolderResponse[] = [
  {
    id: FOLDER_UUID,
    root_path: '/home/user/workspace/repo',
    status: 'IDLE',
    last_scanned_at: new Date().toISOString(),
    created_at: new Date().toISOString(),
    document_count: 35,
  },
];

export const mockJob: JobStatusResponse = {
  job_id: JOB_UUID,
  folder_id: FOLDER_UUID,
  status: 'RUNNING',
  total_files: 100,
  processed_files: 50,
  indexed_files: 45,
  skipped_files: 5,
  failed_files: 0,
  error: null,
  started_at: new Date().toISOString(),
  finished_at: null,
};

export const mockDocument: DocumentDetailResponse = {
  id: DOC_UUID,
  folder_id: FOLDER_UUID,
  folder_root_path: '/home/user/workspace/repo',
  relative_path: 'src/main.rs',
  title: 'Main Entrypoint',
  content: 'fn main() {\n    println!("Hello MSW");\n}\n',
  tags: ['rust', 'entrypoint'],
  type: 'code',
  language: 'rust',
  project: 'lynxsearch',
  file_size: 48,
  content_hash: 'mock_sha256_hash',
  updated_at: new Date().toISOString(),
};

export const mockSettings: AppSettings = {
  max_file_size_bytes: 2097152,
  weights: {
    title: 3.0,
    tags: 2.0,
    content: 1.0,
  },
  ignore_patterns: ['.git', 'node_modules', 'target'],
};

const sampleResultItem = {
  id: DOC_UUID,
  title: 'Main Entrypoint',
  relative_path: 'src/main.rs',
  type: 'code' as const,
  language: 'rust',
  project: 'lynxsearch',
  tags: ['rust', 'entrypoint'],
  file_size: 48,
  updated_at: new Date().toISOString(),
  score: 1.5,
  highlights: [
    {
      snippet: 'fn <em>main</em>() {',
      line_number: 1,
    },
  ],
};

export const mockSearchSuccess: SearchResponse = {
  query: 'main',
  page: 1,
  size: 20,
  total: 1,
  took_ms: 5,
  items: [sampleResultItem],
  results: [sampleResultItem],
  warnings: [],
  facets: {
    extensions: [{ key: 'rs', doc_count: 1 }],
    languages: [{ key: 'rust', doc_count: 1 }],
    types: [{ key: 'code', doc_count: 1 }],
    projects: [{ key: 'lynxsearch', doc_count: 1 }],
    tags: [
      { key: 'rust', doc_count: 1 },
      { key: 'entrypoint', doc_count: 1 },
    ],
  },
};

export const mockSearchEmpty: SearchResponse = {
  query: '',
  page: 1,
  size: 20,
  total: 0,
  took_ms: 1,
  items: [],
  results: [],
  warnings: [],
  facets: {
    extensions: [],
    languages: [],
    types: [],
    projects: [],
    tags: [],
  },
};

export const mockSuggest: SuggestResponse = {
  suggestions: ['main', 'main.rs', 'maintenance'],
};

export const handlers = [
  // Health
  http.get('*/api/health', () => HttpResponse.json(mockHealth)),
  http.get('*/api/health/live', () => HttpResponse.json(mockLive)),
  http.get('*/api/health/ready', () => HttpResponse.json(mockReady)),

  // Stats
  http.get('*/api/stats', () => HttpResponse.json(mockStats)),

  // Folders
  http.get('*/api/folders', () => HttpResponse.json(mockFolders)),
  http.post('*/api/index/folder', async () => {
    const res: IndexFolderResponse = {
      job_id: mockJob.job_id,
      folder_id: mockFolders[0].id,
      status: 'RUNNING',
      message: 'Background scanning job created successfully.',
    };
    return HttpResponse.json(res, { status: 202 });
  }),
  http.delete('*/api/folders/:id', ({ params }) => {
    const res: DeleteFolderResponse = {
      folder_id: String(params.id),
      success: true,
      deleted_documents: 15,
    };
    return HttpResponse.json(res);
  }),

  // Jobs
  http.get('*/api/index/jobs/:id', () => HttpResponse.json(mockJob)),
  http.post('*/api/index/jobs/:id/cancel', ({ params }) => {
    const res: CancelJobResponse = {
      job_id: String(params.id),
      status: 'CANCELLED',
      message: 'Job cancelled successfully.',
    };
    return HttpResponse.json(res);
  }),
  http.post('*/api/index/rebuild', () => {
    const res: RebuildIndexResponse = {
      job_id: mockJob.job_id,
      target_index: 'lynx_documents_v2',
      status: 'RUNNING',
      message: 'Index rebuild started successfully.',
    };
    return HttpResponse.json(res, { status: 202 });
  }),

  // Search & Suggest
  http.get('*/api/search', ({ request }) => {
    const url = new URL(request.url);
    const q = url.searchParams.get('q');

    if (q === 'error_500') {
      return HttpResponse.json(
        {
          code: 'INTERNAL_SERVER_ERROR',
          message: 'Simulated 500 search engine failure',
        },
        { status: 500 },
      );
    }

    if (!q || q.trim() === '' || q === 'empty') {
      return HttpResponse.json(mockSearchEmpty);
    }

    return HttpResponse.json(mockSearchSuccess);
  }),

  http.get('*/api/suggest', () => HttpResponse.json(mockSuggest)),

  // Documents
  http.get('*/api/documents/:id', () => HttpResponse.json(mockDocument)),
  http.delete('*/api/documents/:id', ({ params }) => {
    const res: DeleteDocumentResponse = {
      id: String(params.id),
      success: true,
      status: 'DELETED',
      message: 'Document deleted successfully.',
    };
    return HttpResponse.json(res);
  }),

  // Settings
  http.get('*/api/settings', () => HttpResponse.json(mockSettings)),
  http.put('*/api/settings', async ({ request }) => {
    const body = (await request.json()) as Partial<AppSettings>;
    return HttpResponse.json({ ...mockSettings, ...body });
  }),
  http.post('*/api/settings/reset', () => HttpResponse.json(mockSettings)),
];
