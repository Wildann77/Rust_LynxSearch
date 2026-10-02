import { describe, it, expect } from 'vitest';
import { BACKEND_URL, getBackendUrl } from '../api/config';

describe('Desktop API configuration', () => {
  it('defaults to localhost 127.0.0.1:3001', () => {
    expect(BACKEND_URL).toBe('http://127.0.0.1:3001');
    expect(getBackendUrl()).toBe('http://127.0.0.1:3001');
  });
});
