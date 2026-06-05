'use client';

import { useStreamStore } from '@/stores/streamStore';

function LatBar({
  label,
  ms,
  budget,
}: {
  label: string;
  ms: number;
  budget: number;
}) {
  const pct = Math.min(100, (ms / budget) * 100);
  const over = ms > budget;
  return (
    <div className="flex items-center gap-2 min-w-0">
      <span className="text-[10px] uppercase tracking-wider text-terminal-muted w-14 shrink-0">
        {label}
      </span>
      <div className="flex-1 h-1.5 bg-terminal-border rounded-sm overflow-hidden">
        <div
          className={`h-full transition-all duration-150 ${over ? 'bg-flow-sell' : 'bg-cyan-400'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span
        className={`mono text-[10px] w-12 text-right shrink-0 ${over ? 'text-flow-sell' : 'text-terminal-live'}`}
      >
        {ms.toFixed(1)}ms
      </span>
    </div>
  );
}

export default function LatencyMonitor() {
  const connected = useStreamStore((s) => s.connected);
  const latency = useStreamStore((s) => s.latency);
  const batches = useStreamStore((s) => s.batchesFlushed);

  return (
    <div className="px-3 py-1.5 border-b border-terminal-border bg-terminal-panel flex items-center gap-4 text-xs">
      <span className="text-terminal-muted uppercase tracking-widest text-[10px] w-16 shrink-0">
        Latency
      </span>
      <div className="flex-1 grid grid-cols-2 lg:grid-cols-4 gap-x-4 gap-y-1">
        <LatBar label="WS" ms={latency.wsMs} budget={15} />
        <LatBar label="Ingest" ms={latency.ingestMs} budget={10} />
        <LatBar label="Render" ms={latency.renderMs} budget={33} />
        <LatBar label="E2E" ms={latency.e2eMs} budget={150} />
      </div>
      <div className="text-[10px] text-terminal-muted mono shrink-0 hidden sm:block">
        seq {latency.seq} · {batches} batches ·{' '}
        <span className={connected ? 'text-terminal-live' : 'text-flow-sell'}>
          {connected ? 'LIVE' : 'DOWN'}
        </span>
      </div>
    </div>
  );
}
