'use client';

import { useDexScreenerContext } from '@/components/terminal/DexScreenerProvider';
import { tokenMeta, tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

function fmt(n: number, digits = 4): string {
  if (n >= 1000) return n.toLocaleString(undefined, { maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(digits);
  return n.toFixed(Math.min(6, digits + 2));
}

export default function MarketStatsBar() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const token = useMarketStore((s) => (selectedMint ? s.tokens[selectedMint] : undefined));
  const { snapshot: dex } = useDexScreenerContext();

  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const meta = selectedMint ? tokenMeta(selectedMint) : undefined;
  const price = dex?.priceUsd ?? token?.price_usd ?? meta?.refPrice ?? 0;
  const chg = dex?.changeH24Pct ?? token?.changePct ?? 0;
  const vol = dex?.volumeH24Usd ?? token?.volume ?? 0;
  const liq = dex?.liquidityUsd ?? 0;
  const up = chg >= 0;

  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-1.5 border-b border-terminal-border bg-terminal-bg/80 text-[11px] mono shrink-0">
      <div className="flex items-center gap-2 pr-3 border-r border-terminal-border">
        <span className="text-sm font-bold text-terminal-accent">{symbol}</span>
        <span className="text-lg font-semibold tabular-nums text-slate-50">${fmt(price)}</span>
        <span className={`text-xs tabular-nums ${up ? 'text-flow-buy' : 'text-flow-sell'}`}>
          {up ? '▲' : '▼'} {Math.abs(chg).toFixed(2)}%
        </span>
      </div>
      <Stat label="Vol 24h" value={`$${fmt(vol, 0)}`} />
      {liq > 0 && <Stat label="Liq" value={`$${fmt(liq, 0)}`} />}
      {dex && (
        <Stat
          label="Pair"
          value={`${dex.baseSymbol}/${dex.quoteSymbol}`}
          className="text-slate-400"
        />
      )}
      <span className="text-[9px] text-terminal-muted ml-auto uppercase tracking-wider">
        DexScreener · USD
      </span>
    </div>
  );
}

function Stat({
  label,
  value,
  className = 'text-slate-300',
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <div className="flex items-center gap-1">
      <span className="text-terminal-muted text-[9px]">{label}</span>
      <span className={`tabular-nums ${className}`}>{value}</span>
    </div>
  );
}
