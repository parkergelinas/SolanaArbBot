'use client';

import { useConnected, useWhales, useSwaps } from '@/lib/hooks';

export default function Header() {
  const connected = useConnected();
  const whales = useWhales();
  const swaps = useSwaps();

  return (
    <header className="flex items-center justify-between px-4 py-3 border-b border-platform-border bg-platform-surface">
      <div>
        <h1 className="text-sm font-bold tracking-[0.2em] text-platform-accent">
          SOL INTEL
        </h1>
        <p className="text-[10px] text-platform-muted uppercase tracking-wider">
          Whale · Smart Money · Live Flow
        </p>
      </div>
      <div className="flex items-center gap-4 text-xs mono">
        <span className="text-platform-muted">{swaps.length} txs</span>
        <span className="text-platform-accent">{whales.length} whales</span>
        <span className="flex items-center gap-1.5">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              connected ? 'bg-platform-accent animate-pulse' : 'bg-red-500'
            }`}
          />
          {connected ? 'LIVE' : 'OFF'}
        </span>
      </div>
    </header>
  );
}
