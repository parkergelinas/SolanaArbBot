'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useStreamStore } from '@/stores/streamStore';
import { useWsContext } from './WebSocketProvider';

const NAV = [
  { href: '/bot',          label: 'Bot',       icon: '◉' },
  { href: '/terminal',     label: 'Terminal',  icon: '▣' },
  { href: '/',             label: 'Overview',  icon: '◈' },
  { href: '/signals',      label: 'Signals',   icon: '⚡' },
  { href: '/trades',       label: 'Trades',    icon: '↔' },
  { href: '/backtests',    label: 'Backtests', icon: '⏪' },
  { href: '/config',       label: 'Config',    icon: '⚙' },
  { href: '/risk',         label: 'Risk',      icon: '🛡' },
] as const;

export default function Navigation() {
  const pathname = usePathname();
  const { connected: controlConnected } = useWsContext();
  const streamConnected = useStreamStore((s) => s.connected);
  const onTerminal = pathname.startsWith('/terminal');
  const connected = onTerminal ? streamConnected : controlConnected;

  return (
    <nav className="w-56 flex-shrink-0 bg-slate-800 border-r border-slate-700 flex flex-col">
      {/* Logo */}
      <div className="px-4 py-5 border-b border-slate-700">
        <span className="text-green-400 font-bold text-sm tracking-wider">SOLANA ARB</span>
        <span className="block text-slate-400 text-xs mt-0.5">Control Plane</span>
      </div>

      {/* Links */}
      <ul className="flex-1 py-3 space-y-0.5 px-2">
        {NAV.map(({ href, label, icon }) => {
          const active = pathname === href || (href === '/terminal' && pathname.startsWith('/terminal'));
          return (
            <li key={href}>
              <Link
                href={href}
                className={`flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors ${
                  active
                    ? 'bg-green-500/10 text-green-400 font-medium'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-700/50'
                }`}
              >
                <span className="w-4 text-center">{icon}</span>
                {label}
              </Link>
            </li>
          );
        })}
      </ul>

      {/* WebSocket status */}
      <div className="px-4 py-3 border-t border-slate-700">
        <div className="flex items-center gap-2">
          <span
            className={`w-2 h-2 rounded-full ${connected ? 'bg-green-400 live-pulse' : 'bg-red-500'}`}
          />
          <span className="text-xs text-slate-400">
            {connected ? 'Stream live' : 'Disconnected'}
          </span>
        </div>
      </div>
    </nav>
  );
}
