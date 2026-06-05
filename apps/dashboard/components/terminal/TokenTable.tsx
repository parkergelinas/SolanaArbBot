'use client';

import { useMarketStore, shortMint } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

function flowColor(imb: number) {
  if (imb > 0.1) return 'text-flow-buy';
  if (imb < -0.1) return 'text-flow-sell';
  return 'text-terminal-muted';
}

export default function TokenTable() {
  const tokens = useMarketStore((s) => s.tokens);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const setSelectedMint = useUiStore((s) => s.setSelectedMint);

  const rows = Object.values(tokens).sort(
    (a, b) => b.timestamp_ms - a.timestamp_ms,
  );

  return (
    <section className="border-b border-terminal-border bg-terminal-panel flex flex-col min-h-0">
      <div className="px-2 py-1 border-b border-terminal-border flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Tokens
        </span>
        <span className="text-[10px] text-terminal-muted">{rows.length} active</span>
      </div>
      <div className="overflow-auto max-h-36">
        <table className="w-full text-[11px] mono">
          <thead className="sticky top-0 bg-terminal-panel text-terminal-muted">
            <tr className="border-b border-terminal-border">
              <th className="text-left px-2 py-1 font-normal">Mint</th>
              <th className="text-right px-2 py-1 font-normal">Price</th>
              <th className="text-right px-2 py-1 font-normal">Chg%</th>
              <th className="text-right px-2 py-1 font-normal">Volume</th>
              <th className="text-right px-2 py-1 font-normal">Flow</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-2 py-4 text-center text-terminal-muted">
                  Awaiting stream…
                </td>
              </tr>
            ) : (
              rows.map((row) => {
                const active = row.mint === selectedMint;
                const chgPos = row.changePct >= 0;
                return (
                  <tr
                    key={row.mint}
                    onClick={() => setSelectedMint(row.mint)}
                    className={`cursor-pointer border-b border-terminal-border/50 hover:bg-terminal-hover ${
                      active ? 'bg-terminal-live/10' : ''
                    }`}
                  >
                    <td className="px-2 py-0.5 text-terminal-live">{shortMint(row.mint, 5, 4)}</td>
                    <td className="px-2 py-0.5 text-right text-slate-200">
                      ${row.price_usd.toFixed(4)}
                    </td>
                    <td
                      className={`px-2 py-0.5 text-right ${
                        chgPos ? 'text-flow-buy' : 'text-flow-sell'
                      }`}
                    >
                      {chgPos ? '+' : ''}
                      {row.changePct.toFixed(2)}%
                    </td>
                    <td className="px-2 py-0.5 text-right text-slate-400">
                      {row.volume.toFixed(2)}
                    </td>
                    <td className={`px-2 py-0.5 text-right ${flowColor(row.flowImbalance)}`}>
                      {(row.flowImbalance * 100).toFixed(0)}%
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
