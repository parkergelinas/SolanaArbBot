'use client';

import type { SignalStats } from '@/lib/signals';
import { formatPct } from '@/lib/types';

interface SignalStatsBarProps {
  stats: SignalStats;
  connected: boolean;
}

function StatCell({
  label,
  value,
  className = '',
}: {
  label: string;
  value: string | number;
  className?: string;
}) {
  return (
    <div className="flex flex-col justify-center px-3 py-1.5 border-r border-ds-border last:border-r-0 min-w-[4.5rem]">
      <span className="text-[8px] uppercase tracking-[0.14em] text-ds-text-muted leading-none">
        {label}
      </span>
      <span className={`text-[13px] font-mono font-medium tabular-nums leading-tight mt-0.5 ${className}`}>
        {value}
      </span>
    </div>
  );
}

export default function SignalStatsBar({ stats, connected }: SignalStatsBarProps) {
  return (
    <div className="flex items-stretch shrink-0 bg-ds-surface border border-ds-border rounded-terminal overflow-x-auto terminal-scroll">
      <StatCell
        label="Live"
        value={connected ? stats.liveCount : '—'}
        className={connected ? 'text-ds-green' : 'text-ds-text-muted'}
      />
      <StatCell label="Total" value={stats.total} className="text-ds-text-primary" />
      <StatCell label="Whale" value={stats.whale} className="text-ds-blue" />
      <StatCell label="Smart $" value={stats.smartMoney} className="text-purple-400" />
      <StatCell label="Mom" value={stats.momentum} className="text-ds-green" />
      <StatCell label="Avg str" value={formatPct(stats.avgStrength)} className="text-ds-text-secondary" />
      <StatCell label="Avg conf" value={formatPct(stats.avgConfidence)} className="text-ds-text-secondary" />
      <div className="flex items-center gap-2 px-3 py-1.5 ml-auto shrink-0 border-l border-ds-border">
        <span
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${
            connected ? 'bg-ds-green live-pulse' : 'bg-ds-red'
          }`}
        />
        <span className={`text-[10px] font-mono uppercase tracking-wider ${connected ? 'text-ds-green' : 'text-ds-red'}`}>
          {connected ? 'Feed live' : 'Offline'}
        </span>
      </div>
    </div>
  );
}
