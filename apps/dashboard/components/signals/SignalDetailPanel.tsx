'use client';

import {
  DIRECTION_META,
  SIGNAL_TYPE_META,
  shortPool,
} from '@/lib/signals';
import type { SignalEvent } from '@/lib/types';
import { formatPct, tsToDate } from '@/lib/types';

interface SignalDetailPanelProps {
  signal: SignalEvent | null;
  isLive: boolean;
}

function Meter({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div>
      <div className="flex justify-between text-[10px] uppercase tracking-wider text-platform-muted mb-1">
        <span>{label}</span>
        <span className="mono">{formatPct(value)}</span>
      </div>
      <div className="h-1.5 bg-platform-border rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${Math.round(value * 100)}%`, backgroundColor: color }}
        />
      </div>
    </div>
  );
}

export default function SignalDetailPanel({ signal, isLive }: SignalDetailPanelProps) {
  if (!signal) {
    return (
      <aside className="surface-card p-6 flex flex-col items-center justify-center text-center min-h-[280px] lg:min-h-0 lg:w-80 shrink-0">
        <div className="w-10 h-10 rounded-full bg-platform-surface border border-platform-border flex items-center justify-center mb-3">
          <span className="text-platform-muted text-lg">⚡</span>
        </div>
        <p className="text-sm text-slate-300 font-medium">Select a signal</p>
        <p className="text-xs text-platform-muted mt-1 max-w-[14rem]">
          Click any row to inspect strength, features, and explanation trace.
        </p>
      </aside>
    );
  }

  const typeMeta = SIGNAL_TYPE_META[signal.signal_type];
  const dirMeta = DIRECTION_META[signal.direction];
  const ts = tsToDate(signal.timestamp_micros);

  return (
    <aside className="surface-card p-4 lg:w-80 shrink-0 flex flex-col gap-4 lg:sticky lg:top-0 lg:self-start">
      <div className="flex items-start justify-between gap-2">
        <div className="flex flex-wrap gap-1.5">
          <span
            className="px-2 py-0.5 rounded-md text-[11px] font-medium"
            style={{ color: typeMeta.color, backgroundColor: typeMeta.bg }}
          >
            {typeMeta.label}
          </span>
          <span
            className="px-2 py-0.5 rounded-md text-[11px] font-medium"
            style={{ color: dirMeta.color, backgroundColor: dirMeta.bg }}
          >
            {signal.direction}
          </span>
          {isLive && (
            <span className="px-2 py-0.5 rounded-md text-[11px] font-medium text-platform-accent bg-platform-accent/10">
              LIVE
            </span>
          )}
        </div>
        <span className="text-[10px] text-platform-muted mono shrink-0">#{signal.signal_id}</span>
      </div>

      <div>
        <p className="text-[10px] uppercase tracking-wider text-platform-muted mb-1">Pool</p>
        <p className="mono text-xs text-slate-200 break-all">{shortPool(signal.pool_address, 8, 8)}</p>
        <p className="text-[10px] text-platform-muted mt-2">
          {ts.toLocaleString()} · {signal.timeframe_secs}s window
        </p>
      </div>

      <div className="space-y-3">
        <Meter label="Strength" value={signal.strength} color={typeMeta.color} />
        <Meter label="Confidence" value={signal.confidence} color="#8b95a8" />
      </div>

      <div>
        <p className="text-[10px] uppercase tracking-wider text-platform-muted mb-2">Explanation</p>
        <p className="text-xs text-slate-300 leading-relaxed bg-platform-bg/60 rounded-lg p-3 border border-platform-border/60">
          {signal.explanation || 'No explanation provided.'}
        </p>
      </div>

      <div>
        <p className="text-[10px] uppercase tracking-wider text-platform-muted mb-2">Features</p>
        <div className="grid grid-cols-1 gap-1">
          {Object.entries(signal.feature_vector).map(([k, v]) => (
            <div
              key={k}
              className="flex justify-between gap-2 text-[11px] mono px-2 py-1.5 rounded-md bg-platform-bg/40 border border-platform-border/40"
            >
              <span className="text-platform-muted truncate">{k.replace(/_/g, ' ')}</span>
              <span className="text-slate-200 tabular-nums">
                {typeof v === 'number' ? v.toFixed(3) : String(v)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </aside>
  );
}
