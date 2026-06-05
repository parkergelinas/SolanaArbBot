import type { SignalEvent } from '@/lib/types';
import { directionColor, formatPct, signalColor, tsToDate } from '@/lib/types';

interface SignalCardProps {
  signal: SignalEvent;
  expanded?: boolean;
}

function StrengthBar({ value, color }: { value: number; color: string }) {
  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 h-1.5 bg-slate-700 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${(value * 100).toFixed(0)}%`, backgroundColor: color }}
        />
      </div>
      <span className="text-xs mono text-slate-400 w-10 text-right">
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
    <div className="bg-slate-800 border border-slate-700 rounded-xl p-4 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            className="px-2 py-0.5 rounded text-xs font-medium"
            style={{ backgroundColor: color + '22', color }}
          >
            {signal.signal_type}
          </span>
          <span
            className="px-2 py-0.5 rounded text-xs font-medium"
            style={{ backgroundColor: dirColor + '22', color: dirColor }}
          >
            {signal.direction}
          </span>
        </div>
        <span className="text-xs text-slate-500">{ts.toLocaleTimeString()}</span>
      </div>

      {/* Pool address */}
      <p className="mono text-xs text-slate-400 truncate">
        Pool: {signal.pool_address.slice(0, 16)}…{signal.pool_address.slice(-8)}
      </p>

      {/* Strength & confidence bars */}
      <div className="space-y-1.5">
        <div className="flex items-center gap-2 text-xs text-slate-500">
          <span className="w-20">Strength</span>
          <div className="flex-1">
            <StrengthBar value={signal.strength} color={color} />
          </div>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500">
          <span className="w-20">Confidence</span>
          <div className="flex-1">
            <StrengthBar value={signal.confidence} color="#94a3b8" />
          </div>
        </div>
      </div>

      {/* Explanation */}
      {expanded && (
        <>
          <p className="text-xs text-slate-300 bg-slate-900/50 rounded p-2 leading-relaxed">
            {signal.explanation}
          </p>

          {/* Feature vector */}
          <details className="text-xs">
            <summary className="cursor-pointer text-slate-500 hover:text-slate-300">
              Feature vector
            </summary>
            <div className="mt-2 grid grid-cols-2 gap-1 text-slate-400 mono">
              {Object.entries(signal.feature_vector).map(([k, v]) => (
                <div key={k} className="flex justify-between gap-2 bg-slate-900/40 px-2 py-1 rounded">
                  <span className="truncate">{k}</span>
                  <span className="text-slate-200">
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
