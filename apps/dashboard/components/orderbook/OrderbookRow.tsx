'use client';

import { memo } from 'react';

import { formatPrice, formatSize } from '@/lib/formatters';
import type { OrderbookLevel } from './useOrderbookData';

interface OrderbookRowProps {
  level: OrderbookLevel;
  side: 'bid' | 'ask';
}

const OrderbookRow = memo(function OrderbookRow({ level, side }: OrderbookRowProps) {
  const isBid = side === 'bid';

  return (
    <div
      className="relative grid grid-cols-[1fr_1fr_1fr] h-[22px] items-center px-2 text-[11px] font-mono tabular-nums leading-none"
      style={{ lineHeight: 1 }}
    >
      <div
        className={`absolute inset-y-0 ${isBid ? 'left-0' : 'right-0'} ${isBid ? 'bg-ds-green-dim' : 'bg-ds-red-dim'}`}
        style={{ width: `${level.depthPct}%` }}
      />
      <span className="z-[1] text-right text-ds-text-primary">{formatSize(level.size)}</span>
      <span
        className={`z-[1] text-center ${isBid ? 'text-ds-green' : 'text-ds-red'}`}
      >
        {formatPrice(level.price)}
      </span>
      <span className="z-[1] text-right text-ds-text-primary">
        {formatSize(level.cumulative)}
      </span>
    </div>
  );
});

export default OrderbookRow;
