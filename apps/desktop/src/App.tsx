import { WorkspaceShell } from './components/layout/WorkspaceShell';

export interface ComponentHealth {
  status: 'up' | 'down';
  latency_ms?: number | null;
  error?: string | null;
}

export interface HealthResponse {
  status: 'ok' | 'degraded';
  version: string;
  timestamp: string;
  database: ComponentHealth;
  elasticsearch: ComponentHealth;
}

export type HealthState = 'connecting' | 'ok' | 'dependency_unavailable' | 'offline';

export default function App() {
  return <WorkspaceShell />;
}
