'use client';

import { useWallets } from '@/lib/hooks';

const TIER_COLOR: Record<string, string> = {
  whale: 'text-platform-accent',
  smart: 'text-purple-400',
  active: 'text-blue-400',
  retail: 'text-platform-muted',
};

export default function WalletRail() {
  const wallets = useWallets();

  return (
    <aside className="w-56 shrink-0 flex flex-col min-h-0 bg-platform-surface border border-platform-border rounded-xl overflow-hidden">
      <div className="px-3 py-2 border-b border-platform-border">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted">
          Wallet Tracker
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto p-1 space-y-1">
        {wallets.length === 0 ? (
          <li className="text-[10px] text-platform-muted text-center py-6">No wallets yet</li>
        ) : (
          wallets.map((w) => (
            <li
              key={w.wallet}
              className="rounded-md border border-platform-border/60 bg-platform-bg px-2 py-1.5"
            >
              <div className="flex justify-between text-[10px]">
                <span className={TIER_COLOR[w.tier] ?? 'text-slate-300'}>{w.tier}</span>
                <span className="mono text-platform-muted">{w.swap_count} swaps</span>
              </div>
              <p className="text-[10px] mono text-slate-300 truncate mt-0.5">{w.wallet}</p>
              <p className="text-[10px] mono text-platform-muted mt-0.5">
                {w.volume_sol_24h.toFixed(1)} SOL vol
              </p>
            </li>
          ))
        )}
      </ul>
    </aside>
  );
}
