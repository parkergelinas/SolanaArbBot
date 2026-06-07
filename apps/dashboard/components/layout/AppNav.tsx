'use client';

import { Home, LineChart, MoreHorizontal, Radio, Receipt, Shield, Bot, Settings } from 'lucide-react';
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
  { href: '/',        label: 'Overview', icon: Home },
  { href: '/signals', label: 'Signals',  icon: Radio },
  { href: '/trades',  label: 'Trades',   icon: Receipt },
] as const;

const MOBILE_MORE = APP_ROUTES.filter(
  (r) => !MOBILE_PRIMARY.some((p) => p.href === r.href),
);

function useFeedStatus() {
  const connected       = useStreamStore((s) => s.connected);
  const connectionMode  = useStreamStore((s) => s.connectionMode);
  const demo            = isTerminalDemoEnabled();

  const feedLabel = connected
    ? connectionMode.toUpperCase()
    : demo
      ? 'DEMO'
      : 'OFFLINE';

  const feedColor = connected
    ? connectionMode === 'live'
      ? 'var(--green)'
      : 'var(--amber)'
    : 'var(--text-muted)';

  const pillClass = connected
    ? connectionMode === 'live'
      ? 'border-[color-mix(in_srgb,var(--green)_28%,var(--bg-border))] text-[color:var(--green)]'
      : 'border-[color-mix(in_srgb,var(--amber)_28%,var(--bg-border))] text-[color:var(--amber)]'
    : 'border-[var(--bg-border)] text-[color:var(--text-muted)]';

  return { feedLabel, feedColor, pillClass, connected };
}

