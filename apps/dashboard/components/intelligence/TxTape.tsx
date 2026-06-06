'use client';

import { useIntelSwaps } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';

export default function TxTape() {
  const swaps = useIntelSwaps().slice().reverse().slice(0, 24);

  return (
    <section className="shrink-0 bg-ds-surface border border-ds-border rounded-terminal flex flex-col overflow-hidden max-h-[9.5rem]">
      <div className="px-3 py-1.5 border-b border-ds-border shrink-0 flex items-center justify-between">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Live Swap Tape
        </span>
        <span className="text-[9px] font-mono text-ds-text-muted">{swaps.length} rows</span>
      </div>
      <div className="flex-1 min-h-0 overflow-auto terminal-scroll">
        <table className="w-full text-[10px] font-mono">
          <thead className="sticky top-0 bg-ds-elevated border-b border-ds-border">
            <tr className="text-ds-text-muted">
              <th className="text-left px-2 py-1 font-medium text-[9px] uppercase tracking-wider">Time</th>
              <th className="text-left px-2 py-1 font-medium text-[9px] uppercase tracking-wider">DEX</th>
              <th className="text-right px-2 py-1 font-medium text-[9px] uppercase tracking-wider">SOL</th>
              <th className="text-left px-2 py-1 font-medium text-[9px] uppercase tracking-wider">Wallet</th>
            </tr>
          </thead>
          <tbody>
            {swaps.length === 0 ? (
              <tr>
                <td colSpan={4} className="text-center text-ds-text-muted py-4 text-[10px]">
                  No swaps yet — intelligence-api :8090
                </td>
              </tr>
            ) : (
              swaps.map((s) => (
                <tr
                  key={s.signature}
                  className="border-b border-ds-border/40 hover:bg-ds-elevated/40"
                >
                  <td className="px-2 py-1 text-ds-text-muted tabular-nums">
                    {new Date(s.timestamp).toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                      second: '2-digit',
                    })}
                  </td>
                  <td className="px-2 py-1 text-ds-text-secondary">{s.dex}</td>
                  <td className="px-2 py-1 text-right text-ds-green tabular-nums">
                    {s.amount_sol.toFixed(2)}
                  </td>
                  <td className="px-2 py-1 text-ds-text-muted">{shortPool(s.wallet, 4, 4)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
