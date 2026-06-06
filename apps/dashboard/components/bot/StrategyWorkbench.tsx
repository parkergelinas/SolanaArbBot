'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';

import BacktestOutlookBar from '@/components/bot/BacktestOutlookBar';
import { DsPanel } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { saveStrategyChoice } from '@/lib/backtest/applySelection';
import {
  confidenceTone,
  formatRoiPct,
  metricsForEngine,
  rankingForConfig,
  simulateEnriched,
} from '@/lib/backtest/metrics';
import { useStrategyBacktest } from '@/lib/hooks/useStrategyBacktest';
import {
  allPresets,
  enabledStrategyLabels,
  presetDisplayName,
  type StrategyPreset,
} from '@/lib/strategies/presets';
import { api } from '@/lib/api';
import { useBotStore } from '@/stores/botStore';
import type { BotStrategy } from '@/stores/botStore';

const STRATEGIES = [
  { key: 'scalp' as const, label: 'Scalping', desc: 'Short-term momentum entries' },
  { key: 'arb' as const, label: 'DEX Arb', desc: 'Cross-DEX spread capture' },
  { key: 'whale_copy' as const, label: 'Whale Copy', desc: 'Mirror large wallet flows' },
  { key: 'momentum' as const, label: 'Momentum', desc: 'Trend-following signals' },
  { key: 'sniper' as const, label: 'Sniper', desc: 'Fast entry on new pools' },
];

function ParamSlider({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="space-y-1 block">
      <span className="text-[10px] uppercase tracking-[0.12em] text-ds-text-muted">
        {label} · {format(value)}
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full accent-ds-blue h-1.5 touch-target"
      />
    </label>
  );
}

interface StrategyWorkbenchProps {
  paramInvalid: boolean;
  onSyncMsg: (msg: string | null) => void;
}

