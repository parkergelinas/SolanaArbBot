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

export default function StatusBar({ compactOnMobile = false }: { compactOnMobile?: boolean }) {
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
    <footer
      className={`flex items-center justify-between border-t border-ds-border bg-ds-surface shrink-0 font-mono gap-2 ${
        compactOnMobile
          ? 'h-6 px-2 text-[9px] lg:min-h-7 lg:h-auto lg:px-3 lg:py-0 lg:text-[11px]'
          : 'min-h-7 h-auto sm:h-7 px-2 sm:px-3 py-1 sm:py-0 text-[10px] sm:text-[11px]'
      }`}
    >
      <div className="flex items-center gap-2 sm:gap-3 min-w-0 overflow-x-auto terminal-scroll">
        <span className="flex items-center gap-1.5 text-ds-text-secondary shrink-0">
          <span className={`inline-block w-1.5 h-1.5 rounded-full ${dotClass}`} />
          <span className="hidden min-[400px]:inline">{label}</span>
        </span>
        <span className="text-ds-text-secondary shrink-0 hidden sm:inline">
          Slot: {slot > 0 ? slot.toLocaleString() : '—'}
        </span>
        <span className="text-ds-text-secondary shrink-0">
          <span className="sm:hidden">Lat </span>
          <span className="hidden sm:inline">Latency: </span>
          <span className={latencyColor(latency.wsMs)}>{latency.wsMs}ms</span>
        </span>
      </div>

      <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
        {isPaperTradingEnabled() && (
          <span className="px-1.5 py-px border border-ds-amber text-ds-amber bg-ds-elevated rounded-terminal uppercase tracking-wider text-[9px] sm:text-[10px]">
            paper
          </span>
        )}
        <button
          type="button"
          onClick={() => setHalted((h) => !h)}
          className={`${compactOnMobile ? 'px-1.5 py-px' : 'touch-target px-2 py-1 sm:py-px'} border rounded-terminal text-[9px] sm:text-[10px] uppercase tracking-wider transition-colors ${
            halted
              ? 'border-ds-red bg-ds-red text-ds-base'
              : 'border-ds-red text-ds-red bg-ds-surface hover:bg-ds-red hover:text-ds-base'
          }`}
        >
          ■ HALT
        </button>
        <span className="hidden sm:inline text-ds-text-secondary tabular-nums">{clock}</span>
      </div>
    </footer>
  );
}
