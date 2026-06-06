'use client';

import { useEffect, useState } from 'react';

import { formatUtcClock } from '@/lib/formatters';
import { isPaperTradingEnabled } from '@/lib/config/env';
import { useMarketStore } from '@/stores/marketStore';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

type ConnState = 'connected' | 'degraded' | 'disconnected';

function connectionState(mode: ConnectionMode, connected: boolean): ConnState {
  if (!connected) return 'disconnected';
  if (mode === 'degraded') return 'degraded';
  return 'connected';
}

function latencyColor(ms: number): string {
  if (ms < 100) return 'text-ds-green';
  if (ms <= 300) return 'text-ds-amber';
  return 'text-ds-red';
}

export default function StatusBar() {
  const [halted, setHalted] = useState(false);
  const [clock, setClock] = useState('');
  const connected = useStreamStore((s) => s.connected);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const latency = useStreamStore((s) => s.latency);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const slot = useMarketStore((s) => {
    if (!selectedMint) return 0;
    return s.tokens[selectedMint]?.slot ?? s.prices[selectedMint]?.slot ?? 0;
  });

  const conn = connectionState(connectionMode, connected);

  useEffect(() => {
    const tick = () => setClock(formatUtcClock());
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  const dotClass =
    conn === 'connected'
      ? 'bg-ds-green'
      : conn === 'degraded'
        ? 'bg-ds-amber'
        : 'bg-ds-red status-dot-disconnected';

  const label =
    conn === 'connected' ? 'CONNECTED' : conn === 'degraded' ? 'DEGRADED' : 'DISCONNECTED';

  return (
    <footer className="flex items-center justify-between h-7 px-3 border-t border-ds-border bg-ds-surface shrink-0 text-[11px] font-mono">
      <div className="flex items-center gap-3">
        <span className="flex items-center gap-1.5 text-ds-text-secondary">
          <span className={`inline-block w-1.5 h-1.5 rounded-full ${dotClass}`} />
          {label}
        </span>
        <span className="text-ds-text-secondary">
          Slot: {slot > 0 ? slot.toLocaleString() : '—'}
        </span>
        <span className="text-ds-text-secondary">
          Latency:{' '}
          <span className={latencyColor(latency.wsMs)}>{latency.wsMs}ms</span>
        </span>
      </div>

      <div />

      <div className="flex items-center gap-2">
        {isPaperTradingEnabled() && (
          <span className="px-1.5 py-px border border-ds-amber text-ds-amber bg-ds-elevated rounded-terminal uppercase tracking-wider text-[10px]">
            paper
          </span>
        )}
        <button
          type="button"
          onClick={() => setHalted((h) => !h)}
          className={`px-2 py-px border rounded-terminal text-[10px] uppercase tracking-wider transition-colors ${
            halted
              ? 'border-ds-red bg-ds-red text-ds-base'
              : 'border-ds-red text-ds-red bg-ds-surface hover:bg-ds-red hover:text-ds-base'
          }`}
        >
          ■ HALT
        </button>
        <span className="text-ds-text-secondary tabular-nums">{clock}</span>
      </div>
    </footer>
  );
}