export default function StrategyWorkbench({ paramInvalid, onSyncMsg }: StrategyWorkbenchProps) {
  const strategies = useBotStore((s) => s.strategies);
  const activePresetId = useBotStore((s) => s.activePresetId);
  const customPresets = useBotStore((s) => s.customPresets);
  const toggleStrategy = useBotStore((s) => s.toggleStrategy);
  const setMinConfidence = useBotStore((s) => s.setMinConfidence);
  const setMinWhaleSol = useBotStore((s) => s.setMinWhaleSol);
  const setAutoCopyWhale = useBotStore((s) => s.setAutoCopyWhale);
  const setScalpTakeProfitPct = useBotStore((s) => s.setScalpTakeProfitPct);
  const setScalpStopLossPct = useBotStore((s) => s.setScalpStopLossPct);
  const setArbMinProfitUsd = useBotStore((s) => s.setArbMinProfitUsd);
  const setArbMaxLossUsd = useBotStore((s) => s.setArbMaxLossUsd);
  const applyPreset = useBotStore((s) => s.applyPreset);
  const saveCustomPreset = useBotStore((s) => s.saveCustomPreset);
  const deleteCustomPreset = useBotStore((s) => s.deleteCustomPreset);
  const refreshCustomPresets = useBotStore((s) => s.refreshCustomPresets);

  const [tab, setTab] = useState<'presets' | 'custom' | 'backtest'>('presets');
  const [customName, setCustomName] = useState('');
  const [customDesc, setCustomDesc] = useState('');
  const [syncing, setSyncing] = useState(false);
  const [backtestHours, setBacktestHours] = useState(6);

  const { report, loaded, hours: reportHours, activeMetrics, metricsForPreset } = useStrategyBacktest(
    strategies,
    activePresetId,
  );

  useEffect(() => {
    refreshCustomPresets();
  }, [refreshCustomPresets]);

  useEffect(() => {
    if (reportHours > 0) setBacktestHours(reportHours);
  }, [reportHours]);

  const presets = useMemo(() => allPresets(customPresets), [customPresets]);
  const activePreset = presets.find((p) => p.id === activePresetId);
  const activeLabel = presetDisplayName(activePresetId, customPresets);
  const enabled = enabledStrategyLabels(strategies);

  const liveMetrics = useMemo(
    () => simulateEnriched(strategies, report, backtestHours),
    [strategies, report, backtestHours],
  );

  const syncRankingChoice = useCallback(
    (config: typeof strategies) => {
      const match = rankingForConfig(config, report);
      if (match) saveStrategyChoice(match.id);
    },
    [report],
  );

  const syncConfig = async () => {
    setSyncing(true);
    onSyncMsg(null);
    try {
      await api.patchConfig(useBotStore.getState().toConfigPatch());
      onSyncMsg('Config synced — paper mode enforced.');
    } catch (e) {
      onSyncMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const applyAndSync = async (preset: StrategyPreset) => {
    applyPreset(preset);
    syncRankingChoice(preset.config);
    setSyncing(true);
    onSyncMsg(null);
    try {
      await api.patchConfig(useBotStore.getState().toConfigPatch());
      onSyncMsg(`Loaded "${preset.name}" and synced to API.`);
    } catch (e) {
      onSyncMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };

  return (
    <DsPanel
      className="shrink-0"
      title="Strategy Workbench"
      action={
        <div className="flex items-center gap-2">
          <span className="text-[9px] font-mono text-ds-text-muted hidden sm:inline">
            {enabled.join(' · ') || 'none'}
          </span>
          <button
            type="button"
            onClick={syncConfig}
            disabled={syncing || paramInvalid}
            className="text-[10px] px-2 py-0.5 rounded-terminal border border-ds-border text-ds-text-muted hover:text-ds-text-primary hover:border-ds-blue/30 disabled:opacity-50"
          >
            {syncing ? 'Syncing…' : 'Sync to API'}
          </button>
        </div>
      }
    >
      <div className="flex w-full p-0.5 gap-0.5 bg-ds-elevated border border-ds-border rounded-terminal mb-4">
        {(['presets', 'custom', 'backtest'] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`segment-btn flex-1 py-2 capitalize ${tab === t ? 'segment-btn-active' : ''}`}
          >
            {t === 'backtest' ? 'Backtest' : t}
          </button>
        ))}
      </div>

      <BacktestOutlookBar
        metrics={activeMetrics ?? liveMetrics}
        loaded={loaded}
        hours={backtestHours}
      />

      {tab === 'presets' && (
        <div className="space-y-3 mt-4">
          <p className="text-[10px] text-ds-text-muted leading-relaxed">
            Swap between built-in and saved presets. Active:{' '}
            <span className="text-ds-text-secondary font-medium">{activeLabel}</span>
            . Metrics from{' '}
            <span className="font-mono text-ds-blue">backtest_results.json</span>.
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
            {presets.map((preset) => {
              const active = activePresetId === preset.id;
              const engines = enabledStrategyLabels(preset.config);
              const est = loaded ? metricsForPreset(preset) : null;
              return (
                <button
                  key={preset.id}
                  type="button"
                  onClick={() => {
                    applyPreset(preset);
                    syncRankingChoice(preset.config);
                  }}
                  className={`text-left rounded-terminal border p-3 transition-colors min-h-[44px] ${
                    active
                      ? 'border-ds-blue bg-ds-blue/10'
                      : 'border-ds-border bg-ds-elevated/30 hover:border-ds-blue/30'
                  }`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="text-[11px] font-semibold text-ds-text-primary">{preset.name}</span>
                    <div className="flex items-center gap-1 shrink-0">
                      {est && (
                        <DsBadge tone={confidenceTone(est.confidenceScore)}>
                          {est.confidenceScore.toFixed(0)}%
                        </DsBadge>
                      )}
                      {preset.kind === 'custom' && (
                        <span className="text-[8px] uppercase text-ds-text-muted">custom</span>
                      )}
                    </div>
                  </div>
                  <p className="text-[9px] text-ds-text-muted mt-1 line-clamp-2">{preset.description}</p>
                  <p className="text-[9px] font-mono text-ds-text-secondary mt-2">{engines.join(' · ') || '—'}</p>
                  {est && (
                    <p className="text-[9px] font-mono mt-1.5 flex flex-wrap gap-x-2">
                      <span className="text-ds-green">{formatRoiPct(est.dailyRoiPct)}/day</span>
                      <span className="text-ds-text-muted">{formatRoiPct(est.monthlyRoiPct)}/mo</span>
                    </p>
                  )}
                </button>
              );
            })}
          </div>
          {activePreset && (
            <button
              type="button"
              onClick={() => applyAndSync(activePreset)}
              disabled={syncing || paramInvalid}
              className="touch-target px-3 py-2 rounded-terminal bg-ds-blue text-white text-[11px] font-medium hover:opacity-90 disabled:opacity-40"
            >
              Apply & sync {activePreset.name}
            </button>
          )}
        </div>
      )}

      {tab === 'custom' && (
        <div className="space-y-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <label className="block">
              <span className="text-[9px] uppercase tracking-wider text-ds-text-muted">Preset name</span>
              <input
                type="text"
                value={customName}
                onChange={(e) => setCustomName(e.target.value)}
                placeholder="My fusion strat"
                className="mt-1 w-full bg-ds-elevated border border-ds-border rounded-terminal px-3 py-2 text-[12px] text-ds-text-primary outline-none focus:border-ds-blue/40"
              />
            </label>
            <label className="block">
              <span className="text-[9px] uppercase tracking-wider text-ds-text-muted">Description</span>
              <input
                type="text"
                value={customDesc}
                onChange={(e) => setCustomDesc(e.target.value)}
                placeholder="Optional notes"
                className="mt-1 w-full bg-ds-elevated border border-ds-border rounded-terminal px-3 py-2 text-[12px] text-ds-text-primary outline-none focus:border-ds-blue/40"
              />
            </label>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
            {STRATEGIES.map(({ key, label, desc }) => {
              const solo = loaded ? metricsForEngine(key as BotStrategy, strategies, report, backtestHours) : null;
              return (
                <button
                  key={key}
                  type="button"
                  onClick={() => toggleStrategy(key)}
                  className={`touch-target text-left rounded-terminal border px-3 py-2.5 transition-all ${
                    strategies[key]
                      ? 'border-ds-blue/40 bg-ds-blue/8'
                      : 'border-ds-border bg-ds-elevated/30 opacity-70'
                  }`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <p className="text-[11px] font-semibold text-ds-text-primary">{label}</p>
                    {solo && (
                      <DsBadge tone={confidenceTone(solo.confidenceScore)}>
                        {solo.confidenceScore.toFixed(0)}%
                      </DsBadge>
                    )}
                  </div>
                  <p className="text-[9px] text-ds-text-muted mt-0.5">{desc}</p>
                  {solo && (
                    <p className="text-[9px] font-mono mt-1 text-ds-green">
                      {formatRoiPct(solo.dailyRoiPct)}/day est.
                    </p>
                  )}
                </button>
              );
            })}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-3 border-t border-ds-border">
            <div className="space-y-3">
              <p className="text-[9px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">Scalping exits</p>
              <ParamSlider
                label="Take profit"
                value={strategies.scalp_take_profit_pct}
                min={0.003}
                max={0.05}
                step={0.001}
                format={(v) => `${(v * 100).toFixed(2)}%`}
                onChange={setScalpTakeProfitPct}
              />
              <ParamSlider
                label="Stop loss"
                value={strategies.scalp_stop_loss_pct}
                min={0.002}
                max={0.03}
                step={0.001}
                format={(v) => `${(v * 100).toFixed(2)}%`}
                onChange={setScalpStopLossPct}
              />
            </div>
            <div className="space-y-3">
              <p className="text-[9px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">DEX arb limits</p>
              <ParamSlider
                label="Min profit"
                value={strategies.arb_min_profit_usd}
                min={0.05}
                max={5}
                step={0.05}
                format={(v) => `$${v.toFixed(2)}`}
                onChange={setArbMinProfitUsd}
              />
              <ParamSlider
                label="Max loss / trade"
                value={strategies.arb_max_loss_usd}
                min={0.5}
                max={10}
                step={0.5}
                format={(v) => `$${v.toFixed(2)}`}
                onChange={setArbMaxLossUsd}
              />
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-3 border-t border-ds-border">
            <ParamSlider
              label="Min confidence"
              value={strategies.min_confidence}
              min={0.4}
              max={0.95}
              step={0.05}
              format={(v) => `${(v * 100).toFixed(0)}%`}
              onChange={setMinConfidence}
            />
            <ParamSlider
              label="Min whale"
              value={strategies.min_whale_sol}
              min={10}
              max={500}
              step={10}
              format={(v) => `${v} SOL`}
              onChange={setMinWhaleSol}
            />
            <label className="flex items-center gap-2 cursor-pointer pt-4">
              <input
                type="checkbox"
                checked={strategies.auto_copy_whale}
                onChange={(e) => setAutoCopyWhale(e.target.checked)}
                className="rounded border-ds-border accent-ds-blue w-4 h-4"
              />
              <span className="text-[11px] text-ds-text-secondary">Auto copy whale (paper)</span>
            </label>
          </div>

          <div className="flex flex-wrap items-center gap-2 pt-2 border-t border-ds-border">
            <button
              type="button"
              onClick={() => {
                const saved = saveCustomPreset(customName, customDesc);
                if (saved) {
                  setCustomName('');
                  setCustomDesc('');
                  onSyncMsg(`Saved preset "${saved.name}".`);
                }
              }}
              disabled={!customName.trim()}
              className="touch-target px-3 py-2 rounded-terminal bg-ds-green/10 border border-ds-green/30 text-ds-green text-[11px] font-medium disabled:opacity-40"
            >
              Save as preset
            </button>
            {customPresets.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {customPresets.map((p) => (
                  <span
                    key={p.id}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded-terminal border border-ds-border text-[10px]"
                  >
                    {p.name}
                    <button
                      type="button"
                      onClick={() => deleteCustomPreset(p.id)}
                      className="text-ds-text-muted hover:text-ds-red"
                      aria-label={`Delete ${p.name}`}
                    >
                      ✕
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {tab === 'backtest' && (
        <div className="space-y-4">
          <p className="text-[10px] text-ds-text-muted leading-relaxed">
            Estimates holdout performance for the active config. Scalp/arb use{' '}
            <code className="font-mono text-ds-blue">backtest_results.json</code> when present; whale/momentum/sniper use heuristics.
            Run <code className="font-mono text-ds-green">cargo run -p backtester-app</code> for full pipeline replay.
          </p>

          <div className="flex flex-wrap items-end gap-3">
            <label className="block">
              <span className="text-[9px] uppercase tracking-wider text-ds-text-muted">Window (hours)</span>
              <select
                value={backtestHours}
                onChange={(e) => setBacktestHours(Number(e.target.value))}
                className="mt-1 block bg-ds-elevated border border-ds-border text-ds-text-primary text-[11px] font-mono rounded-terminal px-2 py-1.5"
              >
                {[3, 6, 12, 24].map((h) => (
                  <option key={h} value={h}>
                    {h}h
                  </option>
                ))}
              </select>
            </label>
            {!report && (
              <span className="text-[10px] text-ds-amber">No holdout file — heuristic mode</span>
            )}
          </div>

          <BacktestOutlookBar metrics={liveMetrics} loaded={loaded} hours={backtestHours} />

          {liveMetrics.enabledEngines.length > 0 && (
            <div className="rounded-terminal border border-ds-border bg-ds-elevated/20 p-3 space-y-3">
              <dl className="grid grid-cols-2 sm:grid-cols-3 gap-3 text-[10px]">
                <div>
                  <dt className="text-ds-text-muted uppercase text-[8px]">Net PnL ({backtestHours}h)</dt>
                  <dd className={`font-mono ${liveMetrics.netPnlUsd >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
                    ${liveMetrics.netPnlUsd.toFixed(0)}
                  </dd>
                </div>
                <div>
                  <dt className="text-ds-text-muted uppercase text-[8px]">Trades/day</dt>
                  <dd className="font-mono text-ds-text-secondary">{liveMetrics.tradesPerDay.toFixed(0)}</dd>
                </div>
                <div>
                  <dt className="text-ds-text-muted uppercase text-[8px]">Sharpe</dt>
                  <dd className="font-mono text-ds-text-secondary">{liveMetrics.sharpeApprox.toFixed(1)}</dd>
                </div>
              </dl>
              <ul className="text-[9px] text-ds-text-muted space-y-0.5 list-disc pl-4">
                {liveMetrics.notes.map((n) => (
                  <li key={n}>{n}</li>
                ))}
              </ul>
              <button
                type="button"
                onClick={() => {
                  syncRankingChoice(strategies);
                  void syncConfig();
                }}
                disabled={syncing || paramInvalid}
                className="text-[10px] px-2.5 py-1.5 rounded-terminal border border-ds-blue/40 text-ds-blue hover:bg-ds-blue/10 disabled:opacity-40"
              >
                Sync this config to API
              </button>
            </div>
          )}
        </div>
      )}

      {paramInvalid && (
        <p className="text-[11px] text-ds-red mt-3">
          Stop loss must be less than take profit; max loss must be positive.
        </p>
      )}
    </DsPanel>
  );
}
