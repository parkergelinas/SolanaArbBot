'use client';

import { useIntelSwaps } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';

export default function TxTape() {
  const swaps = useIntelSwaps().slice().reverse().slice(0, 24);

  return (
    <section className="glass-card flex flex-col min-h-0 overflow-hidden">
      <div className="px-3 py-2.5 border-b border-platform-border/80">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted font-semibold">
          Live Swap Tape
        </span>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-[10px] mono">
          <thead>
            <tr className="text-platform-muted border-b border-platform-border/50">
              <th className="text-left px-2 py-1.5 font-medium">Time</th>
              <th className="text-left px-2 py-1.5 font-medium">DEX</th>
              <th className="text-right px-2 py-1.5 font-medium">SOL</th>
              <th className="text-left px-2 py-1.5 font-medium">Wallet</th>
            </tr>
          </thead>
          <tbody>
            {swaps.length === 0 ? (
              <tr>
                <td colSpan={4} className="text-center text-platform-muted py-6">
                  No swaps yet
                </td>
              </tr>
            ) : (
              swaps.map((s) => (
                <tr
                  key={s.signature}
                  className="border-b border-platform-border/30 hover:bg-platform-elevated/40"
                >
                  <td className="px-2 py-1.5 text-platform-muted">
                    {new Date(s.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                  </td>
                  <td className="px-2 py-1.5 text-slate-300">{s.dex}</td>
                  <td className="px-2 py-1.5 text-right text-platform-accent">
                    {s.amount_sol.toFixed(2)}
                  </td>
                  <td className="px-2 py-1.5 text-platform-muted">{shortPool(s.wallet, 4, 4)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
