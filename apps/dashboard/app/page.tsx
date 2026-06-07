'use client';

import { useCallback, useMemo, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import Link from 'next/link';
import {
  Activity,
  TrendingUp,
  TrendingDown,
  Zap,
  Shield,
  Clock,
  DollarSign,
  BarChart3,
  Radio,
  StopCircle,
  PlayCircle,
} from 'lucide-react';

import { PageShell, CompactPageHeader } from '@/components/layout/PageShell';
import { useStreamLatest, useStreamSignals, useFetch } from '@/lib/hooks';
import { useWsContext } from '@/components/WebSocketProvider';
import { api } from '@/lib/api';
import {
  formatUsd,
  type HealthStatus,
  type Portfolio,
  type Risk,
  type SignalEvent,
  type SystemStatus,
} from '@/lib/types';
import { cn } from '@/lib/cn';
import { StatCard } from '@/components/ui/stat-card';
import { LiveBadge, PulsingDot } from '@/components/ui/live-badge';
import { Panel } from '@/components/ui/panel';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

// ── Signal row ────────────────────────────────────────────────────────────────

const KIND_COLOR: Record<string, string> = {
  WhaleFlow:   '#3b82f6',
  SmartMoney:  '#a855f7',
  Momentum:    '#22c55e',
  Swap:        '#f59e0b',
  Arb:         '#06b6d4',
};

function SignalRow({ signal, index }: { signal: SignalEvent; index: number }) {
  const color = KIND_COLOR[signal.signal_type] ?? '#6b7280';
  const ts = new Date(signal.timestamp_micros / 1000);
  const strengthPct = `${(signal.strength * 100).toFixed(0)}%`;

  return (
    <motion.div
      layout
      initial={{ opacity: 0, x: -8 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 8 }}
      transition={{ duration: 0.2, delay: index * 0.04 }}
      className={cn(
        'flex items-center gap-3 px-3 py-2 border-b border-ds-border/50 last:border-0',
        'hover:bg-ds-elevated/40 transition-colors duration-150 group',
      )}
    >
      {/* Type badge */}
      <span
        className="shrink-0 px-1.5 py-px rounded text-[9px] font-semibold uppercase tracking-wide"
        style={{ backgroundColor: color + '18', color, border: `1px solid ${color}30` }}
      >
        {signal.signal_type}
      </span>

      {/* Pool */}
      <span className="flex-1 min-w-0 font-mono text-[10px] text-ds-text-muted truncate">
        {signal.pool_address.slice(0, 8)}…{signal.pool_address.slice(-6)}
      </span>

      {/* Strength bar */}
      <div className="hidden sm:flex items-center gap-1.5 w-24 shrink-0">
        <div className="flex-1 h-1 bg-ds-elevated rounded-full overflow-hidden">
          <motion.div
            initial={{ width: 0 }}
            animate={{ width: strengthPct }}
            transition={{ duration: 0.5, ease: 'easeOut' }}
            className="h-full rounded-full"
            style={{ backgroundColor: color }}
          />
        </div>
        <span className="text-[10px] font-mono text-ds-text-muted w-8 text-right tabular-nums">
          {strengthPct}
        </span>
      </div>

      {/* Direction */}
      <span
        className={cn(
          'shrink-0 text-[10px] font-mono',
          signal.direction === 'Long' ? 'text-ds-green' : signal.direction === 'Short' ? 'text-ds-red' : 'text-ds-text-muted',
        )}
      >
        {signal.direction}
      </span>

      {/* Time */}
      <span className="shrink-0 text-[9px] font-mono text-ds-text-muted tabular-nums">
        {ts.toLocaleTimeString()}
      </span>
    </motion.div>
  );
}

// ── Connection row ─────────────────────────────────────────────────────────────

function ConnectionRow({
  label,
  ok,
  detail,
}: {
  label: string;
  ok: boolean;
  detail?: string;
}) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-ds-border/40 last:border-0">
      <div className="flex items-center gap-2">
        <PulsingDot ok={ok} />
        <span className="text-[11px] text-ds-text-secondary">{label}</span>
      </div>
      {detail && (
        <span className="text-[9px] font-mono text-ds-text-muted">{detail}</span>
      )}
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export default function OverviewPage() {
  const { connected: wsConnected } = useWsContext();
  const health = useStreamLatest<HealthStatus>('health');
  const status = useStreamLatest<SystemStatus>('status');
  const portfolio = useStreamLatest<Portfolio>('portfolio');
  const risk = useStreamLatest<Risk>('risk');

  const liveSignals = useStreamSignals<SignalEvent>(8);
  const wsEmpty = liveSignals.length === 0;

  const fetchFallback = useCallback(() => api.liveSignals({ limit: 8 }), []);
  const { data: polledSignals } = useFetch(fetchFallback, wsEmpty ? 5_000 : null);

  const displaySignals = useMemo(() => {
    if (liveSignals.length > 0) return liveSignals;
    return polledSignals ?? [];
  }, [liveSignals, polledSignals]);

  const [busy, setBusy] = useState(false);
  const running = status?.running ?? false;

  const handleStart = async () => {
    setBusy(true);
    try { await api.startSystem(); } finally { setBusy(false); }
  };
  const handleStop = async () => {
    setBusy(true);
    try { await api.stopSystem(); } finally { setBusy(false); }
  };

  const wsStatus = wsConnected ? 'live' : 'offline';
  const exposurePct = risk?.current_exposure_pct ?? 0;
  const riskOk = risk?.risk_status === 'healthy';

  return (
    <PageShell desk>
      {/* ── Header ──────────────────────────────────────────────────────────── */}
      <CompactPageHeader
        title="Overview"
        subtitle="System health & live portfolio"
        actions={
          <>
            <LiveBadge status={wsStatus} />
            {running && (
              <LiveBadge
                status="live"
                label={status?.mode ?? 'paper'}
                className="capitalize"
              />
            )}
            <motion.button
              whileTap={{ scale: 0.95 }}
              onClick={handleStart}
              disabled={running || busy}
              className={cn(
                'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-terminal text-[11px] font-medium border transition-colors duration-150',
                'bg-ds-green/10 text-ds-green border-ds-green/30',
                'hover:bg-ds-green/20 disabled:opacity-40 disabled:cursor-not-allowed',
              )}
            >
              <PlayCircle className="w-3 h-3" />
              Start
            </motion.button>
            <motion.button
              whileTap={{ scale: 0.95 }}
              onClick={handleStop}
              disabled={!running || busy}
              className={cn(
                'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-terminal text-[11px] font-medium border transition-colors duration-150',
                'bg-ds-red/10 text-ds-red border-ds-red/30',
                'hover:bg-ds-red/20 disabled:opacity-40 disabled:cursor-not-allowed',
              )}
            >
              <StopCircle className="w-3 h-3" />
              Stop
            </motion.button>
          </>
        }
      />

      {/* ── KPI grid ─────────────────────────────────────────────────────────── */}
      <motion.div
        className="grid grid-cols-2 sm:grid-cols-4 xl:grid-cols-8 gap-2 shrink-0"
        initial="hidden"
        animate="visible"
        variants={{ visible: { transition: { staggerChildren: 0.06 } }, hidden: {} }}
      >
        <StatCard
          label="Capital"
          value={portfolio?.capital_usd ?? null}
          format={(n) => formatUsd(n)}
          loading={!portfolio}
        />
        <StatCard
          label="Unrealised"
          value={portfolio?.unrealised_pnl ?? null}
          format={(n) => formatUsd(n)}
          accent={portfolio ? (portfolio.unrealised_pnl >= 0 ? 'green' : 'red') : 'default'}
          trend={portfolio ? (portfolio.unrealised_pnl > 0 ? 'up' : portfolio.unrealised_pnl < 0 ? 'down' : 'neutral') : undefined}
          loading={!portfolio}
        />
        <StatCard
          label="Realised P&L"
          value={portfolio?.realised_pnl ?? null}
          format={(n) => formatUsd(n)}
          accent={portfolio ? (portfolio.realised_pnl >= 0 ? 'green' : 'red') : 'default'}
          trend={portfolio ? (portfolio.realised_pnl > 0 ? 'up' : portfolio.realised_pnl < 0 ? 'down' : 'neutral') : undefined}
          loading={!portfolio}
        />
        <StatCard
          label="Win rate"
          value={portfolio ? portfolio.win_rate * 100 : null}
          format={(n) => `${n.toFixed(1)}%`}
          accent="blue"
          loading={!portfolio}
        />
        <StatCard
          label="Signals"
          value={status?.signals_processed ?? null}
          format={(n) => n.toLocaleString()}
          loading={!status}
        />
        <StatCard
          label="Uptime"
          value={health ? fmtUptime(health.uptime_secs) : null}
          loading={!health}
        />
        <StatCard
          label="Exposure"
          value={risk ? exposurePct : null}
          format={(n) => `${n.toFixed(1)}%`}
          accent={exposurePct > 80 ? 'red' : exposurePct > 50 ? 'amber' : 'default'}
          loading={!risk}
        />
        <StatCard
          label="Risk"
          value={risk?.risk_status ?? null}
          accent={riskOk ? 'green' : 'red'}
          loading={!risk}
        />
      </motion.div>

      {/* ── Middle row ─────────────────────────────────────────────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_18rem] gap-2 shrink-0">
        {/* Connection status */}
        <Panel compact title="Live data stack" action={
          <Link
            href="/terminal"
            className="text-[10px] px-2 py-0.5 rounded-terminal border border-ds-border text-ds-blue hover:bg-ds-blue/10 transition-colors"
          >
            Terminal →
          </Link>
        }>
          <ConnectionRow label="Control API (WS)" ok={wsConnected} detail="Bot status · portfolio · config" />
          <ConnectionRow label="Engine" ok={running} detail={running ? status?.mode : 'stopped'} />
          <ConnectionRow label="Risk layer" ok={riskOk} detail={risk?.risk_status} />
          <ConnectionRow label="Signal bus" ok={(status?.signals_processed ?? 0) > 0} detail={
            status?.signals_processed
              ? `${status.signals_processed.toLocaleString()} processed`
              : 'no signals yet'
          } />
        </Panel>

        {/* Portfolio detail */}
        <Panel compact title="Portfolio" accent="bg-ds-blue/50">
          {portfolio ? (
            <dl className="grid grid-cols-2 gap-x-3 gap-y-2 text-[11px]">
              {[
                { k: 'Trades',   v: portfolio.total_trades.toLocaleString() },
                { k: 'Open',     v: String(portfolio.open_positions) },
                { k: 'Mode',     v: status?.mode ?? '–' },
                { k: 'Engine',   v: running ? 'Running' : 'Stopped', color: running ? 'text-ds-green' : 'text-ds-text-muted' },
              ].map(({ k, v, color }) => (
                <div key={k}>
                  <dt className="text-[9px] text-ds-text-muted uppercase tracking-wider">{k}</dt>
                  <dd className={cn('font-mono tabular-nums capitalize', color ?? 'text-ds-text-primary')}>
                    <AnimatePresence mode="wait">
                      <motion.span
                        key={v}
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.15 }}
                      >
                        {v}
                      </motion.span>
                    </AnimatePresence>
                  </dd>
                </div>
              ))}
            </dl>
          ) : (
            <div className="space-y-2">
              {[1, 2, 3, 4].map((i) => (
                <div key={i} className="h-8 bg-ds-elevated rounded animate-pulse" />
              ))}
            </div>
          )}
        </Panel>
      </div>

      {/* ── Signal feed ─────────────────────────────────────────────────────── */}
      <Panel
        className="flex-1 min-h-0 flex flex-col"
        flush
        title="Live signals"
        action={
          displaySignals.length > 0 ? (
            <div className="flex items-center gap-2">
              {liveSignals.length > 0 && (
                <span className="flex items-center gap-1 text-[9px] font-mono text-ds-green">
                  <Radio className="w-2.5 h-2.5" />
                  WS live
                </span>
              )}
              <span className="text-[10px] font-mono text-ds-text-muted">
                {displaySignals.length} signals
              </span>
            </div>
          ) : undefined
        }
      >
        <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
          <AnimatePresence mode="popLayout" initial={false}>
            {displaySignals.length === 0 ? (
              <motion.div
                key="empty"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="flex flex-col items-center justify-center gap-3 py-12 text-center"
              >
                <Activity className="w-8 h-8 text-ds-text-muted/40" />
                <p className="text-[11px] text-ds-text-muted">
                  No signals yet — start the engine or connect to market data
                </p>
                <Link
                  href="/terminal"
                  className="text-[10px] text-ds-blue hover:underline"
                >
                  Open terminal →
                </Link>
              </motion.div>
            ) : (
              [...displaySignals].reverse().map((s, i) => (
                <SignalRow key={`${s.signal_id}-${i}`} signal={s} index={i} />
              ))
            )}
          </AnimatePresence>
        </div>
      </Panel>
    </PageShell>
  );
}
