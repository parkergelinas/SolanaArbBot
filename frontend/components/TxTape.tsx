'use client';

import { useSwaps } from '@/lib/hooks';

export default function TxTape() {
  const swaps = [...useSwaps()].reverse().slice(0, 24);

  return (
    <section className="h-36 shrink-0 flex flex-col bg-platform-surface border border-platform-border rounded-xl overflow-hidden">
      <div className="px-3 py-1.5 border-b border-platform-border flex justify-between">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted">
          Live Transactions
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        <table className="w-full text-[10px] mono">
          <thead className="text-platform-muted sticky top-0 bg-platform-surface">
            <tr>
              <th className="text-left px-2 py-1 font-normal">Time</th>
              <th className="text-left px-2 py-1 font-normal">DEX</th>
              <th className="text-left px-2 py-1 font-normal">Wallet</th>
              <th className="text-right px-2 py-1 font-normal">SOL</th>
            </tr>
          </thead>
          <tbody>
            {swaps.map((s) => (
              <tr key={s.signature} className="border-t border-platform-border/40 hover:bg-platform-accent/5">
                <td className="px-2 py-0.5 text-platform-muted">
                  {new Date(s.timestamp).toLocaleTimeString()}
                </td>
                <td className="px-2 py-0.5 text-slate-300">{s.dex}</td>
                <td className="px-2 py-0.5 text-slate-400 truncate max-w-[8rem]">
                  {s.wallet.slice(0, 10)}
                </td>
                <td className="px-2 py-0.5 text-right text-platform-accent">
                  {s.amount_sol.toFixed(2)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
