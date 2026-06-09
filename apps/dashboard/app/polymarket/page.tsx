'use client';

import { useCallback, useEffect, useState } from 'react';
import { RefreshCw, TrendingDown, TrendingUp } from 'lucide-react';
import {
  Area, AreaChart, CartesianGrid,
  ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts';

import { CompactPageHeader, DsPanel, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { DsTable, DsTableHead, DsTd, DsTh } from '@/components/ui/DsTable';
import StatCard from '@/components/ui/stat-card';

// ── API base ──────────────────────────────────────────────────────────────────

const POLY_API = process.env.NEXT_PUBLIC_POLY_API_URL ?? 'http://127.0.0.1:4000';

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${POLY_API}${path}`, { cache: 'no-store' });
  if (!res.ok) throw new Error(`${path} ${res.status}`);
  return res.json() as Promise<T>;
}

// ── Types ─────────────────────────────────────────────────────────────────────

interface Outcome {
  token_id: string;
  name: string;
  mid: number | null;
  bid_best: number | null;
  ask_best: number | null;
  spread: number | null;
  spread_pct: number | null;
  buy_ratio: number | null;
  trade_count: number;
  trade_vwap: number | null;
  book_imbalance_10c: number | null;
  weighted_mid: number | null;
  depth_5c_bid: number;
  depth_5c_ask: number;
  total_bid_depth: number;
  total_ask_depth: number;
}

interface Market {
  condition_id: string;
  question: string;
  slug: string | null;
  event_title: string | null;
  end_date: string | null;
  days_to_expiry: number | null;
  volume_24h: number | null;
  volume_total: number | null;
  liquidity: number | null;
  spread: number | null;
  competitive: number | null;
  price_change_1wk: number | null;
  arb_deviation: number | null;
  price_sum: number | null;
  edge_score: number;
  vol_liq_ratio: number | null;
  outcomes_json: Outcome[];
  ts: string;
}

interface Stats {
  market_count: number;
  total_volume_24h: number;
  avg_edge_score: number;
  max_edge_score: number;
  arb_signals: number;
  unique_markets: number;
  last_scan_ts: string;
  total_rows: number;
}

// ── Formatters ────────────────────────────────────────────────────────────────

function fmtUsd(v: number | null): string {
  if (v == null) return '--';
  if (v >= 1_000_000) return `$${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1_000)     return `$${(v / 1_000).toFixed(1)}K`;
  return `$${v.toFixed(0)}`;
}
function fmtPct(v: number | null, decimals = 1): string {
  if (v == null) return '--';
  return `${(v * 100).toFixed(decimals)}%`;
}
function fmtDte(v: number | null): string {
  if (v == null) return '--';
  if (v < 0)    return 'expired';
  if (v < 1)    return `${(v * 24).toFixed(1)}h`;
  return `${v.toFixed(1)}d`;
}
function fmtScore(v: number): string {
  return v.toFixed(2);
}

// ── Colour helpers ────────────────────────────────────────────────────────────

function probColor(v: number | null): string {
  if (v == null) return 'text-ds-text-muted';
  if (v >= 0.70) return 'text-ds-green';
  if (v <= 0.30) return 'text-ds-red';
  return 'text-ds-amber';
}
function changeColor(v: number | null): string {
  if (v == null) return 'text-ds-text-muted';
  return v >= 0 ? 'text-ds-green' : 'text-ds-red';
}
function arbColor(v: number | null): string {
  if (v == null) return 'text-ds-text-muted';
  if (Math.abs(v) >= 0.02) return 'text-ds-green font-semibold';
  return 'text-ds-text-secondary';
}
function imbalColor(v: number | null): string {
  if (v == null) return 'text-ds-text-muted';
  if (v >= 0.65) return 'text-ds-green';
  if (v <= 0.35) return 'text-ds-red';
  return 'text-ds-text-secondary';
}

// ── Score badge ───────────────────────────────────────────────────────────────

function ScoreBadge({ score }: { score: number }) {
  const tone = score >= 7 ? 'green' : score >= 5 ? 'amber' : 'muted';
  return <DsBadge tone={tone}>{fmtScore(score)}</DsBadge>;
}

// ── Flow bar ──────────────────────────────────────────────────────────────────

function FlowBar({ ratio }: { ratio: number | null }) {
  if (ratio == null) return <span className="text-ds-text-muted text-xs">--</span>;
  const buyPct = Math.round(ratio * 100);
  const sellPct = 100 - buyPct;
  return (
    <div className="flex flex-col gap-0.5 w-16">
      <div className="flex h-1 rounded-full overflow-hidden bg-ds-surface">
        <div className="bg-ds-green" style={{ width: `${buyPct}%` }} />
        <div className="bg-ds-red"   style={{ width: `${sellPct}%` }} />
      </div>
      <span className="text-[10px] text-ds-text-muted font-mono">{buyPct}% buy</span>
    </div>
  );
}

// ── Selected market detail ────────────────────────────────────────────────────

function MarketDetail({ market, history }: { market: Market; history: Market[] }) {
  const yes = market.outcomes_json?.[0];
  const no  = market.outcomes_json?.[1];

  const chartData = history.slice(-60).map((h) => ({
    ts: h.ts.slice(11, 16),
    yes: h.outcomes_json?.[0]?.mid != null ? +(h.outcomes_json[0].mid * 100).toFixed(1) : null,
    no:  h.outcomes_json?.[1]?.mid != null ? +(h.outcomes_json[1].mid * 100).toFixed(1) : null,
  })).filter((d) => d.yes != null || d.no != null);

  return (
    <DsPanel className="flex flex-col gap-4">
      <div>
        <p className="text-xs text-ds-text-muted uppercase tracking-widest mb-1">Selected Market</p>
        <p className="text-sm text-ds-text-primary font-medium leading-snug">{market.question}</p>
        {market.event_title && (
          <p className="text-xs text-ds-text-muted mt-0.5">{market.event_title}</p>
        )}
      </div>

      {/* Price chart */}
      {chartData.length > 1 && (
        <div>
          <p className="text-[10px] text-ds-text-muted uppercase tracking-widest mb-2">Price History</p>
          <ResponsiveContainer width="100%" height={120}>
            <AreaChart data={chartData} margin={{ top: 0, right: 0, left: -28, bottom: 0 }}>
              <defs>
                <linearGradient id="gYes" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="var(--green)" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="var(--green)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="2 4" stroke="var(--bg-border)" />
              <XAxis dataKey="ts" tick={{ fontSize: 9, fill: 'var(--text-muted)' }} />
              <YAxis domain={[0, 100]} tick={{ fontSize: 9, fill: 'var(--text-muted)' }} unit="%" />
              <Tooltip
                contentStyle={{ background: 'var(--bg-elevated)', border: '1px solid var(--bg-border)', fontSize: 11 }}
                formatter={(v: number) => [`${v}%`]}
              />
              <Area type="monotone" dataKey="yes" name="YES" stroke="var(--green)" fill="url(#gYes)" dot={false} strokeWidth={1.5} />
              <Area type="monotone" dataKey="no"  name="NO"  stroke="var(--red)"   fill="none"        dot={false} strokeWidth={1} strokeDasharray="3 3" />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Outcome stats */}
      <div className="grid grid-cols-2 gap-3">
        {[yes, no].map((o, i) => o && (
          <div key={i} className="bg-ds-surface border border-ds-border rounded-terminal p-2.5 flex flex-col gap-1.5">
            <p className={`text-xs font-semibold uppercase tracking-wider ${i === 0 ? 'text-ds-green' : 'text-ds-red'}`}>{o.name}</p>
            <p className={`text-2xl font-mono font-bold ${probColor(o.mid)}`}>
              {o.mid != null ? fmtPct(o.mid) : '--'}
            </p>
            <div className="text-[10px] text-ds-text-muted font-mono grid grid-cols-2 gap-x-2 gap-y-0.5">
              <span>Bid</span>   <span className="text-ds-text-secondary">{fmtPct(o.bid_best)}</span>
              <span>Ask</span>   <span className="text-ds-text-secondary">{fmtPct(o.ask_best)}</span>
              <span>Spread</span><span className="text-ds-text-secondary">{fmtPct(o.spread)}</span>
              <span>VWAP</span>  <span className="text-ds-text-secondary">{fmtPct(o.trade_vwap)}</span>
              <span>Trades</span><span className="text-ds-text-secondary">{o.trade_count}</span>
              <span>Imbal</span> <span className={imbalColor(o.book_imbalance_10c)}>{o.book_imbalance_10c?.toFixed(2) ?? '--'}</span>
            </div>
            <FlowBar ratio={o.buy_ratio} />
          </div>
        ))}
      </div>

      {/* Market meta */}
      <div className="text-[10px] font-mono text-ds-text-muted grid grid-cols-2 gap-x-4 gap-y-1">
        <span>Vol 24h</span>     <span className="text-ds-text-secondary">{fmtUsd(market.volume_24h)}</span>
        <span>Liquidity</span>   <span className="text-ds-text-secondary">{fmtUsd(market.liquidity)}</span>
        <span>Vol/Liq</span>     <span className="text-ds-text-secondary">{market.vol_liq_ratio?.toFixed(2) ?? '--'}</span>
        <span>YES+NO</span>      <span className={arbColor(market.arb_deviation)}>{market.price_sum != null ? fmtPct(market.price_sum) : '--'}</span>
        <span>Competitive</span> <span className="text-ds-text-secondary">{market.competitive?.toFixed(3) ?? '--'}</span>
        <span>Expires</span>     <span className="text-ds-text-secondary">{fmtDte(market.days_to_expiry)}</span>
        <span>Last scan</span>   <span className="text-ds-text-secondary">{market.ts.slice(0, 19).replace('T', ' ')}</span>
      </div>
    </DsPanel>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

type SortKey = 'edge_score' | 'volume_24h' | 'arb_deviation' | 'competitive';

export default function PolymarketPage() {
  const [markets, setMarkets]         = useState<Market[]>([]);
  const [stats, setStats]             = useState<Stats | null>(null);
  const [selected, setSelected]       = useState<Market | null>(null);
  const [history, setHistory]         = useState<Market[]>([]);
  const [sortKey, setSortKey]         = useState<SortKey>('edge_score');
  const [loading, setLoading]         = useState(false);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [error, setError]             = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [m, s] = await Promise.all([
        apiFetch<Market[]>(`/markets?limit=100&sort=${sortKey}`),
        apiFetch<Stats>('/stats'),
      ]);
      setMarkets(m);
      setStats(s);
      setLastRefresh(new Date());
    } catch (e) {
      setError(e instanceof Error ? e.message : 'fetch failed');
    } finally {
      setLoading(false);
    }
  }, [sortKey]);

  useEffect(() => { load(); }, [load]);

  // Auto-refresh every 60 s
  useEffect(() => {
    const id = setInterval(load, 60_000);
    return () => clearInterval(id);
  }, [load]);

  const selectMarket = useCallback(async (m: Market) => {
    setSelected(m);
    try {
      const h = await apiFetch<Market[]>(`/market/${m.condition_id}?limit=200`);
      setHistory(h);
    } catch {
      setHistory([]);
    }
  }, []);

  const sorted = [...markets].sort((a, b) => {
    if (sortKey === 'arb_deviation') return Math.abs(b.arb_deviation ?? 0) - Math.abs(a.arb_deviation ?? 0);
    const bv = (b as any)[sortKey] ?? 0;
    const av = (a as any)[sortKey] ?? 0;
    return bv - av;
  });

  return (
    <PageShell>
      <CompactPageHeader title="Polymarket" subtitle="Prediction market scanner" />

      {/* ── Stat bar ── */}
      <div className="grid grid-cols-2 sm:grid-cols-4 xl:grid-cols-6 gap-3 mb-4">
        <StatCard label="Markets"     value={stats?.market_count ?? null} />
        <StatCard label="Vol 24h"     value={fmtUsd(stats?.total_volume_24h ?? null)} />
        <StatCard label="Avg Score"   value={stats?.avg_edge_score?.toFixed(2) ?? null} />
        <StatCard label="Arb Signals" value={stats?.arb_signals ?? null} accent={stats?.arb_signals ? 'amber' : undefined} />
        <StatCard label="Unique Mkts" value={stats?.unique_markets ?? null} />
        <StatCard label="DB Rows"     value={stats?.total_rows?.toLocaleString() ?? null} />
      </div>

      {/* ── Error banner ── */}
      {error && (
        <div className="mb-3 px-3 py-2 bg-ds-red/10 border border-ds-red/30 rounded-terminal text-ds-red text-xs font-mono">
          {error} — is the scanner API running? (python api.py)
        </div>
      )}

      <div className="flex gap-4 items-start">
        {/* ── Market table ── */}
        <div className="flex-1 min-w-0">
          {/* Toolbar */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex gap-1">
              {(['edge_score', 'volume_24h', 'arb_deviation', 'competitive'] as SortKey[]).map((k) => (
                <button
                  key={k}
                  onClick={() => setSortKey(k)}
                  className={`filter-pill text-[10px] ${sortKey === k ? 'active' : ''}`}
                >
                  {k === 'edge_score' ? 'Score' : k === 'volume_24h' ? 'Volume' : k === 'arb_deviation' ? 'Arb' : 'Competitive'}
                </button>
              ))}
            </div>
            <button
              onClick={load}
              disabled={loading}
              className="flex items-center gap-1.5 text-[10px] text-ds-text-muted hover:text-ds-text-secondary transition-colors"
            >
              <RefreshCw className={`w-3 h-3 ${loading ? 'animate-spin' : ''}`} />
              {lastRefresh ? lastRefresh.toLocaleTimeString() : 'refresh'}
            </button>
          </div>

          <DsPanel className="overflow-x-auto">
            <DsTable>
              <DsTableHead>
                <tr>
                  <DsTh>Score</DsTh>
                  <DsTh>Question</DsTh>
                  <DsTh>Vol 24h</DsTh>
                  <DsTh>Liq</DsTh>
                  <DsTh>DTE</DsTh>
                  <DsTh>YES</DsTh>
                  <DsTh>NO</DsTh>
                  <DsTh>Spread</DsTh>
                  <DsTh>Flow</DsTh>
                  <DsTh>Arb</DsTh>
                  <DsTh>1wk</DsTh>
                </tr>
              </DsTableHead>
              <tbody>
                {sorted.map((m) => {
                  const yes = m.outcomes_json?.[0];
                  const no  = m.outcomes_json?.[1];
                  const isSelected = selected?.condition_id === m.condition_id;
                  return (
                    <tr
                      key={m.condition_id}
                      onClick={() => selectMarket(m)}
                      className={`cursor-pointer hover:bg-ds-elevated/40 transition-colors ${
                        isSelected ? 'bg-ds-elevated/60 border-l-2 border-l-ds-green' : ''
                      }`}
                    >
                      <DsTd><ScoreBadge score={m.edge_score} /></DsTd>
                      <DsTd>
                        <span className="text-ds-text-primary text-xs leading-snug line-clamp-2 max-w-xs">
                          {m.question}
                        </span>
                        {m.event_title && (
                          <span className="block text-[10px] text-ds-text-muted truncate max-w-xs">{m.event_title}</span>
                        )}
                      </DsTd>
                      <DsTd><span className="font-mono text-xs">{fmtUsd(m.volume_24h)}</span></DsTd>
                      <DsTd><span className="font-mono text-xs">{fmtUsd(m.liquidity)}</span></DsTd>
                      <DsTd>
                        <span className={`text-xs font-mono ${m.days_to_expiry != null && m.days_to_expiry < 3 ? 'text-ds-amber' : 'text-ds-text-secondary'}`}>
                          {fmtDte(m.days_to_expiry)}
                        </span>
                      </DsTd>
                      <DsTd>
                        <span className={`font-mono text-xs ${probColor(yes?.mid ?? null)}`}>
                          {yes?.mid != null ? fmtPct(yes.mid) : '--'}
                        </span>
                      </DsTd>
                      <DsTd>
                        <span className={`font-mono text-xs ${probColor(no?.mid ?? null)}`}>
                          {no?.mid != null ? fmtPct(no.mid) : '--'}
                        </span>
                      </DsTd>
                      <DsTd>
                        <span className="font-mono text-xs text-ds-text-secondary">
                          {yes?.spread_pct != null ? fmtPct(yes.spread_pct) : '--'}
                        </span>
                      </DsTd>
                      <DsTd><FlowBar ratio={yes?.buy_ratio ?? null} /></DsTd>
                      <DsTd>
                        <span className={`font-mono text-xs ${arbColor(m.arb_deviation)}`}>
                          {m.arb_deviation != null ? `${(m.arb_deviation * 100).toFixed(2)}%` : '--'}
                        </span>
                      </DsTd>
                      <DsTd>
                        <span className={`font-mono text-xs flex items-center gap-0.5 ${changeColor(m.price_change_1wk)}`}>
                          {m.price_change_1wk != null && (
                            m.price_change_1wk >= 0
                              ? <TrendingUp className="w-2.5 h-2.5" />
                              : <TrendingDown className="w-2.5 h-2.5" />
                          )}
                          {m.price_change_1wk != null ? fmtPct(m.price_change_1wk) : '--'}
                        </span>
                      </DsTd>
                    </tr>
                  );
                })}
                {sorted.length === 0 && !loading && (
                  <tr>
                    <td colSpan={11} className="px-2 py-8 text-center text-ds-text-muted text-xs">
                      {error ? 'Scanner API offline — run: python api.py' : 'No data — run the scanner first'}
                    </td>
                  </tr>
                )}
              </tbody>
            </DsTable>
          </DsPanel>
        </div>

        {/* ── Detail panel ── */}
        {selected && (
          <div className="w-80 flex-shrink-0">
            <MarketDetail market={selected} history={history} />
          </div>
        )}
      </div>
    </PageShell>
  );
}
