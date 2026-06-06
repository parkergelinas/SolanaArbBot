'use client';

import Link from 'next/link';

import NetworkSwitcher from '@/components/wallet/NetworkSwitcher';
import WalletConnectButton from '@/components/wallet/WalletConnectButton';
import { formatChangePct, formatPrice } from '@/lib/formatters';
import { useSelectedPairPrice } from '@/lib/hooks/useResolvedPrice';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

const MODE: Record<ConnectionMode, { dot: string }> = {
  live: { dot: 'bg-ds-green' },
  degraded: { dot: 'bg-ds-amber' },
  sim: { dot: 'bg-ds-text-muted' },
};

export default function TerminalMobileHeader() {
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const resolved = useSelectedPairPrice();
  const symbol = selectedMint ? tokenSymbol(selectedMint) : 'SOL';
  const up = resolved.changeH24Pct >= 0;
  const mode = MODE[connectionMode];

  return (
    <header className="lg:hidden flex items-center gap-2 px-3 h-12 shrink-0 border-b border-ds-border bg-ds-surface pt-[var(--safe-top)]">
      <Link
        href="/"
        className="touch-target flex flex-col items-center justify-center shrink-0 w-10 text-ds-text-secondary hover:text-ds-text-primary"
        aria-label="Back to overview"
      >
        <span className="text-[16px] leading-none">‹</span>
        <span className="text-[8px] uppercase tracking-wider">App</span>
      </Link>

      <div className="flex items-center gap-2 min-w-0 flex-1">
        <span className={`w-2 h-2 rounded-full shrink-0 ${mode.dot}`} title={connectionMode} />
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2 min-w-0">
            <span className="text-[13px] font-semibold text-ds-text-primary shrink-0">{symbol}</span>
            <span className="text-[17px] font-mono font-medium tabular-nums text-ds-text-primary leading-none truncate">
              ${formatPrice(resolved.priceUsd)}
            </span>
          </div>
          <div className="flex items-center gap-2 mt-0.5">
            <span
              className={`text-[11px] font-mono tabular-nums ${up ? 'text-ds-green' : 'text-ds-red'}`}
            >
              {formatChangePct(resolved.changeH24Pct)}
            </span>
            {resolved.stale && (
              <span className="text-[8px] text-ds-amber uppercase tracking-wider">stale</span>
            )}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <NetworkSwitcher compact />
        <WalletConnectButton />
      </div>
    </header>
  );
}
