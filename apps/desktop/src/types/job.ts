import { z } from 'zod';

export const JobStatusSchema = z.enum([
  'PENDING',
  'RUNNING',
  'COMPLETED',
  'FAILED',
  'CANCELLED',
]);
export type JobStatus = z.infer<typeof JobStatusSchema>;

export const IndexDocumentRequestSchema = z.object({
  folder_id: z.string().uuid(),
  relative_path: z.string().min(1),
});
export type IndexDocumentRequest = z.infer<typeof IndexDocumentRequestSchema>;

export const IndexDocumentResponseSchema = z.object({
  document_id: z.string().uuid(),
  status: z.string(),
  message: z.string(),
});
export type IndexDocumentResponse = z.infer<typeof IndexDocumentResponseSchema>;

export const RebuildIndexResponseSchema = z.object({
  job_id: z.string().uuid(),
  target_index: z.string(),
  status: z.string(),
  message: z.string(),
});
export type RebuildIndexResponse = z.infer<typeof RebuildIndexResponseSchema>;

export const JobStatusResponseSchema = z.object({
  job_id: z.string().uuid(),
  folder_id: z.string().uuid().nullable().optional(),
  status: z.string(),
  processed_files: z.number().int(),
  indexed_files: z.number().int(),
  skipped_files: z.number().int(),
  failed_files: z.number().int(),
  total_files: z.number().int(),
  error: z.string().nullable().optional(),
  started_at: z.string(),
  finished_at: z.string().nullable().optional(),
});
export type JobStatusResponse = z.infer<typeof JobStatusResponseSchema>;
