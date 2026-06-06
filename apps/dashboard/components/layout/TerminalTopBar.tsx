'use client';

import { useEffect, useState } from 'react';

import { formatChangePct, formatPrice, formatUtcClock, formatVolume } from '@/lib/formatters';
import { useSelectedPairPrice } from '@/lib/hooks/useResolvedPrice';
import { SOL_MINT, tokenSymbol } from '@/lib/terminal/tokens';
import { usePaperStore } from '@/stores/paperStore';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

const MODE: Record<ConnectionMode, { label: string; dot: string; ring: string }> = {
  live: { label: 'LIVE', dot: 'bg-ds-green', ring: 'border-ds-green/30' },
  degraded: { label: 'DEGRADED', dot: 'bg-ds-amber', ring: 'border-ds-amber/30' },
  sim: { label: 'SIM', dot: 'bg-ds-text-muted', ring: 'border-ds-border' },
};

export default function TerminalTopBar() {
  const [clock, setClock] = useState('');
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const refreshConnectionMode = useStreamStore((s) => s.refreshConnectionMode);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const portfolio = usePaperStore((s) => s.portfolio);
  const resolved = useSelectedPairPrice();
  const symbol = selectedMint ? tokenSymbol(selectedMint) : 'SOL';
  const price = resolved.priceUsd;
  const changePct = resolved.changeH24Pct;
  const volume = resolved.volumeH24Usd;
  const solBalance = portfolio?.balances[SOL_MINT] ?? 0;
  const mode = MODE[connectionMode];
  const up = changePct >= 0;

  useEffect(() => {
    const tick = () => setClock(formatUtcClock());
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    const id = setInterval(refreshConnectionMode, 3_000);
    return () => clearInterval(id);
  }, [refreshConnectionMode]);

  return (
    <header className="flex items-center h-12 px-4 border-b border-ds-border bg-ds-surface shrink-0 gap-4">
      <div className="flex items-center gap-3 shrink-0">
        <div>
          <div className="text-[14px] font-semibold tracking-[0.14em] text-ds-text-primary leading-none">
            SOLARB
          </div>
          <div className="text-[9px] text-ds-text-muted uppercase tracking-[0.2em] mt-0.5">
            Pro Terminal
          </div>
        </div>
        <div
          className={`flex items-center gap-1.5 px-2 py-1 border rounded-terminal text-[10px] font-mono ${mode.ring}`}
        >
          <span className={`w-1.5 h-1.5 rounded-full ${mode.dot}`} />
          <span className="text-ds-text-secondary">{mode.label}</span>
        </div>
      </div>

      <div className="flex items-baseline gap-3 min-w-0 flex-1 justify-center">
        <span className="text-[13px] font-semibold text-ds-text-primary">{symbol}</span>
        <span className="text-[22px] font-mono font-medium tabular-nums text-ds-text-primary leading-none">
          ${formatPrice(price)}
        </span>
        <span
          className={`text-[12px] font-mono tabular-nums ${up ? 'text-ds-green' : 'text-ds-red'}`}
        >
          {formatChangePct(changePct)}
        </span>
        <span className="hidden lg:inline text-[11px] font-mono text-ds-text-secondary">
          Vol {formatVolume(volume)}
        </span>
      </div>

      <div className="flex items-center gap-4 shrink-0">
        <div className="hidden md:block text-right">
          <div className="text-[9px] text-ds-text-muted uppercase tracking-wider">SOL</div>
          <div className="text-[12px] font-mono tabular-nums text-ds-text-primary">
            {formatPrice(solBalance, 3)}
          </div>
        </div>
        <span className="text-[11px] font-mono text-ds-text-secondary tabular-nums w-[72px] text-right">
          {clock}
        </span>
      </div>
    </header>
  );
}
