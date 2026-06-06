'use client';

import { formatChangePct, formatPrice, formatVolume } from '@/lib/formatters';
import { useSelectedPairPrice } from '@/lib/hooks/useResolvedPrice';
import { useDexScreenerContext } from '@/components/terminal/DexScreenerProvider';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useUiStore } from '@/stores/uiStore';
import { useMarketStore } from '@/stores/marketStore';

export default function MarketStatsBar() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const resolved = useSelectedPairPrice();
  const token = useMarketStore((s) => (selectedMint ? s.tokens[selectedMint] : undefined));
  const { snapshot: dex } = useDexScreenerContext();

  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const up = resolved.changeH24Pct >= 0;
  const liq = dex?.liquidityUsd ?? 0;

  return (
    <div className="flex items-center gap-4 px-3 h-9 border-b border-ds-border bg-ds-elevated/40 shrink-0 overflow-x-auto terminal-scroll">
      <div className="flex items-center gap-2 pr-3 border-r border-ds-border shrink-0">
        <span className="text-[12px] font-semibold text-ds-text-primary">{symbol}/USDC</span>
        <span className="text-[14px] font-mono font-medium tabular-nums text-ds-text-primary">
          {resolved.priceUsd > 0 ? `$${formatPrice(resolved.priceUsd)}` : '—'}
        </span>
        <span
          className={`text-[11px] font-mono tabular-nums ${up ? 'text-ds-green' : 'text-ds-red'}`}
        >
          {formatChangePct(resolved.changeH24Pct)}
        </span>
        {resolved.stale && (
          <span className="text-[9px] text-ds-amber uppercase tracking-wider">stale</span>
        )}
      </div>
      <Stat label="24h Vol" value={`$${formatVolume(resolved.volumeH24Usd)}`} />
      {liq > 0 && <Stat label="Liquidity" value={`$${formatVolume(liq)}`} />}
      {dex && (
        <Stat label="Pair" value={`${dex.baseSymbol}/${dex.quoteSymbol}`} muted />
      )}
      {token && (
        <Stat
          label="Flow"
          value={`${token.buyFlow > token.sellFlow ? '+' : ''}${((token.flowImbalance ?? 0) * 100).toFixed(0)}%`}
          muted
        />
      )}
      <span className="text-[9px] text-ds-text-muted ml-auto uppercase tracking-wider shrink-0">
        {resolved.source === 'dex' ? 'DexScreener' : resolved.source} · USD
      </span>
    </div>
  );
}

function Stat({
  label,
  value,
  muted,
}: {
  label: string;
  value: string;
  muted?: boolean;
}) {
  return (
    <div className="flex items-center gap-1.5 shrink-0">
      <span className="text-[9px] text-ds-text-muted uppercase tracking-wider">{label}</span>
      <span
        className={`text-[11px] font-mono tabular-nums ${muted ? 'text-ds-text-secondary' : 'text-ds-text-primary'}`}
      >
        {value}
      </span>
    </div>
  );
}
