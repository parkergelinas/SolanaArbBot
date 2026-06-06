'use client';

import type { ReactNode } from 'react';

import type { SignalFilterDirection, SignalFilterType, SignalSortKey } from '@/lib/signals';

const TYPES: SignalFilterType[] = ['All', 'WhaleFlow', 'SmartMoney', 'Momentum', 'Swap'];
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
  Momentum: 'Mom',
  Swap: 'Swap',
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

function SegmentGroup({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <div className="flex items-center gap-1.5 min-w-0">
      <span className="text-[8px] uppercase tracking-[0.14em] text-ds-text-muted shrink-0 hidden sm:inline">
        {label}
      </span>
      <div className="flex items-center gap-0.5 p-0.5 bg-ds-elevated/50 border border-ds-border rounded-terminal overflow-x-auto terminal-scroll">
        {children}
      </div>
    </div>
  );
}

function SegmentBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`px-2 py-0.5 text-[9px] uppercase tracking-wider rounded-terminal whitespace-nowrap transition-colors ${
        active
          ? 'bg-ds-elevated text-ds-blue border border-ds-blue/30'
          : 'text-ds-text-muted hover:text-ds-text-secondary border border-transparent'
      }`}
    >
      {children}
    </button>
  );
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
    <div className="flex flex-wrap items-center gap-2 bg-ds-surface border border-ds-border rounded-terminal px-2 py-1.5">
      <SegmentGroup label="Type">
        {TYPES.map((t) => (
          <SegmentBtn key={t} active={type === t} onClick={() => onType(t)}>
            {TYPE_LABELS[t]}
          </SegmentBtn>
        ))}
      </SegmentGroup>

      <SegmentGroup label="Side">
        {DIRECTIONS.map((d) => (
          <SegmentBtn key={d} active={direction === d} onClick={() => onDirection(d)}>
            {d}
          </SegmentBtn>
        ))}
      </SegmentGroup>

      <div className="flex items-center gap-2 ml-auto shrink-0">
        <span className="text-[10px] font-mono text-ds-text-muted tabular-nums">
          {count} row{count === 1 ? '' : 's'}
        </span>
        <select
          value={sort}
          onChange={(e) => onSort(e.target.value as SignalSortKey)}
          className="bg-ds-elevated border border-ds-border text-ds-text-primary text-[10px] font-mono rounded-terminal px-2 py-0.5 outline-none focus:border-ds-blue/40 cursor-pointer"
        >
          {SORTS.map((s) => (
            <option key={s.key} value={s.key}>
              {s.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
