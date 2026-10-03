import { describe, it, expect } from 'vitest';
import type { HealthResponse, HealthState } from '../App';

function resolveHealthState(
  fetchSuccess: boolean,
  responseOk: boolean,
  data: HealthResponse | null,
): HealthState {
  if (!fetchSuccess || !data) {
    return 'offline';
  }
  if (responseOk && data.status === 'ok') {
    return 'ok';
  }
  return 'dependency_unavailable';
}

describe('Health state resolution logic', () => {
  it('identifies offline state when fetch throws or backend is unreachable', () => {
    const state = resolveHealthState(false, false, null);
    expect(state).toBe('offline');
  });

  it('identifies healthy state when backend is ok and dependencies are up', () => {
    const mockData: HealthResponse = {
      status: 'ok',
      version: '0.1.0',
      timestamp: '2026-10-03T01:30:00Z',
      database: { status: 'up', latency_ms: 2 },
      elasticsearch: { status: 'up', latency_ms: 4 },
    };
    const state = resolveHealthState(true, true, mockData);
    expect(state).toBe('ok');
  });

  it('identifies dependency unavailable when backend is reachable but dependencies degraded', () => {
    const mockData: HealthResponse = {
      status: 'degraded',
      version: '0.1.0',
      timestamp: '2026-10-03T01:30:00Z',
      database: { status: 'down', error: 'Connection refused' },
      elasticsearch: { status: 'down', error: 'Connection refused' },
    };
    const state = resolveHealthState(true, true, mockData);
    expect(state).toBe('dependency_unavailable');
  });
});
