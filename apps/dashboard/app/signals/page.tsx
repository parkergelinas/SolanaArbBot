'use client';

import { useCallback, useState } from 'react';
import SignalCard from '@/components/SignalCard';
import { useFetch, useWsEvents } from '@/lib/hooks';
import { api } from '@/lib/api';
import type { SignalType } from '@/lib/types';

const SIGNAL_TYPES: (SignalType | 'All')[] = ['All', 'WhaleFlow', 'SmartMoney', 'Momentum'];

export default function SignalsPage() {
  const [filter, setFilter]     = useState<SignalType | 'All'>('All');
  const [expanded, setExpanded] = useState<number | null>(null);

  const fetcher = useCallback(
    () => api.signals({ limit: 200, signal_type: filter === 'All' ? undefined : filter }),
    [filter],
  );
  const { data: historical, loading } = useFetch(fetcher, 15_000);
  const liveSignals = useWsEvents('signal', 500);

  // Merge live + historical, deduplicate by signal_id
  const allSignals = [...liveSignals, ...(historical ?? [])].reduce(
    (acc, s) => (acc.some(x => x.signal_id === s.signal_id) ? acc : [...acc, s]),
    [] as typeof liveSignals,
  );

  const filtered = filter === 'All' ? allSignals : allSignals.filter(s => s.signal_type === filter);

  return (
    <div className="space-y-5 max-w-6xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">Signals</h1>
          <p className="text-slate-400 text-sm mt-0.5">Live signal feed with full explanation trace</p>
        </div>
        <span className="text-slate-500 text-sm">{filtered.length} signals</span>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-2">
        {SIGNAL_TYPES.map(t => (
          <button
            key={t}
            onClick={() => setFilter(t)}
            className={`px-3 py-1 rounded-lg text-sm transition-colors ${
              filter === t
                ? 'bg-green-500/15 text-green-400 border border-green-500/30'
                : 'text-slate-400 border border-slate-700 hover:border-slate-600 hover:text-slate-200'
            }`}
          >
            {t}
          </button>
        ))}
      </div>

      {/* Signal list */}
      {loading && filtered.length === 0 ? (
        <div className="text-slate-500 text-sm text-center py-12">Loading…</div>
      ) : filtered.length === 0 ? (
        <div className="text-slate-500 text-sm bg-slate-800 border border-slate-700 rounded-xl p-8 text-center">
          No signals match this filter.
        </div>
      ) : (
        <div className="space-y-3">
          {filtered.map((s, i) => (
            <div
              key={`${s.signal_id}-${i}`}
              onClick={() => setExpanded(expanded === s.signal_id ? null : s.signal_id)}
              className="cursor-pointer"
            >
              <SignalCard signal={s} expanded={expanded === s.signal_id} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
