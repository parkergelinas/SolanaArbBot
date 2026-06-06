'use client';

import { Home, LineChart, MoreHorizontal, Radio, Receipt } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useEffect, useState } from 'react';

import NetworkSwitcher from '@/components/wallet/NetworkSwitcher';
import WalletConnectButton from '@/components/wallet/WalletConnectButton';
import { isTerminalDemoEnabled } from '@/lib/config/env';
import { APP_ROUTES } from '@/lib/nav/routes';
import { useStreamStore } from '@/stores/streamStore';

function isActive(pathname: string, href: string): boolean {
  if (href === '/') return pathname === '/';
  return pathname === href || pathname.startsWith(`${href}/`);
}

const MOBILE_PRIMARY = [
  { href: '/terminal', label: 'Terminal', icon: LineChart },
  { href: '/', label: 'Overview', icon: Home },
  { href: '/signals', label: 'Signals', icon: Radio },
  { href: '/trades', label: 'Trades', icon: Receipt },
] as const;

const MOBILE_MORE = APP_ROUTES.filter(
  (r) => !MOBILE_PRIMARY.some((p) => p.href === r.href),
);

function useFeedStatus() {
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

  return { feedLabel, feedClass };
}

export function AppTopNav({ hideOnMobileTerminal = false }: { hideOnMobileTerminal?: boolean }) {
  const pathname = usePathname();
  const { feedLabel, feedClass } = useFeedStatus();

  return (
    <header
      className={`items-center gap-2 md:gap-3 h-11 md:h-9 px-3 shrink-0 border-b bg-ds-surface border-ds-border pt-[var(--safe-top)] ${
        hideOnMobileTerminal ? 'hidden lg:flex' : 'flex'
      }`}
    >
      <Link
        href="/terminal"
        className="text-[12px] font-semibold tracking-[0.12em] shrink-0 text-ds-text-primary hover:text-ds-blue transition-colors"
      >
        SOLARB
      </Link>

      <nav className="hidden md:flex items-center gap-0.5 flex-1 min-w-0 overflow-x-auto terminal-scroll">
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
        className={`inline-flex md:hidden px-1.5 py-px text-[9px] font-mono uppercase border rounded-terminal shrink-0 ${feedClass}`}
        title="Market stream status"
      >
        {feedLabel}
      </span>

      <span
        className={`hidden md:inline px-1.5 py-px text-[9px] font-mono uppercase border rounded-terminal shrink-0 ${feedClass}`}
        title="Market stream status"
      >
        {feedLabel}
      </span>

      <div className="flex items-center gap-1.5 md:gap-2 shrink-0 ml-auto md:ml-0">
        <NetworkSwitcher compact />
        <WalletConnectButton />
      </div>
    </header>
  );
}

export function AppMobileNav({ hideOnTerminal = false }: { hideOnTerminal?: boolean }) {
  const pathname = usePathname();
  const [moreOpen, setMoreOpen] = useState(false);
  const moreActive = MOBILE_MORE.some((r) => isActive(pathname, r.href));

  useEffect(() => {
    setMoreOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!moreOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMoreOpen(false);
    };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [moreOpen]);

  if (hideOnTerminal) return null;

  return (
    <>
      <nav
        className="mobile-bottom-nav md:hidden grid grid-cols-5 shrink-0 border-t border-ds-border bg-ds-surface z-50"
        aria-label="Primary navigation"
      >
        {MOBILE_PRIMARY.map(({ href, label, icon: Icon }) => {
          const active = isActive(pathname, href);
          return (
            <Link
              key={href}
              href={href}
              className={`touch-target flex flex-col items-center justify-center gap-0.5 px-1 py-1.5 transition-colors ${
                active ? 'text-ds-blue' : 'text-ds-text-secondary'
              }`}
            >
              <Icon className="w-[18px] h-[18px]" strokeWidth={active ? 2.25 : 1.75} aria-hidden />
              <span className="text-[9px] font-medium truncate max-w-full">{label}</span>
            </Link>
          );
        })}

        <button
          type="button"
          onClick={() => setMoreOpen(true)}
          className={`touch-target flex flex-col items-center justify-center gap-0.5 px-1 py-1.5 transition-colors ${
            moreActive ? 'text-ds-blue' : 'text-ds-text-secondary'
          }`}
          aria-expanded={moreOpen}
          aria-haspopup="dialog"
        >
          <MoreHorizontal className="w-[18px] h-[18px]" strokeWidth={moreActive ? 2.25 : 1.75} aria-hidden />
          <span className="text-[9px] font-medium">More</span>
        </button>
      </nav>

      {moreOpen && (
        <>
          <button
            type="button"
            className="md:hidden fixed inset-0 z-[9998] bg-black/55"
            aria-label="Close menu"
            onClick={() => setMoreOpen(false)}
          />
          <div
            role="dialog"
            aria-modal="true"
            aria-label="More pages"
            className="md:hidden fixed inset-x-0 bottom-0 z-[9999] rounded-t-md border-t border-ds-border bg-ds-surface shadow-2xl shadow-black/50"
            style={{ paddingBottom: 'max(0.75rem, var(--safe-bottom))' }}
          >
            <div className="flex items-center justify-between px-4 py-3 border-b border-ds-border">
              <span className="text-[12px] font-semibold uppercase tracking-[0.1em] text-ds-text-primary">
                More
              </span>
              <button
                type="button"
                onClick={() => setMoreOpen(false)}
                className="touch-target text-[11px] font-medium text-ds-text-secondary"
              >
                Close
              </button>
            </div>
            <div className="py-1">
              {MOBILE_MORE.map(({ href, label }) => {
                const active = isActive(pathname, href);
                return (
                  <Link
                    key={href}
                    href={href}
                    className={`touch-target flex items-center px-4 py-3.5 text-[14px] font-medium transition-colors ${
                      active
                        ? 'text-ds-blue bg-ds-blue/8'
                        : 'text-ds-text-primary hover:bg-ds-elevated/70'
                    }`}
                  >
                    {label}
                  </Link>
                );
              })}
            </div>
          </div>
        </>
      )}
    </>
  );
}

export default function AppNav() {
  return (
    <>
      <AppTopNav />
      <AppMobileNav />
    </>
  );
}
