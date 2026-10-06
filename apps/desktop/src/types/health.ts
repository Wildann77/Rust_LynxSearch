import { z } from 'zod';

export const ComponentHealthSchema = z.object({
  status: z.string(),
  latency_ms: z.number().nullable().optional(),
  error: z.string().nullable().optional(),
});
export type ComponentHealth = z.infer<typeof ComponentHealthSchema>;

export const LivenessResponseSchema = z.object({
  status: z.string(),
});
export type LivenessResponse = z.infer<typeof LivenessResponseSchema>;

export const ReadinessResponseSchema = z.object({
  status: z.string(),
  database: ComponentHealthSchema.nullable().optional(),
  elasticsearch: ComponentHealthSchema.nullable().optional(),
});
export type ReadinessResponse = z.infer<typeof ReadinessResponseSchema>;

export const HealthSummaryResponseSchema = z.object({
  status: z.string(),
  version: z.string(),
  timestamp: z.string(),
  database: ComponentHealthSchema,
  elasticsearch: ComponentHealthSchema,
});
export type HealthSummaryResponse = z.infer<typeof HealthSummaryResponseSchema>;
