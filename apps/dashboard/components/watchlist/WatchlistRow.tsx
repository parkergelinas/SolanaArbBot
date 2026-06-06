'use client';

import { memo } from 'react';

import { formatChangePct, formatPrice, formatVolume } from '@/lib/formatters';
import type { WatchlistRowData } from './Watchlist';

interface WatchlistRowProps {
  row: WatchlistRowData;
  active: boolean;
  onSelect: () => void;
  onRemove?: () => void;
  mobile?: boolean;
}

const WatchlistRow = memo(function WatchlistRow({
  row,
  active,
  onSelect,
  onRemove,
  mobile = false,
}: WatchlistRowProps) {
  const chgClass =
    row.changePct > 0
      ? 'text-ds-green'
      : row.changePct < 0
        ? 'text-ds-red'
        : 'text-ds-text-secondary';

  return (
    <div
      className={`group relative w-full grid grid-cols-[3.5rem_1fr_3rem_2.5rem] gap-1 px-2 items-center font-mono tabular-nums border-b border-ds-border transition-colors ${
        mobile ? 'h-11 text-[13px]' : 'h-7 text-[12px]'
      } ${
        active
          ? 'bg-ds-elevated border-l-2 border-l-ds-blue'
          : 'hover:bg-ds-elevated border-l-2 border-l-transparent'
      }`}
      style={{ lineHeight: 1 }}
    >
      <button type="button" className="text-left font-ui font-medium text-ds-text-primary truncate" onClick={onSelect}>
        {row.symbol}
      </button>
      <button type="button" className={`text-right ${row.stale ? 'text-ds-text-secondary' : 'text-ds-text-primary'}`} onClick={onSelect}>
        {row.price > 0 ? formatPrice(row.price) : '—'}
      </button>
      <button type="button" className={`text-right ${chgClass}`} onClick={onSelect}>
        {formatChangePct(row.changePct)}
      </button>
      <button type="button" className="text-right text-ds-text-secondary" onClick={onSelect}>
        {formatVolume(row.volume)}
      </button>
      {onRemove && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
          className={`absolute right-1 top-1/2 -translate-y-1/2 text-[11px] text-ds-text-muted hover:text-ds-red px-2 ${
            mobile ? 'block touch-target' : 'hidden group-hover:block'
          }`}
          aria-label={`Remove ${row.symbol}`}
        >
          ✕
        </button>
      )}
    </div>
  );
});

export default WatchlistRow;
