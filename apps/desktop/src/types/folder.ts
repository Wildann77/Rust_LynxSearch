import { z } from 'zod';

export const FolderStatusSchema = z.enum(['IDLE', 'SCANNING', 'ERROR']);
export type FolderStatus = z.infer<typeof FolderStatusSchema>;

export const RegisterFolderRequestSchema = z.object({
  folder_id: z.string().uuid().optional(),
  root_path: z.string().min(1).max(4096).optional(),
});
export type RegisterFolderRequest = z.infer<typeof RegisterFolderRequestSchema>;

export const FolderResponseSchema = z.object({
  id: z.string().uuid(),
  root_path: z.string(),
  status: FolderStatusSchema,
  document_count: z.number().int().nonnegative(),
  last_scanned_at: z.string().nullable().optional(),
  created_at: z.string(),
});
export type FolderResponse = z.infer<typeof FolderResponseSchema>;

export const IndexFolderResponseSchema = z.object({
  job_id: z.string().uuid(),
  folder_id: z.string().uuid(),
  status: z.string(),
  message: z.string(),
});
export type IndexFolderResponse = z.infer<typeof IndexFolderResponseSchema>;

export const DeleteFolderResponseSchema = z.object({
  success: z.boolean(),
  folder_id: z.string().uuid(),
  deleted_documents: z.number().int().nonnegative(),
});
export type DeleteFolderResponse = z.infer<typeof DeleteFolderResponseSchema>;
