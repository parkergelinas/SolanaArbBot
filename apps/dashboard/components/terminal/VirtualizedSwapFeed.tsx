'use client';

import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore, amountToHuman } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

const VISIBLE_ROWS = 24;

function formatAmount(mint: string, raw: string): string {
  const n = amountToHuman(mint, raw);
  if (n >= 1_000) return n.toFixed(0);
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

export default function VirtualizedSwapFeed() {
  const swaps = useMarketStore((s) => s.swaps);
  const paused = useUiStore((s) => s.swapTapePaused);
  const setPaused = useUiStore((s) => s.setSwapTapePaused);

  const visible = paused ? swaps.slice(-VISIBLE_ROWS) : swaps.slice(-VISIBLE_ROWS);
  const ordered = [...visible].reverse();

  return (
    <section className="flex flex-col flex-shrink-0 h-44 min-h-0 border-t border-terminal-border bg-terminal-panel">
      <div className="px-2 py-1 border-b border-terminal-border flex justify-between items-center">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Live Swaps
        </span>
        <button
          type="button"
          onClick={() => setPaused(!paused)}
          className="text-[10px] text-terminal-muted hover:text-terminal-live"
        >
          {paused ? 'Resume' : 'Pause'}
        </button>
      </div>

      <div className="grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 py-0.5 text-[9px] uppercase text-terminal-muted border-b border-terminal-border shrink-0">
        <span>Time</span>
        <span>DEX</span>
        <span>Pair</span>
        <span className="text-right">In</span>
        <span className="text-right">Out</span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {ordered.length === 0 ? (
          <div className="p-4 text-center text-[11px] text-terminal-muted">No swaps yet</div>
        ) : (
          ordered.map((s) => {
            const isBuy = s.token_out.startsWith('So1111') === false;
            return (
              <div
                key={s.signature}
                className="grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 h-[22px] mono text-[10px] border-b border-terminal-border/40 hover:bg-terminal-hover"
              >
                <span className="text-terminal-muted truncate leading-[22px]">
                  {new Date(s.timestamp_ms).toLocaleTimeString()}
                </span>
                <span className="text-slate-400 truncate leading-[22px]">{s.dex}</span>
                <span
                  className={`truncate leading-[22px] ${
                    isBuy ? 'text-flow-buy' : 'text-flow-sell'
                  }`}
                >
                  {tokenSymbol(s.token_in)}→{tokenSymbol(s.token_out)}
                </span>
                <span className="text-right text-slate-500 truncate leading-[22px]">
                  {formatAmount(s.token_in, s.amount_in)}
                </span>
                <span className="text-right text-slate-500 truncate leading-[22px]">
                  {formatAmount(s.token_out, s.amount_out)}
                </span>
              </div>
            );
          })
        )}
      </div>
    </section>
  );
}
