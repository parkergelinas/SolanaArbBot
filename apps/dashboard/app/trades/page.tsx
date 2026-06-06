'use client';

import { DsPanel, PageHeader, PageShell } from '@/components/layout/PageShell';
import { useStreamTrades } from '@/lib/hooks';
import { formatUsd, type TradeEvent } from '@/lib/types';

function stageColor(stage: TradeEvent['stage']): string {
  switch (stage) {
    case 'filled':
      return 'bg-green-500/15 text-green-400';
    case 'failed':
    case 'rejected':
    case 'canceled':
      return 'bg-red-500/15 text-red-400';
    case 'submitted':
      return 'bg-blue-500/15 text-blue-400';
    default:
      return 'bg-slate-700 text-slate-300';
  }
}

export default function TradesPage() {
  const trades = useStreamTrades(200);

  return (
    <PageShell className="max-w-6xl">
      <PageHeader
        title="Trades"
        description="Live paper execution — quote through fill without refresh"
      />

      <DsPanel flush>
        <table className="w-full text-sm">
          <thead className="border-b border-slate-700">
            <tr className="text-slate-400 text-xs uppercase tracking-wide">
              <th className="text-left px-4 py-3">Time</th>
              <th className="text-left px-4 py-3">Strategy</th>
              <th className="text-left px-4 py-3">Pair</th>
              <th className="text-left px-4 py-3">Side</th>
              <th className="text-left px-4 py-3">Stage</th>
              <th className="text-right px-4 py-3">Size</th>
              <th className="text-right px-4 py-3">Exp. PnL</th>
              <th className="text-right px-4 py-3">Signal</th>
            </tr>
          </thead>
          <tbody>
            {trades.length === 0 ? (
              <tr>
                <td colSpan={8} className="text-center py-16 text-slate-500">
                  No trades yet — start the paper trading engine.
                </td>
              </tr>
            ) : (
              trades.map((t) => (
                <tr key={t.trade_id} className="border-t border-slate-700/50 hover:bg-slate-700/20">
                  <td className="px-4 py-3 text-slate-400 mono text-xs">
                    {new Date(t.timestamp_us / 1000).toLocaleTimeString()}
                  </td>
                  <td className="px-4 py-3 text-slate-300 text-xs capitalize">
                    {t.source_strategy}
                  </td>
                  <td className="px-4 py-3 mono text-xs truncate max-w-[140px] text-slate-300" title={t.pair}>
                    {t.pair.length > 16 ? `${t.pair.slice(0, 16)}…` : t.pair}
                  </td>
                  <td className="px-4 py-3">
                    <span className={`text-xs px-2 py-0.5 rounded font-medium ${
                      t.side === 'long' || t.side === 'buy'
                        ? 'bg-green-500/15 text-green-400'
                        : 'bg-red-500/15 text-red-400'
                    }`}>
                      {t.side}
                    </span>
                  </td>
                  <td className="px-4 py-3">
                    <span className={`text-xs px-2 py-0.5 rounded font-medium capitalize ${stageColor(t.stage)}`}>
                      {t.stage}
                    </span>
                    {t.reject_reason && (
                      <p className="text-[10px] text-red-400/80 mt-0.5 truncate max-w-[120px]" title={t.reject_reason}>
                        {t.reject_reason}
                      </p>
                    )}
                  </td>
                  <td className="px-4 py-3 text-right mono text-slate-300">
                    {formatUsd(t.size_usd)}
                  </td>
                  <td className={`px-4 py-3 text-right mono font-medium ${
                    t.expected_pnl_usd >= 0 ? 'text-green-400' : 'text-red-400'
                  }`}>
                    {t.expected_pnl_usd >= 0 ? '+' : ''}{formatUsd(t.expected_pnl_usd)}
                  </td>
                  <td className="px-4 py-3 text-right mono text-slate-500 text-xs">
                    {t.signal_id != null ? `#${t.signal_id}` : '–'}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </DsPanel>
    </PageShell>
  );
}
