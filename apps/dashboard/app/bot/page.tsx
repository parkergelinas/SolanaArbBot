'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

import WhaleFeed from '@/components/intelligence/WhaleFeed';
import LiveDataStatusCard from '@/components/LiveDataStatusCard';
import PaperTradingPanel from '@/components/paper/PaperTradingPanel';
import { DsPanel, PageHeader, PageShell } from '@/components/layout/PageShell';
import { api } from '@/lib/api';
import { useFetch, useLatestTrade } from '@/lib/hooks';
import { intelligenceToSignals } from '@/lib/intelligence/bridge';
import { useIntelConnected, useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { streamSignalsToEvents } from '@/lib/intelligence/streamBridge';
import { useMarketStore } from '@/stores/marketStore';
import { signalBotScore } from '@/lib/signals';
import { formatUsd, type BotStatus } from '@/lib/types';
import { useBotStore } from '@/stores/botStore';

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
        className="w-full accent-ds-blue h-1"
      />
    </label>
  );
}

export default function BotPage() {
  const { publicKey } = useWallet();
  const { connection } = useConnection();
  const [balance, setBalance] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);

  const intelConnected = useIntelConnected();
  const whales = useWhales();
  const smart = useSmartMoney();

  const {
    strategies,
    toggleStrategy,
    setMinConfidence,
    setMinWhaleSol,
    setAutoCopyWhale,
    setScalpTakeProfitPct,
    setScalpStopLossPct,
    setArbMinProfitUsd,
    setArbMaxLossUsd,
    recordWhaleCopy,
    lastWhaleCopyAt,
    pendingCopies,
    toConfigPatch,
  } = useBotStore();

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

  const syncConfig = async () => {
    setSyncing(true);
    setSyncMsg(null);
    try {
      await api.patchConfig(toConfigPatch());
      setSyncMsg('Config synced — paper mode enforced.');
    } catch (e) {
      setSyncMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const statusClass = bot?.trading_halted
    ? 'text-ds-red'
    : bot?.running
      ? 'text-ds-green'
      : 'text-ds-text-muted';

  const paramInvalid =
    strategies.scalp_stop_loss_pct >= strategies.scalp_take_profit_pct ||
    strategies.arb_max_loss_usd <= 0;

  return (
    <PageShell className="max-w-5xl gap-4">
      <PageHeader
        title="Trading Bot"
        description="Paper execution · strategy fusion · whale-aware copy logic"
        actions={
          publicKey ? (
            <div className="bg-ds-surface border border-ds-border rounded-terminal px-3 py-2 text-right">
              <p className="text-[10px] text-ds-text-muted uppercase tracking-wider">Wallet</p>
              <p className="text-xs font-mono text-ds-text-secondary truncate max-w-[200px]">
                {publicKey.toBase58().slice(0, 8)}…{publicKey.toBase58().slice(-6)}
              </p>
              {balance !== null && (
                <p className="text-sm font-semibold text-ds-blue font-mono mt-0.5">
                  {balance.toFixed(4)} SOL
                </p>
              )}
            </div>
          ) : undefined
        }
      />

      <div className="flex items-center gap-2 px-3 py-2 bg-ds-amber/8 border border-ds-amber/25 rounded-terminal text-[11px] text-ds-amber">
        <span className="font-mono uppercase tracking-wider shrink-0">Paper only</span>
        <span className="text-ds-text-secondary">
          Live trading requires SOLANA_ARB_CONFIRM_LIVE_TRADING=1 on the server — never enabled from this UI.
        </span>
      </div>

      <LiveDataStatusCard />
      <PaperTradingPanel />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-3">
        <DsPanel
          className={`lg:col-span-2 ${bot?.running ? 'border-ds-green/30' : ''}`}
          title="Bot Control"
          action={
            <span className={`text-[10px] font-mono uppercase ${statusClass}`}>
              {bot?.trading_halted ? 'Halted' : bot?.running ? 'Running' : 'Stopped'}
            </span>
          }
        >
          <div className="flex items-start justify-between gap-4">
            <div>
              {bot?.halt_reason && (
                <p className="text-[11px] text-ds-red mb-2">{bot.halt_reason}</p>
              )}
              <p className="text-3xl font-bold font-mono text-ds-text-primary">
                {bot ? formatUsd(bot.net_pnl_usd) : '–'}
              </p>
              <p className="text-[10px] text-ds-text-muted uppercase tracking-wider mt-0.5">
                Net PnL (paper)
              </p>
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

          <div className="flex gap-2 mt-4">
            <button
              type="button"
              onClick={() => runAction('start')}
              disabled={busy || bot?.running || paramInvalid}
              className="flex-1 py-2 rounded-terminal bg-ds-green/10 border border-ds-green/30 text-ds-green text-sm font-medium disabled:opacity-40 hover:bg-ds-green/20 transition-colors"
            >
              Start Bot
            </button>
            <button
              type="button"
              onClick={() => runAction('stop')}
              disabled={busy || !bot?.running}
              className="flex-1 py-2 rounded-terminal bg-ds-red/10 border border-ds-red/30 text-ds-red text-sm font-medium disabled:opacity-40 hover:bg-ds-red/20 transition-colors"
            >
              Stop Bot
            </button>
          </div>
        </DsPanel>

        <div className="grid grid-cols-2 gap-2">
          <StatCard label="Scalp" value={bot?.scalp_trades ?? 0} pnl={bot?.scalp_pnl_usd} />
          <StatCard label="Arb" value={bot?.arb_trades ?? 0} pnl={bot?.arb_pnl_usd} />
          <StatCard label="Win rate" value={`${((bot?.win_rate ?? 0) * 100).toFixed(0)}%`} />
          <StatCard label="Risk" value={bot?.risk_status ?? '–'} />
        </div>
      </div>

      <DsPanel
        title="Strategy Matrix"
        action={
          <button
            type="button"
            onClick={syncConfig}
            disabled={syncing || paramInvalid}
            className="text-[10px] px-2 py-0.5 rounded-terminal border border-ds-border text-ds-text-muted hover:text-ds-text-primary hover:border-ds-blue/30 disabled:opacity-50"
          >
            {syncing ? 'Syncing…' : 'Sync to API'}
          </button>
        }
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2 mb-4">
          {STRATEGIES.map(({ key, label, desc }) => (
            <button
              key={key}
              type="button"
              onClick={() => toggleStrategy(key)}
              className={`text-left rounded-terminal border px-3 py-2 transition-all ${
                strategies[key]
                  ? 'border-ds-blue/40 bg-ds-blue/8'
                  : 'border-ds-border bg-ds-elevated/30 opacity-70'
              }`}
            >
              <p className="text-[11px] font-semibold text-ds-text-primary">{label}</p>
              <p className="text-[9px] text-ds-text-muted mt-0.5">{desc}</p>
            </button>
          ))}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-3 border-t border-ds-border">
          <div className="space-y-3">
            <p className="text-[9px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
              Scalping exits
            </p>
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
            <p className="text-[9px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
              DEX arb limits
            </p>
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

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-4 mt-4 border-t border-ds-border">
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
              className="rounded border-ds-border accent-ds-blue"
            />
            <span className="text-[11px] text-ds-text-secondary">Auto copy whale (paper)</span>
          </label>
        </div>

        {paramInvalid && (
          <p className="text-[11px] text-ds-red mt-3">
            Stop loss must be less than take profit; max loss must be positive.
          </p>
        )}
        {syncMsg && (
          <p className={`text-[11px] mt-2 ${syncMsg.includes('synced') ? 'text-ds-green' : 'text-ds-amber'}`}>
            {syncMsg}
          </p>
        )}
      </DsPanel>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        <DsPanel title="Action Queue" action={<span className="text-[10px] text-ds-text-muted">{intelConnected ? `${actionable.length} signals` : 'Intel offline'}</span>}>
          {actionable.length === 0 ? (
            <p className="text-[11px] text-ds-text-muted py-6 text-center">
              No signals above threshold
            </p>
          ) : (
            <ul className="space-y-1.5">
              {actionable.map(({ signal, score }) => (
                <li
                  key={signal.signal_id}
                  className="rounded-terminal border border-ds-border bg-ds-elevated/30 px-2 py-1.5"
                >
                  <div className="flex justify-between text-[9px]">
                    <span className="text-ds-blue font-semibold uppercase">{signal.signal_type}</span>
                    <span className="font-mono text-ds-text-muted">score {(score * 100).toFixed(0)}</span>
                  </div>
                  <p className="text-[10px] text-ds-text-secondary mt-0.5 line-clamp-2">{signal.explanation}</p>
                </li>
              ))}
            </ul>
          )}
          {pendingCopies > 0 && (
            <p className="text-[10px] text-ds-blue mt-2">Paper copies triggered: {pendingCopies}</p>
          )}
        </DsPanel>
        <WhaleFeed compact />
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
    <div className="bg-ds-surface border border-ds-border rounded-terminal px-3 py-2.5">
      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted">{label}</p>
      <p className="text-lg font-semibold text-ds-text-primary font-mono mt-0.5">{value}</p>
      {pnl !== undefined && (
        <p className={`text-[11px] font-mono mt-0.5 ${pnl >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
          {formatUsd(pnl)}
        </p>
      )}
    </div>
  );
}
