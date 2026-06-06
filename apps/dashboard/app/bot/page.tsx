'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

import StrategyWorkbench from '@/components/bot/StrategyWorkbench';
import WhaleFeed from '@/components/intelligence/WhaleFeed';
import LiveDataStatusCard from '@/components/LiveDataStatusCard';
import PaperTradingPanel from '@/components/paper/PaperTradingPanel';
import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { enabledStrategyLabels, findPreset, presetDisplayName } from '@/lib/strategies/presets';
import { api } from '@/lib/api';
import { formatRoiPct } from '@/lib/backtest/metrics';
import { useFetch, useLatestTrade } from '@/lib/hooks';
import { useStrategyBacktest } from '@/lib/hooks/useStrategyBacktest';
import { intelligenceToSignals } from '@/lib/intelligence/bridge';
import { useIntelConnected, useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { streamSignalsToEvents } from '@/lib/intelligence/streamBridge';
import { useMarketStore } from '@/stores/marketStore';
import { signalBotScore } from '@/lib/signals';
import { formatUsd, type BotStatus } from '@/lib/types';
import { useBotStore } from '@/stores/botStore';

export default function BotPage() {
  const { publicKey } = useWallet();
  const { connection } = useConnection();
  const [balance, setBalance] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [mobileIntelTab, setMobileIntelTab] = useState<'queue' | 'whale'>('queue');
  const [hydrated, setHydrated] = useState(false);

  const intelConnected = useIntelConnected();
  const whales = useWhales();
  const smart = useSmartMoney();

  const strategies = useBotStore((s) => s.strategies);
  const applyPreset = useBotStore((s) => s.applyPreset);
  const activePresetId = useBotStore((s) => s.activePresetId);
  const customPresets = useBotStore((s) => s.customPresets);
  const hydrateFromApiConfig = useBotStore((s) => s.hydrateFromApiConfig);
  const refreshCustomPresets = useBotStore((s) => s.refreshCustomPresets);
  const recordWhaleCopy = useBotStore((s) => s.recordWhaleCopy);
  const lastWhaleCopyAt = useBotStore((s) => s.lastWhaleCopyAt);
  const pendingCopies = useBotStore((s) => s.pendingCopies);
  const toConfigPatch = useBotStore((s) => s.toConfigPatch);

  const { data: bot, refetch } = useFetch<BotStatus>(
    useCallback(() => api.botStatus(), []),
    2_000,
  );
  const latestTrade = useLatestTrade();

  useEffect(() => {
    if (!publicKey) {
      setBalance(null);
      return;
    }
    connection.getBalance(publicKey).then((lamports) => {
      setBalance(lamports / LAMPORTS_PER_SOL);
    }).catch(() => setBalance(null));
  }, [publicKey, connection]);

  useEffect(() => {
    let cancelled = false;
    refreshCustomPresets();
    const custom = useBotStore.getState().customPresets;
    const presetId = useBotStore.getState().activePresetId;
    const preset = findPreset(presetId, custom);

    if (presetId.startsWith('backtest-') || presetId === 'custom-draft') {
      setHydrated(true);
      return;
    }

    if (preset) {
      applyPreset(preset);
      setHydrated(true);
      return;
    }

    api
      .config()
      .then((cfg) => {
        if (!cancelled) hydrateFromApiConfig(cfg);
      })
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setHydrated(true);
      });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- hydrate once on mount
  }, []);

  const marketSignals = useMarketStore((s) => s.signals);

  const intelSignals = useMemo(() => {
    const fromIntel = intelligenceToSignals(whales, smart);
    const fromStream = streamSignalsToEvents(marketSignals).filter(
      (s) => s.signal_type === 'WhaleFlow' || s.signal_type === 'SmartMoney',
    );
    return [...fromIntel, ...fromStream].sort(
      (a, b) => b.timestamp_micros - a.timestamp_micros,
    );
  }, [whales, smart, marketSignals]);

  const actionable = useMemo(() => {
    return intelSignals
      .filter((s) => {
        if (s.signal_type === 'WhaleFlow') {
          const vol = s.feature_vector.volume_short;
          return vol >= strategies.min_whale_sol && s.confidence >= strategies.min_confidence;
        }
        return s.confidence >= strategies.min_confidence;
      })
      .map((s) => ({ signal: s, score: signalBotScore(s, strategies.min_confidence) }))
      .sort((a, b) => b.score - a.score)
      .slice(0, 5);
  }, [intelSignals, strategies.min_confidence, strategies.min_whale_sol]);

  useEffect(() => {
    if (!strategies.auto_copy_whale || !bot?.running || actionable.length === 0) return;
    const top = actionable[0];
    if (top.signal.signal_type !== 'WhaleFlow') return;
    const now = Date.now();
    if (lastWhaleCopyAt && now - lastWhaleCopyAt < 30_000) return;
    recordWhaleCopy();
  }, [actionable, strategies.auto_copy_whale, bot?.running, lastWhaleCopyAt, recordWhaleCopy]);

  const runAction = async (action: 'start' | 'stop') => {
    setBusy(true);
    setSyncMsg(null);
    try {
      if (action === 'start') {
        await api.patchConfig(toConfigPatch());
        await api.startSystem();
      } else {
        await api.stopSystem();
      }
      await refetch();
    } catch (e) {
      setSyncMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const statusClass = bot?.trading_halted
    ? 'text-ds-red'
    : bot?.running
      ? 'text-ds-green'
      : 'text-ds-text-muted';

  const enabledEngines = useMemo(() => enabledStrategyLabels(strategies), [strategies]);
  const noEngines = enabledEngines.length === 0;

  const paramInvalid =
    strategies.scalp_stop_loss_pct >= strategies.scalp_take_profit_pct ||
    strategies.arb_max_loss_usd <= 0;

  const activePresetLabel = presetDisplayName(activePresetId, customPresets);
  const { activeMetrics, loaded: backtestLoaded } = useStrategyBacktest(
    strategies,
    activePresetId,
  );

  return (
    <PageShell desk scroll>
      <CompactPageHeader
        title="Bot"
        subtitle="Paper execution · strategy fusion · whale-aware copy logic"
        actions={
          <div className="flex flex-wrap items-center gap-2 w-full sm:w-auto sm:ml-auto">
            {publicKey && (
              <div className="bg-ds-surface border border-ds-border rounded-terminal px-2.5 py-1.5 text-left sm:text-right min-w-0 flex-1 sm:flex-none">
                <p className="text-[9px] text-ds-text-muted uppercase tracking-wider">Wallet</p>
                <p className="text-[10px] font-mono text-ds-text-secondary truncate max-w-full sm:max-w-[180px]">
                  {publicKey.toBase58().slice(0, 8)}…{publicKey.toBase58().slice(-6)}
                </p>
                {balance !== null && (
                  <p className="text-[11px] font-semibold text-ds-blue font-mono tabular-nums">
                    {balance.toFixed(4)} SOL
                  </p>
                )}
              </div>
            )}
            <DsBadge tone={bot?.running ? 'green' : bot?.trading_halted ? 'red' : 'muted'}>
              {bot?.trading_halted ? 'Halted' : bot?.running ? 'Running' : 'Stopped'}
            </DsBadge>
          </div>
        }
      />

      <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 px-3 py-2 bg-ds-amber/8 border border-ds-amber/25 rounded-terminal text-[11px] text-ds-amber shrink-0">
        <span className="font-mono uppercase tracking-wider shrink-0">Paper only</span>
        <span className="text-ds-text-secondary leading-snug">
          Live trading requires server-side confirmation — never enabled from this UI.
        </span>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2 shrink-0">
        <DsStatPill label="Net PnL" value={bot ? formatUsd(bot.net_pnl_usd) : '–'} accent={bot && bot.net_pnl_usd >= 0 ? 'var(--green)' : 'var(--red)'} />
        <DsStatPill label="Scalp trades" value={bot?.scalp_trades ?? 0} />
        <DsStatPill label="Arb trades" value={bot?.arb_trades ?? 0} />
        <DsStatPill label="Win rate" value={`${((bot?.win_rate ?? 0) * 100).toFixed(0)}%`} accent="var(--blue)" />
        <DsStatPill
          label="Backtest conf."
          value={backtestLoaded && activeMetrics ? `${activeMetrics.confidenceScore.toFixed(0)}%` : '–'}
          accent="var(--green)"
        />
        <DsStatPill
          label="Est. daily ROI"
          value={backtestLoaded && activeMetrics ? formatRoiPct(activeMetrics.dailyRoiPct) : '–'}
          accent={activeMetrics && activeMetrics.dailyRoiPct >= 0 ? 'var(--green)' : 'var(--red)'}
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-2 shrink-0 items-start">
        <LiveDataStatusCard compact />
        <PaperTradingPanel />
      </div>

      {!hydrated ? (
        <DsPanel title="Strategy Workbench" className="shrink-0">
          <div className="space-y-3 animate-pulse">
            <div className="h-3 w-48 bg-ds-elevated rounded" />
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
              {Array.from({ length: 6 }).map((_, i) => (
                <div key={i} className="h-20 bg-ds-elevated/60 rounded-terminal" />
              ))}
            </div>
          </div>
        </DsPanel>
      ) : (
        <StrategyWorkbench paramInvalid={paramInvalid} onSyncMsg={setSyncMsg} />
      )}

      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,2fr)_minmax(11rem,1fr)] gap-2 shrink-0 items-start">
        <DsPanel
          className={`min-w-0 ${bot?.running ? 'border-ds-green/30' : ''}`}
          title="Bot Control"
          action={
            <span className={`text-[10px] font-mono uppercase ${statusClass}`}>
              {bot?.trading_halted ? 'Halted' : bot?.running ? 'Running' : 'Stopped'}
            </span>
          }
        >
          <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 sm:gap-4">
            <div className="min-w-0">
              {bot?.halt_reason && (
                <p className="text-[11px] text-ds-red mb-2">{bot.halt_reason}</p>
              )}
              <p className="text-2xl sm:text-3xl font-bold font-mono text-ds-text-primary tabular-nums">
                {bot ? formatUsd(bot.net_pnl_usd) : '–'}
              </p>
              <p className="text-[10px] text-ds-text-muted uppercase tracking-wider mt-0.5">
                Net PnL (paper)
              </p>
              <p className="text-[10px] text-ds-blue mt-2 truncate">
                Preset: {activePresetLabel}
              </p>
              <p className="text-[9px] font-mono text-ds-text-muted truncate">
                {enabledEngines.join(' · ') || 'No engines enabled'}
              </p>
              {backtestLoaded && activeMetrics && (
                <p className="text-[9px] font-mono mt-1.5 text-ds-green">
                  {activeMetrics.confidenceScore.toFixed(0)}% conf ·{' '}
                  {formatRoiPct(activeMetrics.dailyRoiPct)}/day ·{' '}
                  {formatRoiPct(activeMetrics.monthlyRoiPct)}/mo
                </p>
              )}
            </div>
            <div className="text-right text-[10px] font-mono text-ds-text-muted space-y-1">
              <p>{bot?.runtime_mode ?? bot?.mode ?? 'disabled'}</p>
              {bot?.active_strategies && bot.active_strategies.length > 0 && (
                <p>{bot.active_strategies.join(' · ')}</p>
              )}
            </div>
          </div>

          {latestTrade && (
            <div className="mt-3 rounded-terminal border border-ds-border bg-ds-elevated/40 px-3 py-2">
              <p className="text-[9px] uppercase tracking-wider text-ds-text-muted">Latest trade</p>
              <div className="flex items-center justify-between gap-2 mt-1">
                <span className="text-[11px] text-ds-text-secondary capitalize">
                  {latestTrade.source_strategy} · {latestTrade.stage}
                </span>
                <span
                  className={`text-[11px] font-mono font-medium ${
                    latestTrade.stage === 'filled'
                      ? 'text-ds-green'
                      : latestTrade.stage === 'rejected' || latestTrade.stage === 'failed'
                        ? 'text-ds-red'
                        : 'text-ds-blue'
                  }`}
                >
                  {latestTrade.expected_pnl_usd >= 0 ? '+' : ''}
                  {formatUsd(latestTrade.expected_pnl_usd)}
                </span>
              </div>
            </div>
          )}

          <div className="flex flex-col sm:flex-row gap-2 mt-4">
            <button
              type="button"
              onClick={() => runAction('start')}
              disabled={busy || bot?.running || paramInvalid || noEngines}
              className="touch-target flex-1 py-2.5 sm:py-2 rounded-terminal bg-ds-green/10 border border-ds-green/30 text-ds-green text-sm font-medium disabled:opacity-40 hover:bg-ds-green/20 transition-colors"
            >
              Start Bot
            </button>
            <button
              type="button"
              onClick={() => runAction('stop')}
              disabled={busy || !bot?.running}
              className="touch-target flex-1 py-2.5 sm:py-2 rounded-terminal bg-ds-red/10 border border-ds-red/30 text-ds-red text-sm font-medium disabled:opacity-40 hover:bg-ds-red/20 transition-colors"
            >
              Stop Bot
            </button>
          </div>
          {(paramInvalid || noEngines) && (
            <p className="text-[10px] text-ds-amber mt-2">
              {noEngines
                ? 'Enable at least one strategy in the workbench before starting.'
                : 'Fix scalping exits and arb limits in the workbench before starting.'}
            </p>
          )}
        </DsPanel>

        <div className="grid grid-cols-2 lg:grid-cols-1 gap-2 min-w-0 shrink-0">
          <StatCard label="Scalp" value={bot?.scalp_trades ?? 0} pnl={bot?.scalp_pnl_usd} />
          <StatCard label="Arb" value={bot?.arb_trades ?? 0} pnl={bot?.arb_pnl_usd} />
          <StatCard label="Win rate" value={`${((bot?.win_rate ?? 0) * 100).toFixed(0)}%`} />
          <StatCard label="Risk" value={bot?.risk_status ?? '–'} />
        </div>
      </div>

      {syncMsg && (
        <p
          className={`text-[11px] shrink-0 px-1 ${
            syncMsg.toLowerCase().includes('sync') ? 'text-ds-green' : 'text-ds-amber'
          }`}
        >
          {syncMsg}
        </p>
      )}

      <div className="lg:hidden flex w-full p-0.5 gap-0.5 bg-ds-elevated border border-ds-border rounded-terminal shrink-0">
        <button
          type="button"
          onClick={() => setMobileIntelTab('queue')}
          className={`segment-btn flex-1 py-2 ${mobileIntelTab === 'queue' ? 'segment-btn-active' : ''}`}
        >
          Queue ({actionable.length})
        </button>
        <button
          type="button"
          onClick={() => setMobileIntelTab('whale')}
          className={`segment-btn flex-1 py-2 ${mobileIntelTab === 'whale' ? 'segment-btn-active' : ''}`}
        >
          Whale Radar
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-2 shrink-0 items-start">
        <DsPanel
          className={`min-h-0 min-w-0 ${mobileIntelTab === 'queue' ? 'flex' : 'hidden lg:flex'} flex-col`}
          title="Action Queue"
          action={
            <span className="text-[10px] text-ds-text-muted">
              {intelConnected ? `${actionable.length} signals` : 'Intel offline'}
            </span>
          }
        >
          {actionable.length === 0 ? (
            <p className="text-[11px] text-ds-text-muted py-6 text-center">
              No signals above threshold
            </p>
          ) : (
            <ul className="space-y-1.5 max-h-64 lg:max-h-none overflow-y-auto terminal-scroll">
              {actionable.map(({ signal, score }) => (
                <li
                  key={signal.signal_id}
                  className="rounded-terminal border border-ds-border bg-ds-elevated/30 px-2 py-1.5"
                >
                  <div className="flex justify-between gap-2 text-[9px]">
                    <span className="text-ds-blue font-semibold uppercase shrink-0">{signal.signal_type}</span>
                    <span className="font-mono text-ds-text-muted shrink-0">score {(score * 100).toFixed(0)}</span>
                  </div>
                  <p className="text-[10px] text-ds-text-secondary mt-0.5 line-clamp-2 break-words">
                    {signal.explanation}
                  </p>
                </li>
              ))}
            </ul>
          )}
          {pendingCopies > 0 && (
            <p className="text-[10px] text-ds-blue mt-2 shrink-0">Paper copies triggered: {pendingCopies}</p>
          )}
        </DsPanel>
        <div className={`min-w-0 ${mobileIntelTab === 'whale' ? 'block' : 'hidden lg:block'}`}>
          <WhaleFeed compact />
        </div>
      </div>
    </PageShell>
  );
}

function StatCard({
  label,
  value,
  pnl,
}: {
  label: string;
  value: string | number;
  pnl?: number;
}) {
  return (
    <div className="bg-ds-surface border border-ds-border rounded-terminal px-3 py-2.5 min-w-0 overflow-hidden">
      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted truncate">{label}</p>
      <p className="text-base sm:text-lg font-semibold text-ds-text-primary font-mono mt-0.5 tabular-nums truncate">
        {value}
      </p>
      {pnl !== undefined && (
        <p className={`text-[11px] font-mono mt-0.5 tabular-nums truncate ${pnl >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
          {formatUsd(pnl)}
        </p>
      )}
    </div>
  );
}
