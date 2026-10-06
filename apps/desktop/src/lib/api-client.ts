import { z } from 'zod';
import { BACKEND_URL } from '../api/config';
import { ErrorResponseSchema, type ErrorCode } from '../types/error';

export class ApiClientError extends Error {
  readonly status: number;
  readonly code: ErrorCode | 'UNKNOWN_ERROR';
  readonly details?: unknown;

  constructor(
    status: number,
    message: string,
    code: ErrorCode | 'UNKNOWN_ERROR' = 'UNKNOWN_ERROR',
    details?: unknown,
  ) {
    super(message);
    this.name = 'ApiClientError';
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

export class ApiTimeoutError extends Error {
  readonly timeoutMs: number;

  constructor(timeoutMs: number, url?: string) {
    super(`Request to ${url ?? 'server'} timed out after ${timeoutMs}ms`);
    this.name = 'ApiTimeoutError';
    this.timeoutMs = timeoutMs;
  }
}

export class ApiValidationError extends Error {
  readonly issues: z.ZodIssue[];
  readonly rawData: unknown;

  constructor(message: string, issues: z.ZodIssue[], rawData: unknown) {
    super(message);
    this.name = 'ApiValidationError';
    this.issues = issues;
    this.rawData = rawData;
  }
}

export interface ApiClientConfig {
  baseUrl?: string;
  defaultTimeoutMs?: number;
  headers?: Record<string, string>;
}

export interface RequestOptions<T = unknown> extends Omit<RequestInit, 'body'> {
  body?: unknown;
  schema?: z.ZodType<T>;
  timeoutMs?: number;
  params?: Record<string, string | number | boolean | undefined | null>;
}

export function createTimeoutSignal(
  timeoutMs: number,
  externalSignal?: AbortSignal | null,
) {
  const timeoutController = new AbortController();
  let timedOut = false;
  const timeoutId = setTimeout(() => {
    timedOut = true;
    timeoutController.abort(new Error(`Request timed out after ${timeoutMs}ms`));
  }, timeoutMs);

  let combinedSignal: AbortSignal;
  let cleanup = () => {
    clearTimeout(timeoutId);
  };

  if (!externalSignal) {
    combinedSignal = timeoutController.signal;
  } else if (typeof AbortSignal.any === 'function') {
    combinedSignal = AbortSignal.any([timeoutController.signal, externalSignal]);
  } else {
    const combinedController = new AbortController();
    const onAbort = () => {
      combinedController.abort(externalSignal.reason);
    };
    const onTimeout = () => {
      combinedController.abort(timeoutController.signal.reason);
    };

    if (externalSignal.aborted) {
      combinedController.abort(externalSignal.reason);
    } else {
      externalSignal.addEventListener('abort', onAbort, { once: true });
      timeoutController.signal.addEventListener('abort', onTimeout, { once: true });
      const origCleanup = cleanup;
      cleanup = () => {
        origCleanup();
        externalSignal.removeEventListener('abort', onAbort);
        timeoutController.signal.removeEventListener('abort', onTimeout);
      };
    }
    combinedSignal = combinedController.signal;
  }

  return {
    signal: combinedSignal,
    cleanup,
    isTimeout: () => timedOut,
  };
}

export class ApiClient {
  private baseUrl: string;
  private defaultTimeoutMs: number;
  private defaultHeaders: Record<string, string>;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = (config.baseUrl || BACKEND_URL).replace(/\/+$/, '');
    this.defaultTimeoutMs = config.defaultTimeoutMs ?? 10000;
    this.defaultHeaders = {
      Accept: 'application/json',
      ...config.headers,
    };
  }

  getBaseUrl(): string {
    return this.baseUrl;
  }

  setBaseUrl(url: string): void {
    this.baseUrl = url.replace(/\/+$/, '');
  }

  async request<T>(endpoint: string, options: RequestOptions<T> = {}): Promise<T> {
    const {
      body,
      schema,
      timeoutMs = this.defaultTimeoutMs,
      params,
      headers: customHeaders,
      signal: externalSignal,
      ...fetchInit
    } = options;

    let url = endpoint.startsWith('http://') || endpoint.startsWith('https://')
      ? endpoint
      : `${this.baseUrl}${endpoint.startsWith('/') ? '' : '/'}${endpoint}`;

    if (params) {
      const searchParams = new URLSearchParams();
      for (const [key, val] of Object.entries(params)) {
        if (val !== undefined && val !== null) {
          searchParams.append(key, String(val));
        }
      }
      const qs = searchParams.toString();
      if (qs) {
        url += (url.includes('?') ? '&' : '?') + qs;
      }
    }

    const headers: Record<string, string> = {
      ...this.defaultHeaders,
      ...(customHeaders as Record<string, string>),
    };

    let serializedBody: BodyInit | undefined;
    if (body !== undefined && body !== null) {
      if (
        typeof body === 'string' ||
        body instanceof Blob ||
        body instanceof FormData ||
        body instanceof URLSearchParams
      ) {
        serializedBody = body;
      } else {
        serializedBody = JSON.stringify(body);
        if (!headers['Content-Type']) {
          headers['Content-Type'] = 'application/json';
        }
      }
    }

    const { signal, cleanup, isTimeout } = createTimeoutSignal(timeoutMs, externalSignal);

    try {
      const response = await fetch(url, {
        ...fetchInit,
        headers,
        body: serializedBody,
        signal,
      });

      if (!response.ok) {
        let errorData: unknown;
        try {
          errorData = await response.json();
        } catch {
          errorData = null;
        }

        const parsedError = ErrorResponseSchema.safeParse(errorData);
        if (parsedError.success) {
          throw new ApiClientError(
            response.status,
            parsedError.data.message,
            parsedError.data.code,
            parsedError.data.details,
          );
        }

        const fallbackText = typeof errorData === 'string' ? errorData : response.statusText;
        throw new ApiClientError(
          response.status,
          fallbackText || `HTTP request failed with status ${response.status}`,
          'UNKNOWN_ERROR',
          errorData,
        );
      }

      if (response.status === 204) {
        return undefined as T;
      }

      const responseData = (await response.json()) as unknown;

      if (schema) {
        const parseResult = schema.safeParse(responseData);
        if (!parseResult.success) {
          throw new ApiValidationError(
            `Schema validation failed for endpoint ${endpoint}: ${parseResult.error.message}`,
            parseResult.error.issues,
            responseData,
          );
        }
        return parseResult.data;
      }

      return responseData as T;
    } catch (err: unknown) {
      if (isTimeout()) {
        throw new ApiTimeoutError(timeoutMs, url);
      }
      if (externalSignal?.aborted) {
        throw externalSignal.reason ?? err;
      }
      throw err;
    } finally {
      cleanup();
    }
  }

  get<T>(endpoint: string, options?: Omit<RequestOptions<T>, 'method' | 'body'>): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: 'GET' });
  }

  post<T>(
    endpoint: string,
    body?: unknown,
    options?: Omit<RequestOptions<T>, 'method' | 'body'>,
  ): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: 'POST', body });
  }

  put<T>(
    endpoint: string,
    body?: unknown,
    options?: Omit<RequestOptions<T>, 'method' | 'body'>,
  ): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: 'PUT', body });
  }

  delete<T>(endpoint: string, options?: Omit<RequestOptions<T>, 'method' | 'body'>): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: 'DELETE' });
  }
}

export function createApiClient(config?: ApiClientConfig): ApiClient {
  return new ApiClient(config);
}

export const apiClient = new ApiClient();
