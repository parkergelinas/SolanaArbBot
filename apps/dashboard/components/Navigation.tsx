'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

import { useIntelConnected } from '@/lib/intelligence/hooks';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';

import { useWsContext } from './WebSocketProvider';
import WalletConnectButton from './wallet/WalletConnectButton';

const NAV = [
  { href: '/bot', label: 'Bot', icon: '◉' },
  { href: '/terminal', label: 'Terminal', icon: '▣' },
  { href: '/', label: 'Overview', icon: '◈' },
  { href: '/signals', label: 'Signals', icon: '⚡' },
  { href: '/trades', label: 'Trades', icon: '↔' },
  { href: '/backtests', label: 'Backtests', icon: '⏪' },
  { href: '/config', label: 'Config', icon: '⚙' },
  { href: '/risk', label: 'Risk', icon: '🛡' },
] as const;

function StatusDot({ live, label }: { live: boolean; label: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className={`w-1.5 h-1.5 rounded-full ${live ? 'bg-platform-accent live-pulse' : 'bg-red-500/80'}`} />
      <span className="text-[10px] text-platform-muted">{label}</span>
    </div>
  );
}

const MODE_DOT: Record<ConnectionMode, { dot: string; label: string }> = {
  live: { dot: 'bg-platform-accent live-pulse', label: 'Market · LIVE' },
  degraded: { dot: 'bg-amber-400', label: 'Market · DEGRADED' },
  sim: { dot: 'bg-slate-500', label: 'Market · SIM' },
};

function StreamModeDot({ mode, connected }: { mode: ConnectionMode; connected: boolean }) {
  const style = MODE_DOT[mode];
  const dotClass = connected ? style.dot : 'bg-red-500/80';
  const label = connected ? style.label : 'Market · offline';
  return (
    <div className="flex items-center gap-1.5">
      <span className={`w-1.5 h-1.5 rounded-full ${dotClass}`} />
      <span className="text-[10px] text-platform-muted">{label}</span>
    </div>
  );
}

export default function Navigation() {
  const pathname = usePathname();
  const { connected: controlConnected } = useWsContext();
  const streamConnected = useStreamStore((s) => s.connected);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const intelConnected = useIntelConnected();
  const onTerminal = pathname.startsWith('/terminal');

  return (
    <nav className="w-56 flex-shrink-0 bg-platform-surface/95 border-r border-platform-border flex flex-col backdrop-blur-sm">
      <div className="px-4 py-5 border-b border-platform-border">
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-platform-accent shadow-[0_0_8px_rgba(0,223,168,0.6)]" />
          <span className="text-platform-accent font-bold text-sm tracking-wider">SOLANA ARB</span>
        </div>
        <span className="block text-platform-muted text-[10px] mt-1 uppercase tracking-widest">
          Intelligence Terminal
        </span>
      </div>

      <ul className="flex-1 py-3 space-y-0.5 px-2">
        {NAV.map(({ href, label, icon }) => {
          const active =
            pathname === href || (href === '/terminal' && pathname.startsWith('/terminal'));
          return (
            <li key={href}>
              <Link
                href={href}
                className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all ${
                  active
                    ? 'bg-platform-accent/12 text-platform-accent font-medium shadow-sm border border-platform-accent/20'
                    : 'text-platform-muted hover:text-slate-100 hover:bg-platform-elevated/60'
                }`}
              >
                <span className="w-4 text-center opacity-80">{icon}</span>
                {label}
              </Link>
            </li>
          );
        })}
      </ul>

      <div className="px-3 py-3 border-t border-platform-border space-y-3">
        <WalletConnectButton />
        <div className="space-y-1.5 px-1">
          <StatusDot live={onTerminal ? streamConnected : controlConnected} label="Control" />
          {onTerminal ? (
            <StreamModeDot mode={connectionMode} connected={streamConnected} />
          ) : (
            <StatusDot live={streamConnected} label="Market" />
          )}
          <StatusDot live={intelConnected} label="Whale intel" />
        </div>
      </div>
    </nav>
  );
}
