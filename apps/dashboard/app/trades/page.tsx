'use client';

import { useCallback } from 'react';

import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { DsTable, DsTableHead, DsTd, DsTh } from '@/components/ui/DsTable';
import { useStreamTrades } from '@/lib/hooks';
import { formatUsd, type TradeEvent } from '@/lib/types';

function stageTone(stage: TradeEvent['stage']): 'green' | 'red' | 'blue' | 'muted' {
  switch (stage) {
    case 'filled':
      return 'green';
    case 'failed':
    case 'rejected':
    case 'canceled':
      return 'red';
    case 'submitted':
      return 'blue';
    default:
      return 'muted';
  }
}

export default function TradesPage() {
  const trades = useStreamTrades(200);

  const filled = trades.filter((t) => t.stage === 'filled').length;
  const netPnl = trades.reduce((sum, t) => sum + t.expected_pnl_usd, 0);

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Trades"
        subtitle="Live paper execution — quote through fill without refresh"
        actions={
          <DsBadge tone="blue" mono>
            {trades.length} events
          </DsBadge>
        }
      />

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0">
        <DsStatPill label="Total" value={trades.length} />
        <DsStatPill label="Filled" value={filled} accent="var(--green)" />
        <DsStatPill
          label="Net exp. PnL"
          value={formatUsd(netPnl)}
          accent={netPnl >= 0 ? 'var(--green)' : 'var(--red)'}
        />
        <DsStatPill
          label="Fill rate"
          value={trades.length ? `${((filled / trades.length) * 100).toFixed(0)}%` : '—'}
        />
      </div>

      <DsPanel flush className="flex-1 min-h-[420px] flex flex-col" title="Execution log">
        <DsTable>
          <DsTableHead>
            <tr>
              <DsTh>Time</DsTh>
              <DsTh>Strategy</DsTh>
              <DsTh>Pair</DsTh>
              <DsTh>Side</DsTh>
              <DsTh>Stage</DsTh>
              <DsTh align="right">Size</DsTh>
              <DsTh align="right">Exp. PnL</DsTh>
              <DsTh align="right">Signal</DsTh>
            </tr>
          </DsTableHead>
          <tbody>
            {trades.length === 0 ? (
              <tr>
                <td colSpan={8} className="text-center py-16 text-ds-text-muted text-[11px]">
                  No trades yet — start the paper trading engine.
                </td>
              </tr>
            ) : (
              trades.map((t) => (
                <tr key={t.trade_id} className="hover:bg-ds-elevated/30">
                  <DsTd mono className="text-ds-text-muted text-[10px]">
                    {new Date(t.timestamp_us / 1000).toLocaleTimeString()}
                  </DsTd>
                  <DsTd className="text-ds-text-secondary capitalize text-[10px]">{t.source_strategy}</DsTd>
                  <DsTd mono className="truncate max-w-[140px] text-ds-text-secondary text-[10px]" title={t.pair}>
                    {t.pair.length > 16 ? `${t.pair.slice(0, 16)}…` : t.pair}
                  </DsTd>
                  <DsTd>
                    <DsBadge tone={t.side === 'long' || t.side === 'buy' ? 'green' : 'red'}>
                      {t.side}
                    </DsBadge>
                  </DsTd>
                  <DsTd>
                    <DsBadge tone={stageTone(t.stage)}>{t.stage}</DsBadge>
                    {t.reject_reason && (
                      <p
                        className="text-[9px] text-ds-red/80 mt-0.5 truncate max-w-[120px]"
                        title={t.reject_reason}
                      >
                        {t.reject_reason}
                      </p>
                    )}
                  </DsTd>
                  <DsTd align="right" mono className="text-ds-text-secondary">
                    {formatUsd(t.size_usd)}
                  </DsTd>
                  <DsTd
                    align="right"
                    mono
                    className={t.expected_pnl_usd >= 0 ? 'text-ds-green' : 'text-ds-red'}
                  >
                    {t.expected_pnl_usd >= 0 ? '+' : ''}
                    {formatUsd(t.expected_pnl_usd)}
                  </DsTd>
                  <DsTd align="right" mono className="text-ds-text-muted text-[10px]">
                    {t.signal_id != null ? `#${t.signal_id}` : '–'}
                  </DsTd>
                </tr>
              ))
            )}
          </tbody>
        </DsTable>
      </DsPanel>
    </PageShell>
  );
}
