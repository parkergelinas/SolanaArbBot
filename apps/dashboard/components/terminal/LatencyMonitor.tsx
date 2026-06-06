'use client';

import { useStreamStore } from '@/stores/streamStore';

function LatBar({ label, ms, budget }: { label: string; ms: number; budget: number }) {
  const pct = Math.min(100, (ms / budget) * 100);
  const over = ms > budget;
  return (
    <div className="flex items-center gap-2 min-w-0">
      <span className="text-[9px] uppercase tracking-wider text-ds-text-muted w-12 shrink-0">
        {label}
      </span>
      <div className="flex-1 h-1 bg-ds-border rounded-terminal overflow-hidden">
        <div
          className={`h-full transition-all duration-150 ${over ? 'bg-ds-red' : 'bg-ds-blue'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span
        className={`font-mono text-[9px] w-11 text-right shrink-0 tabular-nums ${over ? 'text-ds-red' : 'text-ds-text-secondary'}`}
      >
        {ms.toFixed(0)}ms
      </span>
    </div>
  );
}

export default function LatencyMonitor() {
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const latency = useStreamStore((s) => s.latency);
  const batches = useStreamStore((s) => s.batchesFlushed);

  return (
    <div className="px-3 h-7 border-b border-ds-border bg-ds-elevated/30 flex items-center gap-4 shrink-0 min-w-0">
      <span className="text-[9px] text-ds-text-muted uppercase tracking-widest shrink-0">
        Latency
      </span>
      <div className="flex-1 min-w-0 grid grid-cols-2 lg:grid-cols-4 gap-x-3">
        <LatBar label="WS" ms={latency.wsMs} budget={100} />
        <LatBar label="Ingest" ms={latency.ingestMs} budget={50} />
        <LatBar label="Render" ms={latency.renderMs} budget={33} />
        <LatBar label="E2E" ms={latency.e2eMs} budget={300} />
      </div>
      <div className="text-[9px] font-mono text-ds-text-muted shrink-0 hidden md:block">
        seq {latency.seq} · {batches}b ·{' '}
        <span
          className={
            connectionMode === 'live'
              ? 'text-ds-green'
              : connectionMode === 'degraded'
                ? 'text-ds-amber'
                : 'text-ds-text-muted'
          }
        >
          {connectionMode.toUpperCase()}
        </span>
      </div>
    </div>
  );
}
