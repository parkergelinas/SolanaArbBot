'use client';

import { AnimatePresence, motion } from 'framer-motion';
import { ArrowDown, ArrowUp, ArrowUpDown, Download, Search, X } from 'lucide-react';
import { useMemo, useState } from 'react';

import { CompactPageHeader, DsPanel, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import DsButton from '@/components/ui/DsButton';
import { DsTable, DsTableHead, DsTd, DsTh } from '@/components/ui/DsTable';
import StatCard from '@/components/ui/stat-card';
import { useStreamTrades } from '@/lib/hooks';
import { formatUsd, type TradeEvent, type TradeStage } from '@/lib/types';

// ── Stage helpers ─────────────────────────────────────────────────────────────

function stageTone(stage: TradeStage): 'green' | 'red' | 'blue' | 'muted' | 'amber' {
  switch (stage) {
    case 'filled':    return 'green';
    case 'failed':
    case 'rejected':
    case 'canceled':  return 'red';
    case 'submitted': return 'blue';
    case 'started':
    case 'quoted':
    case 'validated': return 'amber';
    default:          return 'muted';
  }
}

// ── Sort types ────────────────────────────────────────────────────────────────

type SortKey = 'time' | 'pnl' | 'size';
type SortDir = 'asc' | 'desc';

function SortIcon({ col, sortKey, sortDir }: { col: SortKey; sortKey: SortKey; sortDir: SortDir }) {
  if (col !== sortKey) return <ArrowUpDown className="w-2.5 h-2.5 opacity-35" />;
  return sortDir === 'desc'
    ? <ArrowDown className="w-2.5 h-2.5" />
    : <ArrowUp   className="w-2.5 h-2.5" />;
}

// ── Filter pill component ─────────────────────────────────────────────────────

function Pill({
  label,
  active,
  tone,
  onClick,
}: {
  label: string;
  active: boolean;
  tone?: 'green' | 'red' | 'amber' | 'blue';
  onClick: () => void;
}) {
  const activeClass = tone
    ? `active-${tone}`
    : 'active';
  return (
    <button
      type="button"
      onClick={onClick}
      className={`filter-pill ${active ? activeClass : ''}`}
    >
      {label}
    </button>
  );
}

// ── CSV export helper ─────────────────────────────────────────────────────────

function exportCsv(trades: TradeEvent[]) {
  const header = 'time,strategy,pair,side,stage,size_usd,expected_pnl_usd,signal_id';
  const rows = trades.map((t) =>
    [
      new Date(t.timestamp_us / 1000).toISOString(),
      t.source_strategy,
      t.pair,
      t.side,
      t.stage,
      t.size_usd.toFixed(2),
      t.expected_pnl_usd.toFixed(4),
      t.signal_id ?? '',
    ].join(','),
  );
  const blob = new Blob([[header, ...rows].join('\n')], { type: 'text/csv' });
  const url  = URL.createObjectURL(blob);
  const a    = Object.assign(document.createElement('a'), { href: url, download: 'trades.csv' });
  a.click();
  URL.revokeObjectURL(url);
}

// ── Page ──────────────────────────────────────────────────────────────────────

const STAGES: TradeStage[] = ['filled', 'submitted', 'failed', 'started'];
const STRATEGIES       = ['scalp', 'arb', 'pump', 'cross-dex'];

export default function TradesPage() {
  const trades = useStreamTrades(500);

  // ── Filters ──────────────────────────────────────────────────────────────
  const [stageFilter,    setStageFilter]    = useState<TradeStage | null>(null);
  const [strategyFilter, setStrategyFilter] = useState<string | null>(null);
  const [modeFilter,     setModeFilter]     = useState<'paper' | 'live' | null>(null);
  const [search,         setSearch]         = useState('');
  const [sortKey,        setSortKey]        = useState<SortKey>('time');
  const [sortDir,        setSortDir]        = useState<SortDir>('desc');

  // ── Stats ─────────────────────────────────────────────────────────────────
  const filled  = useMemo(() => trades.filter((t) => t.stage === 'filled').length, [trades]);
  const failed  = useMemo(() => trades.filter((t) => t.stage === 'failed' || t.stage === 'rejected').length, [trades]);
  const netPnl  = useMemo(() => trades.reduce((s, t) => s + t.expected_pnl_usd, 0), [trades]);
  const fillPct = trades.length ? (filled / trades.length) * 100 : 0;

  // ── Filtered + sorted ─────────────────────────────────────────────────────
  const visible = useMemo(() => {
    let out = [...trades];
    if (stageFilter)    out = out.filter((t) => t.stage === stageFilter);
    if (strategyFilter) out = out.filter((t) => t.source_strategy === strategyFilter);
    if (modeFilter)     out = out.filter((t) => (t as any).mode === modeFilter);
    if (search.trim())  {
      const q = search.toLowerCase();
      out = out.filter((t) => t.pair.toLowerCase().includes(q) || t.source_strategy.toLowerCase().includes(q));
    }
    out.sort((a, b) => {
      let diff = 0;
      if (sortKey === 'time') diff = a.timestamp_us - b.timestamp_us;
      if (sortKey === 'pnl')  diff = a.expected_pnl_usd - b.expected_pnl_usd;
      if (sortKey === 'size') diff = a.size_usd - b.size_usd;
      return sortDir === 'desc' ? -diff : diff;
    });
    return out;
  }, [trades, stageFilter, strategyFilter, modeFilter, search, sortKey, sortDir]);

  const activeFilters = [stageFilter, strategyFilter, modeFilter, search.trim()].filter(Boolean).length;

  function toggleSort(key: SortKey) {
    if (sortKey === key) setSortDir((d) => (d === 'desc' ? 'asc' : 'desc'));
    else { setSortKey(key); setSortDir('desc'); }
  }

  function clearFilters() {
    setStageFilter(null);
    setStrategyFilter(null);
    setModeFilter(null);
    setSearch('');
  }

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Trades"
        subtitle="Execution log — streaming via WebSocket"
        actions={
          <div className="flex items-center gap-1.5">
            <DsBadge tone="blue" mono>{visible.length} / {trades.length}</DsBadge>
            <DsButton
              size="xs"
              variant="ghost"
              onClick={() => exportCsv(visible)}
              title="Export visible rows as CSV"
            >
              <Download className="w-3 h-3" />
              Export
            </DsButton>
          </div>
        }
      />

      {/* ── Stats row ──────────────────────────────────────────────────────── */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0 px-3 pb-2">
        <StatCard
          label="Total"
          value={trades.length}
          format="integer"
        />
        <StatCard
          label="Filled"
          value={filled}
          format="integer"
          accent="var(--green)"
          badge={trades.length > 0 ? `${fillPct.toFixed(0)}%` : undefined}
        />
        <StatCard
          label="Failed"
          value={failed}
          format="integer"
          accent={failed > 0 ? 'var(--red)' : undefined}
        />
        <StatCard
          label="Net Exp. PnL"
          value={netPnl}
          format="usd"
          accent={netPnl >= 0 ? 'var(--green)' : 'var(--red)'}
          signed
        />
      </div>

      {/* ── Main panel ─────────────────────────────────────────────────────── */}
      <DsPanel flush className="flex-1 min-h-[380px] flex flex-col" title="Execution log">

        {/* Filter bar */}
        <div className="terminal-filterbar">
          {/* Stage */}
          <span className="text-[color:var(--text-muted)] mr-0.5 text-[9px] font-mono uppercase tracking-wider">Stage</span>
          {STAGES.map((s) => (
            <Pill
              key={s}
              label={s}
              active={stageFilter === s}
              tone={stageTone(s) as any}
              onClick={() => setStageFilter(stageFilter === s ? null : s)}
            />
          ))}

          <div className="w-px h-3.5 bg-[var(--bg-border)] mx-1 shrink-0" />

          {/* Strategy */}
          <span className="text-[color:var(--text-muted)] mr-0.5 text-[9px] font-mono uppercase tracking-wider">Strat</span>
          {STRATEGIES.map((s) => (
            <Pill
              key={s}
              label={s}
              active={strategyFilter === s}
              onClick={() => setStrategyFilter(strategyFilter === s ? null : s)}
            />
          ))}

          <div className="w-px h-3.5 bg-[var(--bg-border)] mx-1 shrink-0" />

          {/* Mode */}
          <Pill label="Paper" active={modeFilter === 'paper'} tone="amber" onClick={() => setModeFilter(modeFilter === 'paper' ? null : 'paper')} />
          <Pill label="Live"  active={modeFilter === 'live'}  tone="green" onClick={() => setModeFilter(modeFilter === 'live'  ? null : 'live')}  />

          {/* Spacer + search */}
          <div className="flex-1" />
          <div className="relative flex items-center">
            <Search className="absolute left-2 w-2.5 h-2.5 text-[color:var(--text-muted)] pointer-events-none" />
            <input
              type="text"
              placeholder="Search pair…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="filter-search pl-6"
              aria-label="Filter by pair"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch('')}
                className="absolute right-1.5 text-[color:var(--text-muted)] hover:text-[color:var(--text-primary)]"
                aria-label="Clear search"
              >
                <X className="w-2.5 h-2.5" />
              </button>
            )}
          </div>

          {activeFilters > 0 && (
            <button
              type="button"
              onClick={clearFilters}
              className="filter-pill active-red ml-1"
            >
              <X className="w-2 h-2" />
              Clear
            </button>
          )}
        </div>

        {/* Table */}
        <DsTable className="flex-1">
            <DsTableHead>
              <tr>
                <DsTh
                  className="sortable cursor-pointer select-none"
                  onClick={() => toggleSort('time')}
                >
                  <span className="flex items-center gap-1">
                    Time <SortIcon col="time" sortKey={sortKey} sortDir={sortDir} />
                  </span>
                </DsTh>
                <DsTh>Strategy</DsTh>
                <DsTh>Pair</DsTh>
                <DsTh>Side</DsTh>
                <DsTh>Stage</DsTh>
                <DsTh
                  align="right"
                  className="sortable cursor-pointer select-none"
                  onClick={() => toggleSort('size')}
                >
                  <span className="flex items-center justify-end gap-1">
                    Size <SortIcon col="size" sortKey={sortKey} sortDir={sortDir} />
                  </span>
                </DsTh>
                <DsTh
                  align="right"
                  className="sortable cursor-pointer select-none"
                  onClick={() => toggleSort('pnl')}
                >
                  <span className="flex items-center justify-end gap-1">
                    Exp. PnL <SortIcon col="pnl" sortKey={sortKey} sortDir={sortDir} />
                  </span>
                </DsTh>
                <DsTh align="right">Signal</DsTh>
              </tr>
            </DsTableHead>
            <tbody>
              {visible.length === 0 ? (
                <tr>
                  <td colSpan={8} className="text-center py-16">
                    {trades.length === 0 ? (
                      <span className="text-[color:var(--text-muted)] text-[11px] font-mono">
                        No trades yet — start the paper trading engine.
                      </span>
                    ) : (
                      <span className="text-[color:var(--text-muted)] text-[11px] font-mono">
                        No trades match the current filters.
                        <button
                          type="button"
                          onClick={clearFilters}
                          className="ml-2 text-[color:var(--blue)] underline-offset-2 hover:underline"
                        >
                          Clear filters
                        </button>
                      </span>
                    )}
                  </td>
                </tr>
              ) : (
                <AnimatePresence mode="popLayout" initial={false}>
                  {visible.map((t) => (
                    <motion.tr
                      key={t.trade_id}
                      layout
                      initial={{ opacity: 0, y: -6 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.15, ease: 'easeOut' }}
                      className="border-b border-[color-mix(in_srgb,var(--bg-border)_50%,transparent)] hover:bg-[color-mix(in_srgb,var(--blue)_3%,transparent)]"
                    >
                      <DsTd mono className="text-[color:var(--text-muted)] text-[10px] whitespace-nowrap">
                        {new Date(t.timestamp_us / 1000).toLocaleTimeString()}
                      </DsTd>
                      <DsTd className="text-[color:var(--text-secondary)] capitalize text-[10px]">
                        {t.source_strategy}
                      </DsTd>
                      <DsTd
                        mono
                        className="truncate max-w-[130px] text-[color:var(--text-secondary)] text-[10px]"
                        title={t.pair}
                      >
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
                            className="text-[9px] text-[color:var(--red)] opacity-80 mt-0.5 truncate max-w-[110px]"
                            title={t.reject_reason}
                          >
                            {t.reject_reason}
                          </p>
                        )}
                      </DsTd>
                      <DsTd align="right" mono className="text-[color:var(--text-secondary)]">
                        {formatUsd(t.size_usd)}
                      </DsTd>
                      <DsTd
                        align="right"
                        mono
                        className={t.expected_pnl_usd >= 0 ? 'text-[color:var(--green)]' : 'text-[color:var(--red)]'}
                      >
                        {t.expected_pnl_usd >= 0 ? '+' : ''}
                        {formatUsd(t.expected_pnl_usd)}
                      </DsTd>
                      <DsTd align="right" mono className="text-[color:var(--text-muted)] text-[10px]">
                        {t.signal_id != null ? `#${t.signal_id}` : '–'}
                      </DsTd>
                    </motion.tr>
                  ))}
                </AnimatePresence>
              )}
            </tbody>
          </DsTable>
      </DsPanel>
    </PageShell>
  );
}
