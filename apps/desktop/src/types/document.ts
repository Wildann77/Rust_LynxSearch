import { z } from 'zod';

export const DocumentDetailResponseSchema = z.object({
  id: z.string().uuid(),
  folder_id: z.string().uuid(),
  folder_root_path: z.string(),
  relative_path: z.string(),
  title: z.string(),
  content: z.string(),
  type: z.enum(['doc', 'code', 'config']),
  language: z.string().nullable().optional(),
  tags: z.array(z.string()).default([]),
  project: z.string().nullable().optional(),
  file_size: z.number().int().nonnegative(),
  content_hash: z.string(),
  updated_at: z.string().nullable().optional(),
});
export type DocumentDetailResponse = z.infer<typeof DocumentDetailResponseSchema>;

export const DeleteDocumentResponseSchema = z.object({
  success: z.boolean(),
  id: z.string().uuid(),
  status: z.string(),
  message: z.string(),
});
export type DeleteDocumentResponse = z.infer<typeof DeleteDocumentResponseSchema>;
