'use client';

import { useTrackedWallets } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';

const TIER_STYLE: Record<string, string> = {
  whale: 'text-ds-green bg-ds-green/10 border-ds-green/25',
  smart: 'text-purple-400 bg-purple-500/10 border-purple-500/25',
  active: 'text-ds-blue bg-ds-blue/10 border-ds-blue/25',
  retail: 'text-ds-text-muted bg-ds-elevated/50 border-ds-border',
};

export default function WalletRail({
  fillHeight = false,
  compact = false,
}: {
  fillHeight?: boolean;
  compact?: boolean;
}) {
  const wallets = useTrackedWallets();

  return (
    <aside
      className={`bg-ds-surface flex flex-col overflow-hidden ${
        fillHeight
          ? 'h-full min-h-0 rounded-none border-0'
          : compact
            ? 'h-64 border border-ds-border rounded-terminal w-full'
            : 'min-h-64 border border-ds-border rounded-terminal w-full lg:w-56 shrink-0'
      }`}
    >
      <div className="px-3 py-1.5 border-b border-ds-border shrink-0">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Wallet Tracker
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto terminal-scroll p-1.5 space-y-1 min-h-0">
        {wallets.length === 0 ? (
          <li className="text-[10px] text-ds-text-muted text-center py-6">Awaiting wallet data</li>
        ) : (
          wallets.map((w) => (
            <li
              key={w.wallet}
              className="rounded-terminal border border-ds-border/60 bg-ds-elevated/30 px-2 py-1.5 hover:border-ds-blue/30 transition-colors"
            >
              <div className="flex justify-between items-center text-[9px]">
                <span
                  className={`px-1 py-px rounded-terminal font-semibold uppercase border ${
                    TIER_STYLE[w.tier] ?? TIER_STYLE.retail
                  }`}
                >
                  {w.tier}
                </span>
                <span className="font-mono text-ds-text-muted">{w.swap_count} tx</span>
              </div>
              <p className="text-[9px] font-mono text-ds-text-secondary truncate mt-0.5">
                {shortPool(w.wallet, 6, 4)}
              </p>
              <div className="flex justify-between mt-0.5 text-[9px] font-mono">
                <span className="text-ds-text-muted">{w.volume_sol_24h.toFixed(1)} SOL</span>
                <span className={w.net_flow_sol >= 0 ? 'text-ds-green' : 'text-ds-red'}>
                  {w.net_flow_sol >= 0 ? '+' : ''}
                  {w.net_flow_sol.toFixed(1)}
                </span>
              </div>
            </li>
          ))
        )}
      </ul>
    </aside>
  );
}
