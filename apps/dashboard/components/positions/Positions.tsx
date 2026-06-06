'use client';



import { useMemo, useState } from 'react';



import { formatPnL, formatPrice, formatSize } from '@/lib/formatters';

import { useResolvedPrices } from '@/lib/hooks/useResolvedPrice';
import { tokenSymbol } from '@/lib/terminal/tokens';

import { usePaperStore } from '@/stores/paperStore';

import PnLSummary from './PnLSummary';



function periodPnl(fills: { timestampMs: number; pnlUsd: number }[], sinceMs: number): number {

  return fills

    .filter((f) => f.timestampMs >= sinceMs)

    .reduce((sum, f) => sum + f.pnlUsd, 0);

}



export default function Positions({ embedded = false }: { embedded?: boolean }) {

  const portfolio = usePaperStore((s) => s.portfolio);

  const [hoveredMint, setHoveredMint] = useState<string | null>(null);



  const priceMints = useMemo(() => {
    if (!portfolio) return [];
    return Object.keys(portfolio.balances).filter((m) => (portfolio.balances[m] ?? 0) > 0.000_001);
  }, [portfolio]);

  const resolved = useResolvedPrices(priceMints);

  const prices = useMemo(() => {
    const out: Record<string, number> = {};
    for (const [mint, r] of Object.entries(resolved)) out[mint] = r.priceUsd;
    return out;
  }, [resolved]);



  const { rows, totalUnrealized, today, week, allTime } = useMemo(() => {

    if (!portfolio) {

      return { rows: [], totalUnrealized: 0, today: 0, week: 0, allTime: 0 };

    }



    let unrealizedTotal = 0;

    const positionRows: PositionRow[] = [];



    for (const [mint, amount] of Object.entries(portfolio.balances)) {

      if (amount <= 0.000_001) continue;

      const px = prices[mint] ?? 0;

      const basis = portfolio.costBasisUsd[mint] ?? px;

      const uPnl = amount * (px - basis);

      unrealizedTotal += uPnl;

      positionRows.push({

        mint,

        pair: `${tokenSymbol(mint)}/USDC`,

        amount,

        entry: basis,

        pnl: uPnl,

      });

    }



    positionRows.sort((a, b) => Math.abs(b.pnl) - Math.abs(a.pnl));



    const now = Date.now();

    const dayStart = new Date();

    dayStart.setHours(0, 0, 0, 0);



    return {

      rows: positionRows,

      totalUnrealized: unrealizedTotal,

      today: periodPnl(portfolio.fills, dayStart.getTime()),

      week: periodPnl(portfolio.fills, now - 7 * 86_400_000),

      allTime: portfolio.realizedPnlUsd + unrealizedTotal,

    };

  }, [portfolio, prices]);



  const content = (

    <>

      <div className="grid grid-cols-[1fr_2.5rem_3rem_3.5rem] gap-1 px-2 py-1.5 text-[10px] uppercase tracking-[0.08em] text-ds-text-secondary border-b border-ds-border shrink-0 font-ui">

        <span>Pair</span>

        <span className="text-right">Size</span>

        <span className="text-right">Entry</span>

        <span className="text-right">PnL</span>

      </div>



      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">

        {rows.length === 0 ? (

          <div className="p-4 text-[11px] text-ds-text-muted text-center">No open positions</div>

        ) : (

          rows.map((row) => {

            const pnlClass =

              row.pnl > 0

                ? 'text-ds-green'

                : row.pnl < 0

                  ? 'text-ds-red'

                  : 'text-ds-text-secondary';

            return (

              <div

                key={row.mint}

                className="relative grid grid-cols-[1fr_2.5rem_3rem_3.5rem] gap-1 px-2 h-7 items-center text-[11px] font-mono tabular-nums border-b border-ds-border hover:bg-ds-elevated group"

                onMouseEnter={() => setHoveredMint(row.mint)}

                onMouseLeave={() => setHoveredMint(null)}

              >

                <span className="text-ds-text-primary font-ui truncate">{row.pair}</span>

                <span className="text-right text-ds-text-primary">{formatSize(row.amount)}</span>

                <span className="text-right text-ds-text-primary">{formatPrice(row.entry)}</span>

                <span className={`text-right ${pnlClass}`}>{formatPnL(row.pnl)}</span>

                {hoveredMint === row.mint && (

                  <button

                    type="button"

                    className="absolute right-1 text-ds-text-muted hover:text-ds-red text-[10px]"

                    aria-label="Close position"

                  >

                    ✕

                  </button>

                )}

              </div>

            );

          })

        )}

      </div>



      {rows.length > 0 && (

        <div className="grid grid-cols-[1fr_2.5rem_3rem_3.5rem] gap-1 px-2 py-2 border-t border-ds-border text-[11px] font-mono font-medium tabular-nums shrink-0">

          <span className="text-ds-text-primary font-ui">TOTAL</span>

          <span />

          <span />

          <span

            className={`text-right ${

              totalUnrealized > 0

                ? 'text-ds-green'

                : totalUnrealized < 0

                  ? 'text-ds-red'

                  : 'text-ds-text-secondary'

            }`}

          >

            {formatPnL(totalUnrealized)}

          </span>

        </div>

      )}



      <PnLSummary today={today} week={week} allTime={allTime} />

    </>

  );



  if (embedded) {

    return <div className="flex flex-col h-full min-h-0">{content}</div>;

  }



  return (

    <aside className="flex flex-col min-h-0 w-[320px] shrink-0 border-l border-ds-border bg-ds-surface">

      <div className="terminal-panel-header justify-between shrink-0">

        <span>Positions</span>

        <span className="font-mono normal-case tracking-normal text-ds-text-secondary">

          {rows.length} open

        </span>

      </div>

      {content}

    </aside>

  );

}



interface PositionRow {

  mint: string;

  pair: string;

  amount: number;

  entry: number;

  pnl: number;

}

