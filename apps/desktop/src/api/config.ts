export const BACKEND_URL: string =
  import.meta.env.VITE_BACKEND_URL || 'http://127.0.0.1:3001';

export function getBackendUrl(): string {
  return BACKEND_URL;
}
