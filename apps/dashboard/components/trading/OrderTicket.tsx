'use client';

import { useMemo } from 'react';

import { formatPnL, formatPrice } from '@/lib/formatters';
import { useResolvedPrices } from '@/lib/hooks/useResolvedPrice';
import { SOL_MINT, tokenSymbol } from '@/lib/terminal/tokens';
import { useNetworkStore } from '@/stores/networkStore';
import { usePaperStore } from '@/stores/paperStore';
import { useUiStore } from '@/stores/uiStore';

const SIZES = [0.1, 0.25, 0.5, 1] as const;

export default function OrderTicket() {
  const cluster = useNetworkStore((s) => s.cluster);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const portfolio = usePaperStore((s) => s.portfolio);
  const enabled = usePaperStore((s) => s.enabled);
  const autoTrade = usePaperStore((s) => s.autoTrade);
  const lastFill = usePaperStore((s) => s.lastFill);
  const error = usePaperStore((s) => s.error);
  const setEnabled = usePaperStore((s) => s.setEnabled);
  const setAutoTrade = usePaperStore((s) => s.setAutoTrade);
  const quickTrade = usePaperStore((s) => s.quickTrade);
  const reset = usePaperStore((s) => s.reset);
  const equityUsd = usePaperStore((s) => s.equityUsd);

  const priceMints = useMemo(() => {
    const mints = new Set<string>([SOL_MINT]);
    if (selectedMint) mints.add(selectedMint);
    if (portfolio) {
      for (const mint of Object.keys(portfolio.balances)) mints.add(mint);
    }
    return Array.from(mints);
  }, [selectedMint, portfolio]);

  const resolved = useResolvedPrices(priceMints);

  const prices = useMemo(() => {
    const out: Record<string, number> = {};
    for (const [mint, r] of Object.entries(resolved)) out[mint] = r.priceUsd;
    return out;
  }, [resolved]);

  const equity = equityUsd(prices);
  const solBal = portfolio?.balances[SOL_MINT] ?? 0;
  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const canTrade = enabled && selectedMint && !selectedMint.startsWith('So1111');

  const trade = (side: 'buy' | 'sell', sol: number) => {
    if (!selectedMint || !canTrade) return;
    quickTrade(selectedMint, side, sol, prices);
  };

  return (
    <div className="flex flex-col h-full min-h-0 text-[11px]">
      <div className="grid grid-cols-2 gap-2 p-3 border-b border-ds-border">
        <Metric label="Equity" value={formatPnL(equity, { signed: false })} />
        <Metric label="SOL" value={formatPrice(solBal, 3)} />
        <Metric
          label="Realized"
          value={formatPnL(portfolio?.realizedPnlUsd ?? 0)}
          colored={portfolio?.realizedPnlUsd}
        />
        <Metric label="Mode" value={cluster === 'devnet' ? 'Devnet' : 'Mainnet'} />
      </div>

      <div className="px-3 py-2 border-b border-ds-border flex items-center justify-between">
        <span className="text-ds-text-secondary uppercase tracking-wider text-[10px]">
          Paper · {symbol}
        </span>
        <label className="flex items-center gap-1.5 cursor-pointer text-ds-text-secondary">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
            className="accent-ds-blue w-3 h-3"
          />
          <span>On</span>
        </label>
      </div>

      {error && <p className="px-3 py-1 text-ds-red text-[10px]">{error}</p>}

      <div className="p-3 space-y-2 flex-1 min-h-0 overflow-y-auto terminal-scroll">
        <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mb-1">Buy</div>
        <div className="grid grid-cols-2 gap-1.5">
          {SIZES.map((amt) => (
            <button
              key={`b-${amt}`}
              type="button"
              disabled={!canTrade}
              onClick={() => trade('buy', amt)}
              className="terminal-btn terminal-btn-buy"
            >
              {amt} SOL
            </button>
          ))}
        </div>

        <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mb-1 pt-1">
          Sell → SOL
        </div>
        <div className="grid grid-cols-2 gap-1.5">
          {SIZES.slice(0, 2).map((amt) => (
            <button
              key={`s-${amt}`}
              type="button"
              disabled={!canTrade}
              onClick={() => trade('sell', amt)}
              className="terminal-btn terminal-btn-sell"
            >
              {amt} SOL
            </button>
          ))}
        </div>
      </div>

      <div className="p-3 border-t border-ds-border space-y-2 shrink-0">
        <label className="flex items-center gap-2 cursor-pointer text-ds-text-secondary text-[10px]">
          <input
            type="checkbox"
            checked={autoTrade}
            onChange={(e) => setAutoTrade(e.target.checked)}
            className="accent-ds-blue w-3 h-3"
          />
          Auto-trade on signals
        </label>
        {lastFill && (
          <div className="font-mono text-[10px] text-ds-text-secondary truncate">
            Last: {lastFill.side} {tokenSymbol(lastFill.tokenOut)}{' '}
            <span className={lastFill.pnlUsd >= 0 ? 'text-ds-green' : 'text-ds-red'}>
              {formatPnL(lastFill.pnlUsd)}
            </span>
          </div>
        )}
        <button
          type="button"
          onClick={() => reset(cluster)}
          className="text-[10px] text-ds-text-muted hover:text-ds-red transition-colors"
        >
          Reset portfolio
        </button>
      </div>
    </div>
  );
}

function Metric({
  label,
  value,
  colored,
}: {
  label: string;
  value: string;
  colored?: number;
}) {
  const color =
    colored != null
      ? colored > 0
        ? 'text-ds-green'
        : colored < 0
          ? 'text-ds-red'
          : 'text-ds-text-secondary'
      : 'text-ds-text-primary';
  return (
    <div>
      <div className="text-[9px] text-ds-text-muted uppercase tracking-wider">{label}</div>
      <div className={`text-[12px] font-mono tabular-nums ${color}`}>{value}</div>
    </div>
  );
}
