'use client';

import {
  DIRECTION_META,
  parseSignalSource,
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
      <div className="flex justify-between text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-0.5">
        <span>{label}</span>
        <span className="font-mono">{formatPct(value)}</span>
      </div>
      <div className="h-1 bg-ds-border rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${Math.round(value * 100)}%`, backgroundColor: color }}
        />
      </div>
    </div>
  );
}

export default function SignalDetailPanel({ signal, isLive }: SignalDetailPanelProps) {
  return (
    <aside className="flex flex-col h-full min-h-0 bg-ds-surface">
      <div className="px-3 py-1.5 border-b border-ds-border shrink-0">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Inspector
        </span>
      </div>

      {!signal ? (
        <div className="flex flex-col items-center justify-center flex-1 min-h-[160px] px-4 py-6 text-center">
          <div className="w-8 h-8 rounded-terminal bg-ds-elevated border border-ds-border flex items-center justify-center mb-2">
            <span className="text-ds-text-muted text-sm">⚡</span>
          </div>
          <p className="text-[11px] text-ds-text-secondary font-medium">Select a signal</p>
          <p className="text-[10px] text-ds-text-muted mt-1 max-w-[12rem] leading-relaxed">
            Click a row to inspect strength, features, and explanation.
          </p>
        </div>
      ) : (
        <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll p-3 flex flex-col gap-3">
          <div className="flex items-start justify-between gap-2">
            <div className="flex flex-wrap gap-1">
              <span
                className="px-1.5 py-px rounded-terminal text-[9px] font-medium uppercase"
                style={{
                  color: SIGNAL_TYPE_META[signal.signal_type].color,
                  backgroundColor: SIGNAL_TYPE_META[signal.signal_type].bg,
                }}
              >
                {SIGNAL_TYPE_META[signal.signal_type].label}
              </span>
              <span
                className="px-1.5 py-px rounded-terminal text-[9px] font-medium"
                style={{
                  color: DIRECTION_META[signal.direction].color,
                  backgroundColor: DIRECTION_META[signal.direction].bg,
                }}
              >
                {signal.direction}
              </span>
              {isLive && (
                <span className="px-1.5 py-px rounded-terminal text-[9px] font-medium text-ds-green bg-ds-green/10 border border-ds-green/25">
                  LIVE
                </span>
              )}
              {parseSignalSource(signal.explanation) && (
                <span className="px-1.5 py-px rounded-terminal text-[9px] font-mono text-ds-text-muted border border-ds-border">
                  {parseSignalSource(signal.explanation)}
                </span>
              )}
            </div>
            <span className="text-[9px] text-ds-text-muted font-mono shrink-0">#{signal.signal_id}</span>
          </div>

          <div>
            <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-0.5">Pool</p>
            <p className="font-mono text-[10px] text-ds-text-primary break-all leading-relaxed">
              {shortPool(signal.pool_address, 8, 8)}
            </p>
            <p className="text-[9px] text-ds-text-muted mt-1 font-mono">
              {tsToDate(signal.timestamp_micros).toLocaleString()} · {signal.timeframe_secs}s
            </p>
          </div>

          <div className="space-y-2">
            <Meter label="Strength" value={signal.strength} color={SIGNAL_TYPE_META[signal.signal_type].color} />
            <Meter label="Confidence" value={signal.confidence} color="var(--text-secondary)" />
          </div>

          <div>
            <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-1">Explanation</p>
            <p className="text-[10px] text-ds-text-secondary leading-relaxed bg-ds-elevated/40 rounded-terminal p-2 border border-ds-border">
              {signal.explanation || 'No explanation provided.'}
            </p>
          </div>

          <div>
            <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-1">Features</p>
            <div className="grid grid-cols-1 gap-0.5">
              {Object.entries(signal.feature_vector).map(([k, v]) => (
                <div
                  key={k}
                  className="flex justify-between gap-2 text-[10px] font-mono px-2 py-1 rounded-terminal bg-ds-elevated/30 border border-ds-border/60"
                >
                  <span className="text-ds-text-muted truncate">{k.replace(/_/g, ' ')}</span>
                  <span className="text-ds-text-primary tabular-nums">
                    {typeof v === 'number' ? v.toFixed(3) : String(v)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}
