import { z } from 'zod';

export const ErrorCodeSchema = z.enum([
  'FOLDER_NOT_FOUND',
  'DOCUMENT_NOT_FOUND',
  'JOB_NOT_FOUND',
  'JOB_CONFLICT',
  'VALIDATION_FAILED',
  'PATH_TRAVERSAL_DETECTED',
  'INVALID_QUERY',
  'DATABASE_UNAVAILABLE',
  'SEARCH_ENGINE_UNAVAILABLE',
  'IO_ERROR',
  'INTERNAL_SERVER_ERROR',
  'REQUEST_TIMEOUT',
]);
export type ErrorCode = z.infer<typeof ErrorCodeSchema>;

export const ErrorResponseSchema = z.object({
  code: ErrorCodeSchema,
  message: z.string(),
  details: z.unknown().nullable().optional(),
});
export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;
