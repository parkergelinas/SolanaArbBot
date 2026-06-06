'use client';

import { useEffect, useState } from 'react';

import { formatPrice, formatUtcClock } from '@/lib/formatters';
import { useSelectedPairPrice } from '@/lib/hooks/useResolvedPrice';
import { SOL_MINT, tokenSymbol } from '@/lib/terminal/tokens';
import { usePaperStore } from '@/stores/paperStore';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

const STRATEGY_LABEL: Record<ConnectionMode, string> = {
  live: 'ARB · LIVE',
  degraded: 'ARB · DEGRADED',
  sim: 'ARB · SIM',
};

export default function Header() {
  const [clock, setClock] = useState('');
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const refreshConnectionMode = useStreamStore((s) => s.refreshConnectionMode);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const portfolio = usePaperStore((s) => s.portfolio);
  const resolved = useSelectedPairPrice();
  const symbol = selectedMint ? tokenSymbol(selectedMint) : 'SOL';
  const price = resolved.priceUsd;

  const solBalance = portfolio?.balances[SOL_MINT] ?? 0;

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
    <header className="flex items-center justify-between h-10 px-3 border-b border-ds-border bg-ds-surface shrink-0 font-ui">
      <div className="flex items-center gap-4 min-w-0">
        <div className="shrink-0">
          <div className="text-[13px] font-semibold tracking-[0.12em] text-ds-text-primary">
            SOLARB
          </div>
        </div>
        <span className="text-[12px] text-ds-text-secondary">{STRATEGY_LABEL[connectionMode]}</span>
      </div>

      <div className="flex items-center gap-6">
        <div className="text-right">
          <div className="text-[11px] text-ds-text-secondary uppercase tracking-[0.08em]">
            SOL Balance
          </div>
          <div className="text-[13px] font-mono tabular-nums text-ds-text-primary leading-none">
            {formatPrice(solBalance, 4)}
          </div>
        </div>
        <div className="text-right hidden sm:block">
          <div className="text-[11px] text-ds-text-secondary uppercase tracking-[0.08em]">
            {symbol}/USDC
          </div>
          <div className="text-[13px] font-mono tabular-nums text-ds-text-primary leading-none">
            {formatPrice(price)}
          </div>
        </div>
      </div>

      <div className="text-[12px] font-mono text-ds-text-secondary tabular-nums shrink-0">
        {clock}
      </div>
    </header>
  );
}
