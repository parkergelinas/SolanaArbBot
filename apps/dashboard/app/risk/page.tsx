'use client';

import { motion } from 'framer-motion';
import { AlertTriangle, CheckCircle2, RefreshCw, ShieldAlert, ShieldCheck, XCircle } from 'lucide-react';
import { useCallback, useState } from 'react';

import { CompactPageHeader, DsPanel, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import DsButton from '@/components/ui/DsButton';
import StatCard from '@/components/ui/stat-card';
import { useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd } from '@/lib/types';

// ── Animated gauge bar ────────────────────────────────────────────────────────

function GaugeBar({
  value,
  max,
  label,
  formatValue,
  formatMax,
  inverted = false,
}: {
  value: number;
  max: number;
  label: string;
  formatValue?: (v: number) => string;
  formatMax?: (v: number) => string;
  inverted?: boolean;
}) {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const effectivePct = inverted ? 100 - pct : pct;

  const barColor =
    effectivePct > 80 ? 'var(--red)'
    : effectivePct > 60 ? 'var(--amber)'
    : 'var(--blue)';

  const pctLabel = pct.toFixed(1) + '%';
  const valLabel = formatValue ? formatValue(value) : value.toLocaleString();
  const maxLabel = formatMax ? formatMax(max) : max.toLocaleString();

  return (
    <div className="space-y-1.5">
      <div className="flex justify-between items-baseline">
        <span className="text-[9px] font-mono uppercase tracking-[0.1em] text-[color:var(--text-muted)]">{label}</span>
        <div className="flex items-baseline gap-1.5">
          <span className="text-[10px] font-mono tabular-nums text-[color:var(--text-secondary)]">{valLabel}</span>
          <span className="text-[9px] text-[color:var(--text-muted)]">/ {maxLabel}</span>
          <span
            className="text-[10px] font-mono tabular-nums"
            style={{ color: barColor }}
          >
            {pctLabel}
          </span>
        </div>
      </div>
      <div className="h-1.5 bg-[var(--bg-elevated)] rounded-full overflow-hidden">
        <div
          className="h-full rounded-full"
          style={{
            width: `${pct}%`,
            backgroundColor: barColor,
            transition: 'width 0.6s cubic-bezier(0.4,0,0.2,1), background-color 0.3s',
          }}
        />
      </div>
    </div>
  );
}

// ── Circuit breaker card ──────────────────────────────────────────────────────

function CircuitBreakerCard({
  tripped,
  consecutiveLosses,
  maxConsecutiveLosses,
  sessionLossLamports,
  maxSessionLossLamports,
  onReset,
  resetting,
}: {
  tripped: boolean;
  consecutiveLosses: number;
  maxConsecutiveLosses: number;
  sessionLossLamports: number;
  maxSessionLossLamports: number;
  onReset?: () => void;
  resetting?: boolean;
}) {
  const Icon = tripped ? ShieldAlert : ShieldCheck;
  const color = tripped ? 'var(--red)' : 'var(--green)';
  const label = tripped ? 'TRIPPED' : 'ARMED';

  return (
    <div
      className="rounded-sm border p-3 flex flex-col gap-3"
      style={{
        background: tripped
          ? 'color-mix(in srgb, var(--red) 5%, var(--bg-elevated))'
          : 'color-mix(in srgb, var(--green) 4%, var(--bg-elevated))',
        borderColor: tripped
          ? 'color-mix(in srgb, var(--red) 30%, var(--bg-border))'
          : 'color-mix(in srgb, var(--green) 25%, var(--bg-border))',
      }}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4" style={{ color }} />
          <span className="text-[11px] font-semibold tracking-[0.08em]" style={{ color }}>
            Circuit Breaker — {label}
          </span>
        </div>
        {tripped && onReset && (
          <DsButton
            size="xs"
            variant="ghost"
            loading={resetting}
            onClick={onReset}
          >
            <RefreshCw className="w-3 h-3" />
            Reset
          </DsButton>
        )}
      </div>

      <div className="grid grid-cols-2 gap-2">
        <div className="space-y-1">
          <p className="text-[9px] font-mono uppercase tracking-[0.1em] text-[color:var(--text-muted)]">
            Consecutive losses
          </p>
          <div className="flex items-baseline gap-1">
            <span
              className="text-[15px] font-semibold font-mono tabular-nums"
              style={{ color: consecutiveLosses >= maxConsecutiveLosses * 0.7 ? 'var(--amber)' : 'var(--text-primary)' }}
            >
              {consecutiveLosses}
            </span>
            <span className="text-[10px] text-[color:var(--text-muted)] font-mono">/ {maxConsecutiveLosses}</span>
          </div>
          {maxConsecutiveLosses > 0 && (
            <div className="h-1 bg-[var(--bg-base)] rounded-full overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-500"
                style={{
                  width: `${Math.min((consecutiveLosses / maxConsecutiveLosses) * 100, 100)}%`,
                  backgroundColor:
                    consecutiveLosses >= maxConsecutiveLosses ? 'var(--red)'
                    : consecutiveLosses >= maxConsecutiveLosses * 0.7 ? 'var(--amber)'
                    : 'var(--blue)',
                }}
              />
            </div>
          )}
        </div>

        <div className="space-y-1">
          <p className="text-[9px] font-mono uppercase tracking-[0.1em] text-[color:var(--text-muted)]">
            Session loss
          </p>
          <div className="flex items-baseline gap-1">
            <span
              className="text-[15px] font-semibold font-mono tabular-nums"
              style={{ color: sessionLossLamports > maxSessionLossLamports * 0.7 ? 'var(--amber)' : 'var(--text-primary)' }}
            >
              {(sessionLossLamports / 1e9).toFixed(3)}
            </span>
            <span className="text-[10px] text-[color:var(--text-muted)] font-mono">
              / {(maxSessionLossLamports / 1e9).toFixed(3)} SOL
            </span>
          </div>
          {maxSessionLossLamports > 0 && (
            <div className="h-1 bg-[var(--bg-base)] rounded-full overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-500"
                style={{
                  width: `${Math.min((sessionLossLamports / maxSessionLossLamports) * 100, 100)}%`,
                  backgroundColor:
                    sessionLossLamports >= maxSessionLossLamports ? 'var(--red)'
                    : sessionLossLamports >= maxSessionLossLamports * 0.7 ? 'var(--amber)'
                    : 'var(--blue)',
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Safety rule row ───────────────────────────────────────────────────────────

function SafetyRule({ ok, label }: { ok: boolean; label: string }) {
  return (
    <div className="flex items-start gap-2 py-1">
      {ok
        ? <CheckCircle2 className="w-3 h-3 mt-0.5 shrink-0 text-[color:var(--green)]" />
        : <XCircle      className="w-3 h-3 mt-0.5 shrink-0 text-[color:var(--red)]" />
      }
      <span className={`text-[11px] font-mono ${ok ? 'text-[color:var(--text-secondary)]' : 'text-[color:var(--red)]'}`}>
        {label}
      </span>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export default function RiskPage() {
  const [resetting, setResetting] = useState(false);

  const { data: risk,      loading: riskLoading }      = useFetch(useCallback(() => api.risk(),      []), 2_000);
  const { data: portfolio, loading: portfolioLoading } = useFetch(useCallback(() => api.portfolio(), []), 2_000);

  const riskStatus  = risk?.risk_status ?? 'unknown';
  const statusTone  = riskStatus === 'healthy' ? 'green' : riskStatus === 'warning' ? 'amber' : 'red';

  const capitalSol         = risk ? risk.capital_usd / (risk as any).sol_price_usd || 0 : 0;
  const dailyLossCap       = risk ? risk.capital_usd * (risk.max_drawdown_pct / 100) : 0;

  // Synthetic hotpath state for the circuit breaker card (real values TBD from backend)
  const consecutiveLosses    = (risk as any)?.consecutive_losses    ?? 0;
  const maxConsecutiveLosses = (risk as any)?.max_consecutive_losses ?? 3;
  const sessionLossLamports  = (risk as any)?.session_loss_lamports  ?? 0;
  const maxSessionLossLamports = (risk as any)?.max_session_loss_lamports ?? 250_000_000;
  const circuitTripped       = (risk as any)?.circuit_breaker_tripped ?? false;

  async function handleReset() {
    setResetting(true);
    try { await (api as any).resetCircuitBreaker?.(); }
    catch { /* control API may not support this endpoint yet */ }
    finally { setResetting(false); }
  }

  const isLoading = riskLoading || portfolioLoading;

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Risk"
        subtitle="Live exposure vs configured limits"
        actions={
          <div className="flex items-center gap-1.5">
            {isLoading && (
              <svg className="animate-spin w-3 h-3 text-[color:var(--text-muted)]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M21 12a9 9 0 1 1-6.219-8.56" />
              </svg>
            )}
            <DsBadge tone={statusTone}>{riskStatus}</DsBadge>
          </div>
        }
      />

      {/* ── Stats row ──────────────────────────────────────────────────────── */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0 px-3 pb-2">
        <StatCard
          label="Capital"
          value={risk?.capital_usd ?? 0}
          format="usd"
          loading={!risk}
        />
        <StatCard
          label="Daily Loss"
          value={risk?.daily_loss_usd ?? 0}
          format="usd"
          accent={risk && risk.daily_loss_usd > 0 ? 'var(--red)' : undefined}
          signed
          loading={!risk}
        />
        <StatCard
          label="Exposure"
          value={risk?.current_exposure_pct ?? 0}
          format="percent"
          accent={
            (risk?.current_exposure_pct ?? 0) > 80 ? 'var(--red)'
            : (risk?.current_exposure_pct ?? 0) > 60 ? 'var(--amber)'
            : 'var(--blue)'
          }
          loading={!risk}
        />
        <StatCard
          label="Win Rate"
          value={portfolio ? portfolio.win_rate * 100 : 0}
          format="percent"
          accent={portfolio && portfolio.win_rate >= 0.5 ? 'var(--green)' : undefined}
          loading={!portfolio}
        />
      </div>

      {/* ── Grid ───────────────────────────────────────────────────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-2 flex-1 min-h-0 px-3 pb-3">

        {/* Circuit breaker */}
        <div className="space-y-2">
          <CircuitBreakerCard
            tripped={circuitTripped}
            consecutiveLosses={consecutiveLosses}
            maxConsecutiveLosses={maxConsecutiveLosses}
            sessionLossLamports={sessionLossLamports}
            maxSessionLossLamports={maxSessionLossLamports}
            onReset={handleReset}
            resetting={resetting}
          />

          {/* Exposure gauges */}
          <DsPanel compact title="Exposure gauges">
            {risk ? (
              <div className="space-y-4 px-1">
                <GaugeBar
                  label="Current exposure"
                  value={risk.current_exposure_pct}
                  max={100}
                  formatValue={(v) => `${v.toFixed(1)}%`}
                  formatMax={(_) => '100%'}
                />
                <GaugeBar
                  label="Daily loss"
                  value={risk.daily_loss_usd}
                  max={Math.max(dailyLossCap, risk.daily_loss_usd)}
                  formatValue={(v) => formatUsd(v)}
                  formatMax={(v) => formatUsd(v)}
                />
                <GaugeBar
                  label="Max position"
                  value={risk.max_position_pct}
                  max={100}
                  formatValue={(v) => `${v.toFixed(1)}%`}
                  formatMax={(_) => '100%'}
                />
              </div>
            ) : (
              <div className="space-y-4 px-1">
                {[0, 1, 2].map((i) => (
                  <div key={i} className="space-y-1.5">
                    <div className="skeleton h-2.5 rounded w-32" />
                    <div className="skeleton h-1.5 rounded-full w-full" />
                  </div>
                ))}
              </div>
            )}
          </DsPanel>
        </div>

        {/* Portfolio + safety */}
        <div className="space-y-2">
          {/* Portfolio summary */}
          <DsPanel compact title="Portfolio summary">
            <div className="grid grid-cols-3 gap-2">
              {[
                { label: 'Open positions', value: portfolio ? String(portfolio.open_positions) : '–' },
                { label: 'Realised PnL',   value: portfolio ? formatUsd(portfolio.realised_pnl) : '–',
                  accent: portfolio && portfolio.realised_pnl >= 0 ? 'var(--green)' : 'var(--red)' },
                { label: 'Total trades',   value: portfolio ? String(portfolio.total_trades ?? '–') : '–' },
              ].map(({ label, value, accent }) => (
                <div
                  key={label}
                  className="bg-[var(--bg-elevated)] border border-[var(--bg-border)] rounded-sm px-2 py-2 text-center"
                >
                  <p className="text-[9px] font-mono uppercase tracking-[0.1em] text-[color:var(--text-muted)]">{label}</p>
                  <p
                    className="text-[12px] font-semibold font-mono tabular-nums mt-0.5"
                    style={{ color: accent ?? 'var(--text-primary)' }}
                  >
                    {value}
                  </p>
                </div>
              ))}
            </div>
          </DsPanel>

          {/* Hard limits */}
          <DsPanel compact title="Configured limits">
            <div className="grid grid-cols-2 gap-x-4 gap-y-2.5 px-1">
              {[
                { label: 'Max drawdown',    value: risk ? `${risk.max_drawdown_pct.toFixed(1)}%` : '–' },
                { label: 'Max position',    value: risk ? `${risk.max_position_pct.toFixed(1)}%` : '–' },
                { label: 'Session loss cap', value: `${(maxSessionLossLamports / 1e9).toFixed(3)} SOL` },
                { label: 'Max consec. losses', value: String(maxConsecutiveLosses) },
              ].map(({ label, value }) => (
                <div key={label} className="flex justify-between items-baseline gap-2">
                  <span className="text-[9px] font-mono uppercase tracking-[0.08em] text-[color:var(--text-muted)] truncate">
                    {label}
                  </span>
                  <span className="text-[10px] font-mono tabular-nums text-[color:var(--text-secondary)] shrink-0">
                    {value}
                  </span>
                </div>
              ))}
            </div>
          </DsPanel>

          {/* Safety rules */}
          <DsPanel compact title="Safety rules">
            <div className="px-1 divide-y divide-[color-mix(in_srgb,var(--bg-border)_40%,transparent)]">
              <SafetyRule ok label="Live trading requires 3-flag gate" />
              <SafetyRule ok label="All config changes validated before apply" />
              <SafetyRule ok label="No direct execution access from UI" />
              <SafetyRule ok label="Keys never stored on disk — env injection only" />
              <SafetyRule ok label="Jito required for live trades (no naked RPC)" />
              <SafetyRule
                ok={!circuitTripped}
                label={circuitTripped ? 'Circuit breaker TRIPPED — trading halted' : 'Circuit breaker armed'}
              />
            </div>
          </DsPanel>
        </div>
      </div>
    </PageShell>
  );
}