export function AppTopNav({ hideOnMobileTerminal = false }: { hideOnMobileTerminal?: boolean }) {
  const pathname = usePathname();
  const { feedLabel, feedColor, pillClass, connected } = useFeedStatus();

  return (
    <header
      className={`shrink-0 border-b border-[var(--bg-border)] bg-[var(--bg-surface)] pt-[var(--safe-top)] ${
        hideOnMobileTerminal ? 'hidden lg:flex' : 'flex'
      } items-center h-9`}
    >
      {/* ── Logo ────────────────────────────────────────────────────────────── */}
      <Link
        href="/terminal"
        className="flex items-center gap-1.5 px-3 shrink-0 group"
        aria-label="SOLARB — go to terminal"
      >
        <svg
          width="14" height="14" viewBox="0 0 14 14" fill="none"
          className="shrink-0 text-[color:var(--blue)] group-hover:text-[color:var(--text-primary)] transition-colors"
          aria-hidden
        >
          <path d="M7 1L13 4.5V9.5L7 13L1 9.5V4.5L7 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
          <path d="M7 4.5L9.5 6V9L7 10.5L4.5 9V6L7 4.5Z" fill="currentColor" />
        </svg>
        <span className="text-[11px] font-semibold tracking-[0.14em] text-[color:var(--text-primary)] group-hover:text-[color:var(--blue)] transition-colors">
          SOLARB
        </span>
      </Link>

      {/* ── Divider ─────────────────────────────────────────────────────────── */}
      <div className="w-px h-3.5 bg-[var(--bg-border)] mx-1 shrink-0" />

      {/* ── Desktop nav ─────────────────────────────────────────────────────── */}
      <nav
        className="hidden md:flex items-center flex-1 min-w-0 overflow-x-auto terminal-scroll h-full"
        aria-label="Main navigation"
      >
        {APP_ROUTES.map(({ href, label }) => {
          const active = isActive(pathname, href);
          return (
            <Link
              key={href}
              href={href}
              className={`relative h-full px-3 flex items-center text-[11px] font-medium whitespace-nowrap transition-colors duration-75 ${
                active
                  ? 'text-[color:var(--blue)]'
                  : 'text-[color:var(--text-muted)] hover:text-[color:var(--text-secondary)]'
              }`}
            >
              {label}
              {active && (
                <span
                  className="absolute bottom-0 inset-x-2 h-[2px] rounded-full"
                  style={{ background: 'var(--blue)' }}
                />
              )}
            </Link>
          );
        })}
      </nav>

      {/* ── Right cluster ───────────────────────────────────────────────────── */}
      <div className="flex items-center gap-1.5 shrink-0 ml-auto px-3">
        {/* Feed status — mobile only tiny version */}
        <span
          className={`inline-flex md:hidden items-center gap-1 px-1.5 py-0.5 text-[9px] font-mono uppercase tracking-[0.07em] border rounded-sm ${pillClass}`}
          title="Stream status"
        >
          <span
            className={`w-1.5 h-1.5 rounded-full shrink-0 ${connected ? 'live-pulse' : ''}`}
            style={{ backgroundColor: feedColor }}
          />
          {feedLabel}
        </span>

        {/* Feed status — desktop */}
        <span
          className={`hidden md:inline-flex items-center gap-1.5 px-2 h-5 text-[9px] font-mono uppercase tracking-[0.07em] border rounded-sm transition-colors ${pillClass}`}
          title="Market stream status"
        >
          <span
            className={`w-1.5 h-1.5 rounded-full shrink-0 ${connected ? 'live-pulse' : ''}`}
            style={{ backgroundColor: feedColor }}
          />
          {feedLabel}
        </span>

        <div className="hidden md:block w-px h-3.5 bg-[var(--bg-border)]" />

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

  useEffect(() => { setMoreOpen(false); }, [pathname]);

  useEffect(() => {
    if (!moreOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setMoreOpen(false); };
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
        className="mobile-bottom-nav md:hidden grid grid-cols-5 shrink-0 border-t border-[var(--bg-border)] bg-[var(--bg-surface)] z-50"
        aria-label="Primary navigation"
      >
        {MOBILE_PRIMARY.map(({ href, label, icon: Icon }) => {
          const active = isActive(pathname, href);
          return (
            <Link
              key={href}
              href={href}
              className={`touch-target flex flex-col items-center justify-center gap-0.5 px-1 py-1.5 transition-colors ${
                active ? 'text-[color:var(--blue)]' : 'text-[color:var(--text-secondary)]'
              }`}
            >
              <Icon
                className="w-[18px] h-[18px]"
                strokeWidth={active ? 2.25 : 1.75}
                aria-hidden
              />
              <span className="text-[9px] font-medium truncate max-w-full">{label}</span>
            </Link>
          );
        })}

        <button
          type="button"
          onClick={() => setMoreOpen(true)}
          className={`touch-target flex flex-col items-center justify-center gap-0.5 px-1 py-1.5 transition-colors ${
            moreActive ? 'text-[color:var(--blue)]' : 'text-[color:var(--text-secondary)]'
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
            className="md:hidden fixed inset-0 z-[9998] bg-black/60"
            aria-label="Close menu"
            onClick={() => setMoreOpen(false)}
          />
          <div
            role="dialog"
            aria-modal="true"
            aria-label="More pages"
            className="md:hidden fixed inset-x-0 bottom-0 z-[9999] rounded-t-[4px] border-t border-[var(--bg-border)] bg-[var(--bg-surface)] shadow-[0_-8px_32px_rgba(0,0,0,0.6)]"
            style={{ paddingBottom: 'max(0.75rem, var(--safe-bottom))' }}
          >
            <div className="flex items-center justify-between px-4 py-2.5 border-b border-[var(--bg-border)]">
              <span className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[color:var(--text-primary)]">
                Navigation
              </span>
              <button
                type="button"
                onClick={() => setMoreOpen(false)}
                className="touch-target flex items-center justify-center text-[10px] font-medium text-[color:var(--text-muted)] hover:text-[color:var(--text-secondary)] transition-colors"
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
                    className={`touch-target flex items-center px-4 py-3 text-[13px] font-medium transition-colors ${
                      active
                        ? 'text-[color:var(--blue)] bg-[color-mix(in_srgb,var(--blue)_8%,transparent)]'
                        : 'text-[color:var(--text-primary)] hover:bg-[var(--bg-elevated)]'
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
