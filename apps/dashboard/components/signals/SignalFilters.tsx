'use client';

import type { SignalFilterDirection, SignalFilterType, SignalSortKey } from '@/lib/signals';
import type { Direction, SignalType } from '@/lib/types';

const TYPES: SignalFilterType[] = ['All', 'WhaleFlow', 'SmartMoney', 'Momentum'];
const DIRECTIONS: SignalFilterDirection[] = ['All', 'Long', 'Short', 'Neutral'];
const SORTS: { key: SignalSortKey; label: string }[] = [
  { key: 'time', label: 'Latest' },
  { key: 'strength', label: 'Strength' },
  { key: 'confidence', label: 'Confidence' },
];

const TYPE_LABELS: Record<SignalFilterType, string> = {
  All: 'All',
  WhaleFlow: 'Whale',
  SmartMoney: 'Smart $',
  Momentum: 'Momentum',
};

interface SignalFiltersProps {
  type: SignalFilterType;
  direction: SignalFilterDirection;
  sort: SignalSortKey;
  count: number;
  onType: (t: SignalFilterType) => void;
  onDirection: (d: SignalFilterDirection) => void;
  onSort: (s: SignalSortKey) => void;
}

export default function SignalFilters({
  type,
  direction,
  sort,
  count,
  onType,
  onDirection,
  onSort,
}: SignalFiltersProps) {
  return (
    <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
      <div className="flex flex-wrap items-center gap-3">
        <div className="segment-control">
          {TYPES.map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => onType(t)}
              className={`segment-btn ${type === t ? 'segment-btn-active' : ''}`}
            >
              {TYPE_LABELS[t]}
            </button>
          ))}
        </div>

        <div className="segment-control">
          {DIRECTIONS.map((d) => (
            <button
              key={d}
              type="button"
              onClick={() => onDirection(d)}
              className={`segment-btn ${direction === d ? 'segment-btn-active' : ''}`}
            >
              {d}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <span className="text-xs text-platform-muted mono tabular-nums">
          {count} signal{count === 1 ? '' : 's'}
        </span>
        <select
          value={sort}
          onChange={(e) => onSort(e.target.value as SignalSortKey)}
          className="bg-platform-surface border border-platform-border text-slate-200 text-xs rounded-lg px-3 py-2 outline-none focus:border-platform-accent/50"
        >
          {SORTS.map((s) => (
            <option key={s.key} value={s.key}>
              Sort: {s.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
