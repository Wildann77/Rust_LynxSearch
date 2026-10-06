import { z } from 'zod';

export const StatsResponseSchema = z.object({
  total_documents: z.number().int().nonnegative(),
  total_size_bytes: z.number().int().nonnegative(),
  types: z.record(z.string(), z.number().int().nonnegative()).default({}),
  languages: z.record(z.string(), z.number().int().nonnegative()).default({}),
  indexed_folders: z.number().int().nonnegative(),
});
export type StatsResponse = z.infer<typeof StatsResponseSchema>;
