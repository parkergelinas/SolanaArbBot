'use client';

import { useEffect, useMemo, useRef, useState } from 'react';

import { candleKey, useMarketStore } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

interface ChartCandle {
  t: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

function CandlestickSvg({ data }: { data: ChartCandle[] }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 400, h: 200 });

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) setSize({ w: width, h: height });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const { yMin, yMax, candles } = useMemo(() => {
    if (data.length === 0) return { yMin: 0, yMax: 1, candles: [] as ChartCandle[] };
    const lows = data.map((d) => d.low);
    const highs = data.map((d) => d.high);
    const min = Math.min(...lows);
    const max = Math.max(...highs);
    const pad = (max - min) * 0.1 || Math.max(min * 0.01, 0.01);
    return { yMin: min - pad, yMax: max + pad, candles: data };
  }, [data]);

  const { w, h } = size;
  const padL = 44;
  const padR = 8;
  const padT = 8;
  const padB = 20;
  const plotW = w - padL - padR;
  const plotH = h - padT - padB;
  const yScale = (v: number) => padT + plotH - ((v - yMin) / (yMax - yMin)) * plotH;
  const slotW = plotW / Math.max(candles.length, 1);
  const barW = Math.max(2, slotW * 0.6);

  const yTicks = 4;
  const tickValues = Array.from({ length: yTicks + 1 }, (_, i) =>
    yMin + ((yMax - yMin) * i) / yTicks,
  );

  return (
    <div ref={containerRef} className="h-full w-full">
      <svg width={w} height={h} className="block">
        {tickValues.map((v) => (
          <g key={v}>
            <line
              x1={padL}
              x2={w - padR}
              y1={yScale(v)}
              y2={yScale(v)}
              stroke="#1e3a4a"
              strokeDasharray="2 4"
            />
            <text
              x={padL - 4}
              y={yScale(v) + 3}
              textAnchor="end"
              fill="#64748b"
              fontSize={9}
              fontFamily="monospace"
            >
              {v.toFixed(v >= 10 ? 2 : 4)}
            </text>
          </g>
        ))}

        {candles.map((c, i) => {
          const x = padL + i * slotW + (slotW - barW) / 2;
          const cx = x + barW / 2;
          const bullish = c.close >= c.open;
          const color = bullish ? '#22c55e' : '#ef4444';
          const yH = yScale(c.high);
          const yL = yScale(c.low);
          const yO = yScale(c.open);
          const yC = yScale(c.close);
          const bodyTop = Math.min(yO, yC);
          const bodyH = Math.max(Math.abs(yC - yO), 1);
          const showLabel = i === 0 || i === candles.length - 1 || i % Math.ceil(candles.length / 6) === 0;

          return (
            <g key={`${c.t}-${i}`}>
              <line x1={cx} x2={cx} y1={yH} y2={yL} stroke={color} strokeWidth={1} />
              <rect x={x} y={bodyTop} width={barW} height={bodyH} fill={color} fillOpacity={0.9} />
              {showLabel && (
                <text
                  x={cx}
                  y={h - 4}
                  textAnchor="middle"
                  fill="#64748b"
                  fontSize={8}
                  fontFamily="monospace"
                >
                  {c.t}
                </text>
              )}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

export default function CandleChart() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const interval = useUiStore((s) => s.candleInterval);
  const setInterval = useUiStore((s) => s.setCandleInterval);
  const candleHistory = useMarketStore((s) => s.candleHistory);

  const chartData = useMemo((): ChartCandle[] => {
    if (!selectedMint) return [];
    const key = candleKey(selectedMint, interval);
    return (candleHistory[key] ?? []).map((c) => ({
      t: new Date(c.ts_open_ms).toLocaleTimeString(),
      open: c.open,
      high: c.high,
      low: c.low,
      close: c.close,
      volume: c.volume,
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
      <div className="flex-1 min-h-[160px] p-1">
        {chartData.length === 0 ? (
          <div className="h-full flex items-center justify-center text-[11px] text-terminal-muted">
            Building candle history…
          </div>
        ) : (
          <CandlestickSvg data={chartData} />
        )}
      </div>
    </section>
  );
}
