'use client';

import { useTrackedWallets } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';

const TIER_STYLE: Record<string, string> = {
  whale: 'text-platform-accent bg-platform-accent/10',
  smart: 'text-purple-400 bg-purple-500/10',
  active: 'text-blue-400 bg-blue-500/10',
  retail: 'text-platform-muted bg-platform-elevated/50',
};

export default function WalletRail() {
  const wallets = useTrackedWallets();

  return (
    <aside className="glass-card w-full lg:w-56 shrink-0 flex flex-col min-h-0 overflow-hidden">
      <div className="px-3 py-2.5 border-b border-platform-border/80">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted font-semibold">
          Wallet Tracker
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto p-2 space-y-1.5 max-h-80 lg:max-h-none">
        {wallets.length === 0 ? (
          <li className="text-[10px] text-platform-muted text-center py-8">Awaiting wallet data</li>
        ) : (
          wallets.map((w) => (
            <li
              key={w.wallet}
              className="rounded-lg border border-platform-border/50 bg-platform-bg/60 px-2.5 py-2 hover:border-platform-accent/30 transition-colors"
            >
              <div className="flex justify-between items-center text-[10px]">
                <span className={`px-1.5 py-0.5 rounded font-semibold uppercase ${TIER_STYLE[w.tier] ?? TIER_STYLE.retail}`}>
                  {w.tier}
                </span>
                <span className="mono text-platform-muted">{w.swap_count} tx</span>
              </div>
              <p className="text-[10px] mono text-slate-300 truncate mt-1">{shortPool(w.wallet, 6, 4)}</p>
              <div className="flex justify-between mt-1 text-[10px] mono">
                <span className="text-platform-muted">{w.volume_sol_24h.toFixed(1)} SOL</span>
                <span className={w.net_flow_sol >= 0 ? 'text-green-400' : 'text-red-400'}>
                  {w.net_flow_sol >= 0 ? '+' : ''}{w.net_flow_sol.toFixed(1)}
                </span>
              </div>
            </li>
          ))
        )}
      </ul>
    </aside>
  );
}
