'use client';

import { formatPnL } from '@/lib/formatters';

interface PnLSummaryProps {
  today: number;
  week: number;
  allTime: number;
}

export default function PnLSummary({ today, week, allTime }: PnLSummaryProps) {
  const pnlClass = (v: number) =>
    v > 0 ? 'text-ds-green' : v < 0 ? 'text-ds-red' : 'text-ds-text-secondary';

  return (
    <div className="border-t border-ds-border shrink-0">
      <div className="grid grid-cols-3 gap-0 px-2 py-2 text-center">
        <div>
          <div className="text-[11px] uppercase tracking-[0.08em] text-ds-text-secondary font-ui">
            Today
          </div>
          <div className={`text-[12px] font-mono tabular-nums font-medium ${pnlClass(today)}`}>
            {formatPnL(today)}
          </div>
        </div>
        <div>
          <div className="text-[11px] uppercase tracking-[0.08em] text-ds-text-secondary font-ui">
            Week
          </div>
          <div className={`text-[12px] font-mono tabular-nums font-medium ${pnlClass(week)}`}>
            {formatPnL(week)}
          </div>
        </div>
        <div>
          <div className="text-[11px] uppercase tracking-[0.08em] text-ds-text-secondary font-ui">
            All Time
          </div>
          <div className={`text-[12px] font-mono tabular-nums font-medium ${pnlClass(allTime)}`}>
            {formatPnL(allTime)}
          </div>
        </div>
      </div>
    </div>
  );
}
