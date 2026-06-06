import type { SignalEvent } from '@/lib/types';
import { directionColor, formatPct, signalColor, tsToDate } from '@/lib/types';

interface SignalCardProps {
  signal: SignalEvent;
  expanded?: boolean;
}

function StrengthBar({ value, color }: { value: number; color: string }) {
  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 h-1 bg-ds-elevated rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${(value * 100).toFixed(0)}%`, backgroundColor: color }}
        />
      </div>
      <span className="text-[10px] font-mono text-ds-text-muted w-10 text-right tabular-nums">
        {formatPct(value)}
      </span>
    </div>
  );
}

export default function SignalCard({ signal, expanded = false }: SignalCardProps) {
  const color = signalColor(signal.signal_type);
  const dirColor = directionColor(signal.direction);
  const ts = tsToDate(signal.timestamp_micros);

  return (
    <div className="bg-ds-surface border border-ds-border rounded-terminal p-3 space-y-2.5">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 flex-wrap">
          <span
            className="px-1.5 py-px rounded-terminal border text-[10px] font-medium uppercase tracking-wider"
            style={{ backgroundColor: color + '18', color, borderColor: color + '40' }}
          >
            {signal.signal_type}
          </span>
          <span
            className="px-1.5 py-px rounded-terminal border text-[10px] font-medium uppercase tracking-wider"
            style={{ backgroundColor: dirColor + '18', color: dirColor, borderColor: dirColor + '40' }}
          >
            {signal.direction}
          </span>
        </div>
        <span className="text-[10px] font-mono text-ds-text-muted shrink-0">{ts.toLocaleTimeString()}</span>
      </div>

      <p className="font-mono text-[10px] text-ds-text-secondary truncate">
        Pool: {signal.pool_address.slice(0, 16)}…{signal.pool_address.slice(-8)}
      </p>

      <div className="space-y-1">
        <div className="flex items-center gap-2 text-[10px] text-ds-text-muted">
          <span className="w-16 shrink-0 uppercase tracking-wider">Strength</span>
          <div className="flex-1 min-w-0">
            <StrengthBar value={signal.strength} color={color} />
          </div>
        </div>
        <div className="flex items-center gap-2 text-[10px] text-ds-text-muted">
          <span className="w-16 shrink-0 uppercase tracking-wider">Confidence</span>
          <div className="flex-1 min-w-0">
            <StrengthBar value={signal.confidence} color="var(--text-secondary)" />
          </div>
        </div>
      </div>

      {expanded && (
        <>
          <p className="text-[11px] text-ds-text-secondary bg-ds-elevated/50 rounded-terminal border border-ds-border/50 p-2 leading-relaxed">
            {signal.explanation}
          </p>

          <details className="text-[10px]">
            <summary className="cursor-pointer text-ds-text-muted hover:text-ds-text-primary">
              Feature vector
            </summary>
            <div className="mt-2 grid grid-cols-2 gap-1 text-ds-text-secondary font-mono">
              {Object.entries(signal.feature_vector).map(([k, v]) => (
                <div
                  key={k}
                  className="flex justify-between gap-2 bg-ds-elevated/40 border border-ds-border/40 px-2 py-1 rounded-terminal"
                >
                  <span className="truncate">{k}</span>
                  <span className="text-ds-text-primary tabular-nums">
                    {typeof v === 'number' ? v.toFixed(4) : String(v)}
                  </span>
                </div>
              ))}
            </div>
          </details>
        </>
      )}
    </div>
  );
}
