'use client';

import { useMemo } from 'react';

import { useResolvedPrices } from '@/lib/hooks/useResolvedPrice';
import { SOL_MINT, tokenSymbol } from '@/lib/terminal/tokens';
import { usePaperStore } from '@/stores/paperStore';
import { useUiStore } from '@/stores/uiStore';

interface QuickTradeBarProps {
  onOpenBook: () => void;
}

export default function QuickTradeBar({ onOpenBook }: QuickTradeBarProps) {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const portfolio = usePaperStore((s) => s.portfolio);
  const enabled = usePaperStore((s) => s.enabled);
  const quickTrade = usePaperStore((s) => s.quickTrade);

  const priceMints = useMemo(() => {
    const mints = new Set<string>([SOL_MINT]);
    if (selectedMint) mints.add(selectedMint);
    return Array.from(mints);
  }, [selectedMint]);

  const resolved = useResolvedPrices(priceMints);
  const prices = useMemo(() => {
    const out: Record<string, number> = {};
    for (const [mint, r] of Object.entries(resolved)) out[mint] = r.priceUsd;
    return out;
  }, [resolved]);

  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const canTrade = enabled && selectedMint && !selectedMint.startsWith('So1111');

  const trade = (side: 'buy' | 'sell', sol: number) => {
    if (!selectedMint || !canTrade) return;
    quickTrade(selectedMint, side, sol, prices);
  };

  return (
    <div className="lg:hidden flex items-center gap-1.5 px-2 py-2 border-t border-ds-border bg-ds-surface shrink-0">
      <button
        type="button"
        onClick={onOpenBook}
        className="touch-target shrink-0 px-3 rounded-terminal border border-ds-border text-[11px] font-medium text-ds-text-secondary hover:text-ds-text-primary hover:bg-ds-elevated/60 transition-colors"
      >
        Book
      </button>
      <button
        type="button"
        disabled={!canTrade}
        onClick={() => trade('buy', 0.1)}
        className="touch-target flex-1 terminal-btn terminal-btn-buy text-[12px] !py-2.5"
      >
        Buy 0.1
      </button>
      <button
        type="button"
        disabled={!canTrade}
        onClick={() => trade('buy', 0.5)}
        className="touch-target flex-1 terminal-btn terminal-btn-buy text-[12px] !py-2.5"
      >
        Buy 0.5
      </button>
      <button
        type="button"
        disabled={!canTrade}
        onClick={() => trade('sell', 0.25)}
        className="touch-target flex-1 terminal-btn terminal-btn-sell text-[12px] !py-2.5"
        title={symbol !== '—' ? `Sell ${symbol}` : undefined}
      >
        Sell
      </button>
    </div>
  );
}
