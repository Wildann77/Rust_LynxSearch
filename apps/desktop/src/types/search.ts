import { z } from 'zod';

export const DocumentTypeSchema = z.enum(['doc', 'code', 'config']);
export type DocumentType = z.infer<typeof DocumentTypeSchema>;

export const SearchHighlightSchema = z.object({
  snippet: z.string(),
  line_number: z.number().int().positive().nullable().optional(),
});
export type SearchHighlight = z.infer<typeof SearchHighlightSchema>;

export const SearchResultItemSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  relative_path: z.string(),
  project: z.string().nullable().optional(),
  type: DocumentTypeSchema,
  language: z.string().nullable().optional(),
  tags: z.array(z.string()).default([]),
  highlights: z.array(SearchHighlightSchema).default([]),
  score: z.number(),
  file_size: z.number().int().nonnegative(),
  updated_at: z.string().nullable().optional(),
});
export type SearchResultItem = z.infer<typeof SearchResultItemSchema>;

export const FacetBucketSchema = z.object({
  key: z.string(),
  doc_count: z.number().int().nonnegative(),
});
export type FacetBucket = z.infer<typeof FacetBucketSchema>;

export const SearchFacetsSchema = z.object({
  types: z.array(FacetBucketSchema).default([]),
  languages: z.array(FacetBucketSchema).default([]),
  tags: z.array(FacetBucketSchema).default([]),
  projects: z.array(FacetBucketSchema).default([]),
});
export type SearchFacets = z.infer<typeof SearchFacetsSchema>;

export const SearchRequestSchema = z.object({
  q: z.string().optional(),
  page: z.number().int().min(1).max(1000).optional(),
  size: z.number().int().min(1).max(100).optional(),
  type: z.string().optional(),
  language: z.string().optional(),
  tag: z.string().optional(),
  project: z.string().optional(),
  sort: z.string().optional(),
});
export type SearchRequest = z.infer<typeof SearchRequestSchema>;

export const SearchResponseSchema = z.object({
  query: z.string(),
  page: z.number().int().nonnegative(),
  size: z.number().int().nonnegative(),
  total: z.number().int().nonnegative(),
  took_ms: z.number().int().nonnegative(),
  items: z.array(SearchResultItemSchema).default([]),
  results: z.array(SearchResultItemSchema).default([]),
  facets: SearchFacetsSchema.default({
    types: [],
    languages: [],
    tags: [],
    projects: [],
  }),
  warnings: z.array(z.string()).default([]),
});
export type SearchResponse = z.infer<typeof SearchResponseSchema>;

export const SuggestRequestSchema = z.object({
  q: z.string().min(1).max(500),
  limit: z.number().int().min(1).max(50).optional(),
});
export type SuggestRequest = z.infer<typeof SuggestRequestSchema>;

export const SuggestResponseSchema = z.object({
  suggestions: z.array(z.string()).default([]),
});
export type SuggestResponse = z.infer<typeof SuggestResponseSchema>;
