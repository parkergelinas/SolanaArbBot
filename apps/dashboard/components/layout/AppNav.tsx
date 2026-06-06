'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

import { APP_ROUTES } from '@/lib/nav/routes';
import { isTerminalDemoEnabled } from '@/lib/config/env';
import { useStreamStore } from '@/stores/streamStore';
import NetworkSwitcher from '@/components/wallet/NetworkSwitcher';
import WalletConnectButton from '@/components/wallet/WalletConnectButton';

function isActive(pathname: string, href: string): boolean {
  if (href === '/') return pathname === '/';
  return pathname === href || pathname.startsWith(`${href}/`);
}

export default function AppNav() {
  const pathname = usePathname();
  const connected = useStreamStore((s) => s.connected);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const demo = isTerminalDemoEnabled();

  const feedLabel = connected
    ? connectionMode.toUpperCase()
    : demo
      ? 'DEMO'
      : 'OFFLINE';

  const feedClass = connected
    ? connectionMode === 'live'
      ? 'text-ds-green border-ds-green/30'
      : 'text-ds-amber border-ds-amber/30'
    : 'text-ds-text-muted border-ds-border';

  return (
    <header className="flex items-center gap-3 h-9 px-3 shrink-0 border-b bg-ds-surface border-ds-border">
      <Link
        href="/terminal"
        className="text-[12px] font-semibold tracking-[0.12em] shrink-0 text-ds-text-primary hover:text-ds-blue transition-colors"
      >
        SOLARB
      </Link>

      <nav className="flex items-center gap-0.5 flex-1 min-w-0 overflow-x-auto terminal-scroll">
        {APP_ROUTES.map(({ href, label }) => {
          const active = isActive(pathname, href);
          return (
            <Link
              key={href}
              href={href}
              className={`px-2.5 py-1 text-[11px] font-medium whitespace-nowrap rounded-terminal transition-colors ${
                active
                  ? 'bg-ds-elevated text-ds-blue'
                  : 'text-ds-text-secondary hover:text-ds-text-primary hover:bg-ds-elevated/60'
              }`}
            >
              {label}
            </Link>
          );
        })}
      </nav>

      <span
        className={`hidden sm:inline px-1.5 py-px text-[9px] font-mono uppercase border rounded-terminal shrink-0 ${feedClass}`}
        title="Market stream status"
      >
        {feedLabel}
      </span>

      <div className="flex items-center gap-2 shrink-0">
        <NetworkSwitcher compact />
        <WalletConnectButton />
      </div>
    </header>
  );
}
