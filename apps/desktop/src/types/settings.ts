import { z } from 'zod';

export const Bm25WeightsSchema = z.object({
  title: z.number().min(0.1).max(100.0),
  tags: z.number().min(0.1).max(100.0),
  content: z.number().min(0.1).max(100.0),
});
export type Bm25Weights = z.infer<typeof Bm25WeightsSchema>;

export const AppSettingsSchema = z.object({
  max_file_size_bytes: z.number().int().positive(),
  weights: Bm25WeightsSchema,
  ignore_patterns: z.array(z.string()).default([]),
});
export type AppSettings = z.infer<typeof AppSettingsSchema>;

export const UpdateBm25WeightsSchema = z.object({
  title: z.number().min(0.1).max(100.0).optional(),
  tags: z.number().min(0.1).max(100.0).optional(),
  content: z.number().min(0.1).max(100.0).optional(),
});
export type UpdateBm25Weights = z.infer<typeof UpdateBm25WeightsSchema>;

export const UpdateSettingsRequestSchema = z.object({
  max_file_size_bytes: z.number().int().min(1024).max(104857600).optional(),
  weights: UpdateBm25WeightsSchema.optional(),
  ignore_patterns: z.array(z.string()).optional(),
});
export type UpdateSettingsRequest = z.infer<typeof UpdateSettingsRequestSchema>;

export const DEFAULT_SETTINGS: AppSettings = {
  max_file_size_bytes: 2 * 1024 * 1024,
  weights: {
    title: 3.0,
    tags: 2.0,
    content: 1.0,
  },
  ignore_patterns: ['.git', 'node_modules', 'target', 'dist', 'build'],
};

