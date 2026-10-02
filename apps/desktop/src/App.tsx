import { useState, useEffect } from 'react';
import { Search, Server, ShieldCheck, Terminal, Cpu, Database } from 'lucide-react';
import { BACKEND_URL } from './api/config';

interface BackendHealth {
  status: string;
  postgres?: boolean;
  elasticsearch?: boolean;
}

export default function App() {
  const [backendHealth, setBackendHealth] = useState<BackendHealth | null>(null);
  const [isConnecting, setIsConnecting] = useState<boolean>(true);

  useEffect(() => {
    async function checkHealth() {
      try {
        const response = await fetch(`${BACKEND_URL}/api/health`);
        if (response.ok) {
          const data = (await response.json()) as BackendHealth;
          setBackendHealth(data);
        } else {
          setBackendHealth({ status: 'offline' });
        }
      } catch {
        setBackendHealth({ status: 'offline' });
      } finally {
        setIsConnecting(false);
      }
    }

    void checkHealth();
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col bg-background text-foreground font-sans antialiased overflow-hidden select-none">
      {/* Top Application Header */}
      <header
        data-tauri-drag-region
        className="flex h-12 items-center justify-between border-b border-border px-4 bg-card select-none"
      >
        <div className="flex items-center gap-2.5">
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary text-black font-mono font-bold text-xs">
            L
          </div>
          <span className="font-semibold text-sm tracking-tight">LynxSearch</span>
          <span className="rounded-full bg-secondary px-2 py-0.5 text-[11px] font-mono text-muted-foreground border border-border">
            v1.0.0-desktop
          </span>
        </div>

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground font-mono">
            <span
              className={`h-2 w-2 rounded-full ${
                isConnecting
                  ? 'bg-amber-400 animate-pulse'
                  : backendHealth?.status === 'ok'
                    ? 'bg-primary'
                    : 'bg-destructive'
              }`}
            />
            <span>
              {isConnecting
                ? 'Connecting to backend...'
                : backendHealth?.status === 'ok'
                  ? `Backend Connected (${BACKEND_URL.replace(/^https?:\/\//, '')})`
                  : `Backend Offline (${BACKEND_URL.replace(/^https?:\/\//, '')})`}
            </span>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex flex-1 flex-col items-center justify-center p-8">
        <div className="w-full max-w-xl space-y-6 text-center">
          <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-secondary/80 border border-border text-primary shadow-inner">
            <Search className="h-7 w-7" />
          </div>

          <div className="space-y-2">
            <h1 className="text-2xl font-bold tracking-tight">LynxSearch Desktop Runtime</h1>
            <p className="text-sm text-muted-foreground max-w-md mx-auto">
              Developer Knowledge Base & Code Search Engine. Running in standalone desktop webview host.
            </p>
          </div>

          {/* Architecture Status Grid */}
          <div className="grid grid-cols-3 gap-3 text-left">
            <div className="rounded-lg border border-border bg-card p-3.5 space-y-1.5">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Cpu className="h-4 w-4 text-primary" />
                <span>Frontend Host</span>
              </div>
              <p className="text-xs font-semibold">Tauri 2.12 + React 19</p>
              <p className="text-[11px] text-muted-foreground">Standalone Webview</p>
            </div>

            <div className="rounded-lg border border-border bg-card p-3.5 space-y-1.5">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Server className="h-4 w-4 text-primary" />
                <span>Backend Service</span>
              </div>
              <p className="text-xs font-semibold">Axum 0.8 / Tokio</p>
              <p className="text-[11px] text-muted-foreground">Port 3001 (No Sidecar)</p>
            </div>

            <div className="rounded-lg border border-border bg-card p-3.5 space-y-1.5">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Database className="h-4 w-4 text-primary" />
                <span>Storage Layer</span>
              </div>
              <p className="text-xs font-semibold">Postgres + ES 8</p>
              <p className="text-[11px] text-muted-foreground">Metadata & BM25</p>
            </div>
          </div>

          {/* Next Steps / Phase Indicator */}
          <div className="rounded-md border border-border/60 bg-secondary/40 px-4 py-3 flex items-center justify-between text-xs font-mono">
            <span className="flex items-center gap-2 text-muted-foreground">
              <Terminal className="h-4 w-4" />
              <span>Phase 1 Setup: Desktop Initialized</span>
            </span>
            <span className="inline-flex items-center gap-1 text-primary">
              <ShieldCheck className="h-3.5 w-3.5" />
              <span>Guardrails Enforced</span>
            </span>
          </div>
        </div>
      </main>
    </div>
  );
}
