import { describe, it, expect } from 'vitest';
import {
  ErrorCodeSchema,
  ErrorResponseSchema,
  HealthSummaryResponseSchema,
  LivenessResponseSchema,
  ReadinessResponseSchema,
  FolderResponseSchema,
  RegisterFolderRequestSchema,
  IndexFolderResponseSchema,
  DeleteFolderResponseSchema,
  JobStatusResponseSchema,
  IndexDocumentRequestSchema,
  RebuildIndexResponseSchema,
  DocumentDetailResponseSchema,
  DeleteDocumentResponseSchema,
  SearchResponseSchema,
  SearchResultItemSchema,
  SuggestResponseSchema,
  StatsResponseSchema,
  AppSettingsSchema,
  UpdateSettingsRequestSchema,
} from '../types';

describe('Type Contracts & Zod Schemas', () => {
  describe('Error Contracts', () => {
    it('validates error codes', () => {
      expect(ErrorCodeSchema.safeParse('FOLDER_NOT_FOUND').success).toBe(true);
      expect(ErrorCodeSchema.safeParse('INVALID_ERROR').success).toBe(false);
    });

    it('validates structured error response with details', () => {
      const payload = {
        code: 'VALIDATION_FAILED',
        message: 'Invalid parameters',
        details: { size: ['Must be <= 100'] },
      };
      const result = ErrorResponseSchema.safeParse(payload);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.code).toBe('VALIDATION_FAILED');
      }
    });

    it('rejects unknown error code', () => {
      const payload = {
        code: 'NON_EXISTENT_CODE',
        message: 'Something broke',
      };
      const result = ErrorResponseSchema.safeParse(payload);
      expect(result.success).toBe(false);
    });
  });

  describe('Health Contracts', () => {
    it('validates liveness response', () => {
      const parsed = LivenessResponseSchema.safeParse({ status: 'alive' });
      expect(parsed.success).toBe(true);
    });

    it('validates readiness response', () => {
      const ready = ReadinessResponseSchema.safeParse({ status: 'ready' });
      expect(ready.success).toBe(true);

      const degraded = ReadinessResponseSchema.safeParse({
        status: 'unavailable',
        database: { status: 'down', error: 'Connection refused' },
        elasticsearch: { status: 'up', latency_ms: 5 },
      });
      expect(degraded.success).toBe(true);
    });

    it('validates health summary response', () => {
      const summary = {
        status: 'ok',
        version: '1.0.0',
        timestamp: '2026-10-05T12:00:00Z',
        database: { status: 'up', latency_ms: 3 },
        elasticsearch: { status: 'up', latency_ms: 6 },
      };
      const parsed = HealthSummaryResponseSchema.safeParse(summary);
      expect(parsed.success).toBe(true);
    });
  });

  describe('Folder Contracts', () => {
    it('validates folder response', () => {
      const folder = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        root_path: '/projects/backend',
        status: 'IDLE',
        document_count: 42,
        last_scanned_at: '2026-10-05T10:00:00Z',
        created_at: '2026-10-01T08:00:00Z',
      };
      const parsed = FolderResponseSchema.safeParse(folder);
      expect(parsed.success).toBe(true);
    });

    it('validates register folder request', () => {
      const byPath = RegisterFolderRequestSchema.safeParse({
        root_path: '/projects/notes',
      });
      expect(byPath.success).toBe(true);

      const byId = RegisterFolderRequestSchema.safeParse({
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
      });
      expect(byId.success).toBe(true);
    });

    it('validates index and delete folder response', () => {
      const indexRes = IndexFolderResponseSchema.safeParse({
        job_id: '550e8400-e29b-41d4-a716-446655440001',
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
        status: 'RUNNING',
        message: 'Scan started',
      });
      expect(indexRes.success).toBe(true);

      const delRes = DeleteFolderResponseSchema.safeParse({
        success: true,
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
        deleted_documents: 10,
      });
      expect(delRes.success).toBe(true);
    });
  });

  describe('Job Contracts', () => {
    it('validates job status response', () => {
      const job = {
        job_id: '550e8400-e29b-41d4-a716-446655440002',
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
        status: 'COMPLETED',
        processed_files: 100,
        indexed_files: 85,
        skipped_files: 15,
        failed_files: 0,
        total_files: 100,
        error: null,
        started_at: '2026-10-05T10:00:00Z',
        finished_at: '2026-10-05T10:01:00Z',
      };
      const parsed = JobStatusResponseSchema.safeParse(job);
      expect(parsed.success).toBe(true);
    });

    it('validates rebuild index and single doc indexing', () => {
      const rebuild = RebuildIndexResponseSchema.safeParse({
        job_id: '550e8400-e29b-41d4-a716-446655440003',
        target_index: 'lynxsearch_docs_v2',
        status: 'RUNNING',
        message: 'Rebuild initiated',
      });
      expect(rebuild.success).toBe(true);

      const docReq = IndexDocumentRequestSchema.safeParse({
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
        relative_path: 'src/main.rs',
      });
      expect(docReq.success).toBe(true);
    });
  });

  describe('Document Contracts', () => {
    it('validates document detail response', () => {
      const doc = {
        id: '990e8400-e29b-41d4-a716-446655440002',
        folder_id: '550e8400-e29b-41d4-a716-446655440000',
        folder_root_path: '/projects/backend',
        relative_path: 'src/main.rs',
        title: 'Main Entry',
        content: 'fn main() {}',
        type: 'code',
        language: 'rust',
        tags: ['entry', 'main'],
        project: 'backend',
        file_size: 512,
        content_hash: 'abc123hash',
        updated_at: '2026-10-05T10:00:00Z',
      };
      const parsed = DocumentDetailResponseSchema.safeParse(doc);
      expect(parsed.success).toBe(true);
    });

    it('validates delete document response', () => {
      const del = DeleteDocumentResponseSchema.safeParse({
        success: true,
        id: '990e8400-e29b-41d4-a716-446655440002',
        status: 'EXCLUDED',
        message: 'Document excluded',
      });
      expect(del.success).toBe(true);
    });
  });

  describe('Search & Suggest Contracts', () => {
    it('validates search response with highlights and facets', () => {
      const searchRes = {
        query: 'authenticate_user',
        page: 1,
        size: 20,
        total: 1,
        took_ms: 12,
        items: [
          {
            id: '990e8400-e29b-41d4-a716-446655440002',
            title: 'Auth Handler',
            relative_path: 'src/auth.rs',
            project: 'backend',
            type: 'code',
            language: 'rust',
            tags: ['security'],
            highlights: [
              {
                snippet: 'pub fn <mark>authenticate_user</mark>()',
                line_number: 42,
              },
            ],
            score: 4.5,
            file_size: 1024,
            updated_at: '2026-10-05T10:00:00Z',
          },
        ],
        results: [],
        facets: {
          types: [{ key: 'code', doc_count: 1 }],
          languages: [{ key: 'rust', doc_count: 1 }],
          tags: [{ key: 'security', doc_count: 1 }],
          projects: [{ key: 'backend', doc_count: 1 }],
        },
        warnings: [],
      };
      const parsed = SearchResponseSchema.safeParse(searchRes);
      expect(parsed.success).toBe(true);
      expect(SearchResultItemSchema.safeParse(searchRes.items[0]).success).toBe(true);
    });

    it('validates autocomplete suggest response', () => {
      const suggest = SuggestResponseSchema.safeParse({
        suggestions: ['authenticate', 'authorize', 'auth_service'],
      });
      expect(suggest.success).toBe(true);
    });
  });

  describe('Stats & Settings Contracts', () => {
    it('validates stats response', () => {
      const stats = StatsResponseSchema.safeParse({
        total_documents: 1500,
        total_size_bytes: 4096000,
        types: { code: 1200, doc: 300 },
        languages: { rust: 800, typescript: 400 },
        indexed_folders: 2,
      });
      expect(stats.success).toBe(true);
    });

    it('validates settings and update settings request', () => {
      const settings = AppSettingsSchema.safeParse({
        max_file_size_bytes: 2097152,
        weights: { title: 3.0, tags: 2.0, content: 1.0 },
        ignore_patterns: ['.git', 'node_modules', 'target'],
      });
      expect(settings.success).toBe(true);

      const updateReq = UpdateSettingsRequestSchema.safeParse({
        max_file_size_bytes: 4194304,
        weights: { title: 5.0 },
      });
      expect(updateReq.success).toBe(true);
    });
  });
});
