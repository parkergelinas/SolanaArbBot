'use client';

import type { SignalStats } from '@/lib/signals';
import { formatPct } from '@/lib/types';

interface SignalStatsBarProps {
  stats: SignalStats;
  connected: boolean;
}

function StatPill({
  label,
  value,
  accent,
}: {
  label: string;
  value: string | number;
  accent?: string;
}) {
  return (
    <div className="surface-card px-4 py-3 min-w-[7rem]">
      <p className="text-[10px] uppercase tracking-widest text-platform-muted mb-1">{label}</p>
      <p className="text-lg font-semibold mono tabular-nums" style={{ color: accent ?? '#f1f5f9' }}>
        {value}
      </p>
    </div>
  );
}

export default function SignalStatsBar({ stats, connected }: SignalStatsBarProps) {
  return (
    <div className="flex flex-wrap items-stretch gap-3">
      <StatPill
        label="Live stream"
        value={connected ? stats.liveCount : '—'}
        accent={connected ? '#00dfa8' : '#8b95a8'}
      />
      <StatPill label="Total" value={stats.total} />
      <StatPill label="Whale" value={stats.whale} accent="#4da3ff" />
      <StatPill label="Smart $" value={stats.smartMoney} accent="#a78bfa" />
      <StatPill label="Momentum" value={stats.momentum} accent="#00dfa8" />
      <StatPill label="Avg strength" value={formatPct(stats.avgStrength)} />
      <StatPill label="Avg confidence" value={formatPct(stats.avgConfidence)} />
      <div className="surface-card px-4 py-3 flex items-center gap-3 ml-auto">
        <span
          className={`w-2 h-2 rounded-full shrink-0 ${
            connected ? 'bg-platform-accent live-pulse' : 'bg-red-500'
          }`}
        />
        <div>
          <p className="text-[10px] uppercase tracking-widest text-platform-muted">Feed</p>
          <p className={`text-sm font-medium ${connected ? 'text-platform-accent' : 'text-red-400'}`}>
            {connected ? 'Connected' : 'Disconnected'}
          </p>
        </div>
      </div>
    </div>
  );
}
