'use client';

import { useMemo } from 'react';
import {
  ComposedChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  Cell,
} from 'recharts';

import { candleKey, useMarketStore } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

export default function CandleChart() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const interval = useUiStore((s) => s.candleInterval);
  const setInterval = useUiStore((s) => s.setCandleInterval);
  const candleHistory = useMarketStore((s) => s.candleHistory);

  const chartData = useMemo(() => {
    if (!selectedMint) return [];
    const key = candleKey(selectedMint, interval);
    return (candleHistory[key] ?? []).map((c) => ({
      t: new Date(c.ts_open_ms).toLocaleTimeString(),
      open: c.open,
      high: c.high,
      low: c.low,
      close: c.close,
      volume: c.volume,
      bullish: c.close >= c.open,
      range: [c.low, c.high] as [number, number],
    }));
  }, [selectedMint, interval, candleHistory]);

  return (
    <section className="flex-1 flex flex-col min-h-0 border-r border-terminal-border bg-terminal-bg">
      <div className="flex items-center justify-between px-2 py-1 border-b border-terminal-border">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          OHLC · {selectedMint ? selectedMint.slice(0, 8) : '—'}
        </span>
        <div className="flex gap-0.5">
          {(['1s', '5s', '1m'] as const).map((iv) => (
            <button
              key={iv}
              type="button"
              onClick={() => setInterval(iv)}
              className={`px-1.5 py-0.5 rounded text-[10px] mono ${
                interval === iv
                  ? 'bg-terminal-live/20 text-terminal-live'
                  : 'text-terminal-muted hover:text-slate-300'
              }`}
            >
              {iv}
            </button>
          ))}
        </div>
      </div>
      <div className="flex-1 min-h-[200px] p-1">
        {chartData.length === 0 ? (
          <div className="h-full flex items-center justify-center text-[11px] text-terminal-muted">
            Building candle history…
          </div>
        ) : (
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart data={chartData} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
              <CartesianGrid stroke="#1e3a4a" strokeDasharray="2 4" />
              <XAxis dataKey="t" tick={{ fill: '#64748b', fontSize: 9 }} interval="preserveStartEnd" />
              <YAxis
                yAxisId="price"
                domain={['auto', 'auto']}
                tick={{ fill: '#64748b', fontSize: 9 }}
                width={48}
              />
              <YAxis yAxisId="vol" orientation="right" hide />
              <Tooltip
                contentStyle={{
                  background: '#0a1628',
                  border: '1px solid #1e3a4a',
                  fontSize: 10,
                  fontFamily: 'monospace',
                }}
                formatter={(value: number, name: string) => [
                  value.toFixed(4),
                  name.toUpperCase(),
                ]}
              />
              <Bar yAxisId="price" dataKey="high" barSize={6} radius={[1, 1, 0, 0]}>
                {chartData.map((entry, i) => (
                  <Cell
                    key={i}
                    fill={entry.bullish ? '#22c55e' : '#ef4444'}
                    fillOpacity={0.85}
                  />
                ))}
              </Bar>
              <Bar yAxisId="vol" dataKey="volume" fill="#22d3ee" fillOpacity={0.25} barSize={4} />
            </ComposedChart>
          </ResponsiveContainer>
        )}
      </div>
    </section>
  );
}
