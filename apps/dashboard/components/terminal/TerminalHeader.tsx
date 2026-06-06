'use client';

import { useEffect, useState } from 'react';

import NetworkSwitcher from '@/components/wallet/NetworkSwitcher';
import { tokenMeta, tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

const MODE_STYLE: Record<
  ConnectionMode,
  { label: string; border: string; text: string; dot: string }
> = {
  live: {
    label: 'LIVE',
    border: 'border-terminal-accent/50 text-terminal-accent bg-terminal-accent/8',
    text: 'text-terminal-accent',
    dot: 'bg-terminal-accent live-pulse',
  },
  degraded: {
    label: 'DEGRADED',
    border: 'border-terminal-warn/40 text-terminal-warn bg-terminal-warn/8',
    text: 'text-terminal-warn',
    dot: 'bg-terminal-warn',
  },
  sim: {
    label: 'SIM',
    border: 'border-terminal-warn/40 text-terminal-warn bg-terminal-warn/8',
    text: 'text-terminal-warn',
    dot: 'bg-terminal-warn',
  },
};

export default function TerminalHeader() {
  const [now, setNow] = useState('');
  useEffect(() => {
    const tick = () =>
      setNow(
        new Date().toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
        }),
      );
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  const connectionMode = useStreamStore((s) => s.connectionMode);
  const refreshConnectionMode = useStreamStore((s) => s.refreshConnectionMode);
  const url = useStreamStore((s) => s.url);
  const messagesApplied = useStreamStore((s) => s.messagesApplied);
  const lastSeq = useStreamStore((s) => s.lastSeq);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const price = useMarketStore((s) => (selectedMint ? s.tokens[selectedMint] : undefined));
  const meta = selectedMint ? tokenMeta(selectedMint) : undefined;
  const symbol = selectedMint ? tokenSymbol(selectedMint) : 'SOL';

  useEffect(() => {
    const id = setInterval(refreshConnectionMode, 3_000);
    return () => clearInterval(id);
  }, [refreshConnectionMode]);

  const mode = MODE_STYLE[connectionMode];

  return (
    <header className="terminal-header flex items-center justify-between px-3 py-2 border-b border-terminal-border shrink-0">
      <div className="flex items-center gap-4">
        <div>
          <div className="text-sm font-bold tracking-[0.18em] text-terminal-accent">
            SOLARB
          </div>
          <div className="text-[9px] text-terminal-muted uppercase tracking-[0.25em]">
            Personal Trading Terminal
          </div>
        </div>
        <div
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-[10px] mono font-medium ${mode.border}`}
        >
          <span className={`w-2 h-2 rounded-full ${mode.dot}`} />
          {mode.label}
        </div>
        <div className="hidden md:flex items-center gap-3 text-[10px] mono text-terminal-muted">
          <span>Seq {lastSeq}</span>
          <span>{messagesApplied.toLocaleString()} evt</span>
        </div>
      </div>

      <div className="text-center">
        <div className="text-[10px] text-terminal-muted uppercase tracking-wider">
          {symbol}/USD
        </div>
        <div className="text-2xl font-semibold tabular-nums text-slate-50 leading-tight">
          $
          {(price?.price_usd ?? meta?.refPrice ?? 0).toLocaleString(undefined, {
            minimumFractionDigits: 2,
            maximumFractionDigits: price && price.price_usd < 1 ? 6 : 2,
          })}
        </div>
        {price && (
          <div
            className={`text-[11px] mono font-medium ${
              price.changePct >= 0 ? 'text-flow-buy' : 'text-flow-sell'
            }`}
          >
            {price.changePct >= 0 ? '+' : ''}
            {price.changePct.toFixed(2)}%
          </div>
        )}
      </div>

      <div className="flex items-center gap-3 text-right text-[10px] mono">
        <NetworkSwitcher compact />
        <div>
          <div className="text-slate-300 tabular-nums">{now}</div>
          <div className="text-terminal-muted truncate max-w-[10rem] mt-0.5" title={url}>
            {connectionMode !== 'sim' ? url.replace('ws://', '').replace('wss://', '') : 'demo · local sim'}
          </div>
        </div>
      </div>
    </header>
  );
}
