'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

import WhaleFeed from '@/components/intelligence/WhaleFeed';
import { api } from '@/lib/api';
import { useFetch } from '@/lib/hooks';
import { intelligenceToSignals } from '@/lib/intelligence/bridge';
import { useIntelConnected, useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
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

export default function BotPage() {
  const { publicKey } = useWallet();
  const { connection } = useConnection();
  const [balance, setBalance] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const intelConnected = useIntelConnected();
  const whales = useWhales();
  const smart = useSmartMoney();

  const {
    strategies,
    toggleStrategy,
    setMinConfidence,
    setMinWhaleSol,
    setAutoCopyWhale,
    recordWhaleCopy,
    lastWhaleCopyAt,
    pendingCopies,
    toConfigPatch,
  } = useBotStore();

  const { data: bot, refetch } = useFetch<BotStatus>(
    useCallback(() => api.botStatus(), []),
    2_000,
  );

  useEffect(() => {
    if (!publicKey) {
      setBalance(null);
      return;
    }
    connection.getBalance(publicKey).then((lamports) => {
      setBalance(lamports / LAMPORTS_PER_SOL);
    }).catch(() => setBalance(null));
  }, [publicKey, connection]);

  const intelSignals = useMemo(
    () => intelligenceToSignals(whales, smart),
    [whales, smart],
  );

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
    try {
      if (action === 'start') {
        await api.patchConfig(toConfigPatch());
        await api.startSystem();
      } else {
        await api.stopSystem();
      }
      await refetch();
    } finally {
      setBusy(false);
    }
  };

  const syncConfig = async () => {
    setSyncing(true);
    try {
      await api.patchConfig(toConfigPatch());
    } finally {
      setSyncing(false);
    }
  };

  const statusColor = bot?.trading_halted
    ? 'text-red-400'
    : bot?.running
      ? 'text-platform-accent'
      : 'text-slate-400';

  return (
    <div className="max-w-5xl mx-auto space-y-5 pb-8">
      <header className="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold text-slate-100 tracking-tight">Trading Bot</h1>
          <p className="text-platform-muted text-sm mt-0.5">
            Paper execution · strategy fusion · whale-aware copy logic
          </p>
        </div>
        {publicKey && (
          <div className="glass-card px-3 py-2 text-right">
            <p className="text-[10px] text-platform-muted uppercase tracking-wider">Wallet</p>
            <p className="text-xs mono text-slate-300 truncate max-w-[200px]">
              {publicKey.toBase58().slice(0, 8)}…{publicKey.toBase58().slice(-6)}
            </p>
            {balance !== null && (
              <p className="text-sm font-semibold text-platform-accent mono mt-0.5">
                {balance.toFixed(4)} SOL
              </p>
            )}
          </div>
        )}
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div
          className={`lg:col-span-2 rounded-2xl border p-5 ${
            bot?.running
              ? 'border-platform-accent/30 bg-gradient-to-br from-platform-accent/8 to-transparent'
              : 'border-platform-border glass-card'
          }`}
        >
          <div className="flex items-center justify-between">
            <div>
              <p className={`text-sm font-semibold capitalize ${statusColor}`}>
                {bot?.trading_halted ? 'Halted' : bot?.running ? 'Running' : 'Stopped'}
              </p>
              {bot?.halt_reason && (
                <p className="text-xs text-red-400 mt-0.5">{bot.halt_reason}</p>
              )}
            </div>
            <span className="text-[10px] uppercase tracking-wider text-platform-muted px-2 py-1 rounded-md border border-platform-border">
              {bot?.mode ?? 'paper'}
            </span>
          </div>
          <p className="text-3xl font-bold mono text-slate-100 mt-3">
            {bot ? formatUsd(bot.net_pnl_usd) : '–'}
          </p>
          <p className="text-xs text-platform-muted">Net PnL (paper)</p>

          <div className="flex gap-3 mt-5">
            <button
              type="button"
              onClick={() => runAction('start')}
              disabled={busy || bot?.running}
              className="flex-1 py-2.5 rounded-xl bg-platform-accent/20 border border-platform-accent/40 text-platform-accent font-medium text-sm disabled:opacity-40 hover:bg-platform-accent/30 transition-colors"
            >
              Start Bot
            </button>
            <button
              type="button"
              onClick={() => runAction('stop')}
              disabled={busy || !bot?.running}
              className="flex-1 py-2.5 rounded-xl bg-red-600/15 border border-red-500/35 text-red-400 font-medium text-sm disabled:opacity-40 hover:bg-red-600/25 transition-colors"
            >
              Stop Bot
            </button>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <StatCard label="Scalp" value={bot?.scalp_trades ?? 0} pnl={bot?.scalp_pnl_usd} />
          <StatCard label="Arb" value={bot?.arb_trades ?? 0} pnl={bot?.arb_pnl_usd} />
          <StatCard label="Win rate" value={`${((bot?.win_rate ?? 0) * 100).toFixed(0)}%`} />
          <StatCard label="Risk" value={bot?.risk_status ?? '–'} />
        </div>
      </div>

      <section className="glass-card p-4 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-slate-200">Strategy Matrix</h2>
          <button
            type="button"
            onClick={syncConfig}
            disabled={syncing}
            className="text-xs px-3 py-1.5 rounded-lg border border-platform-border text-platform-muted hover:text-slate-200 hover:border-platform-accent/30 transition-colors disabled:opacity-50"
          >
            {syncing ? 'Syncing…' : 'Sync to API'}
          </button>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
          {STRATEGIES.map(({ key, label, desc }) => (
            <button
              key={key}
              type="button"
              onClick={() => toggleStrategy(key)}
              className={`text-left rounded-xl border px-3 py-2.5 transition-all ${
                strategies[key]
                  ? 'border-platform-accent/40 bg-platform-accent/10'
                  : 'border-platform-border/60 bg-platform-bg/40 opacity-70'
              }`}
            >
              <p className="text-xs font-semibold text-slate-200">{label}</p>
              <p className="text-[10px] text-platform-muted mt-0.5">{desc}</p>
            </button>
          ))}
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-2 border-t border-platform-border/60">
          <label className="space-y-1">
            <span className="text-[10px] uppercase tracking-wider text-platform-muted">
              Min confidence {(strategies.min_confidence * 100).toFixed(0)}%
            </span>
            <input
              type="range"
              min={0.4}
              max={0.95}
              step={0.05}
              value={strategies.min_confidence}
              onChange={(e) => setMinConfidence(parseFloat(e.target.value))}
              className="w-full accent-platform-accent"
            />
          </label>
          <label className="space-y-1">
            <span className="text-[10px] uppercase tracking-wider text-platform-muted">
              Min whale {strategies.min_whale_sol} SOL
            </span>
            <input
              type="range"
              min={10}
              max={500}
              step={10}
              value={strategies.min_whale_sol}
              onChange={(e) => setMinWhaleSol(parseInt(e.target.value, 10))}
              className="w-full accent-platform-accent"
            />
          </label>
          <label className="flex items-center gap-2 cursor-pointer pt-4">
            <input
              type="checkbox"
              checked={strategies.auto_copy_whale}
              onChange={(e) => setAutoCopyWhale(e.target.checked)}
              className="rounded border-platform-border accent-platform-accent"
            />
            <span className="text-xs text-slate-300">Auto copy whale (paper)</span>
          </label>
        </div>
      </section>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <section className="glass-card p-4">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-semibold text-slate-200">Action Queue</h2>
            <span className="text-[10px] text-platform-muted">
              {intelConnected ? `${actionable.length} signals` : 'Intel offline'}
            </span>
          </div>
          {actionable.length === 0 ? (
            <p className="text-xs text-platform-muted py-6 text-center">
              No signals above threshold — tune confidence or start intelligence-api
            </p>
          ) : (
            <ul className="space-y-2">
              {actionable.map(({ signal, score }) => (
                <li
                  key={signal.signal_id}
                  className="rounded-lg border border-platform-border/60 bg-platform-bg/50 px-3 py-2"
                >
                  <div className="flex justify-between text-[10px]">
                    <span className="text-platform-accent font-semibold uppercase">
                      {signal.signal_type}
                    </span>
                    <span className="mono text-platform-muted">score {(score * 100).toFixed(0)}</span>
                  </div>
                  <p className="text-xs text-slate-300 mt-1 line-clamp-2">{signal.explanation}</p>
                </li>
              ))}
            </ul>
          )}
          {pendingCopies > 0 && (
            <p className="text-[10px] text-platform-accent mt-2">
              Paper copies triggered: {pendingCopies}
            </p>
          )}
        </section>

        <WhaleFeed compact />
      </div>
    </div>
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
    <div className="glass-card p-3">
      <p className="text-[10px] uppercase tracking-wider text-platform-muted">{label}</p>
      <p className="text-lg font-semibold text-slate-100 mono mt-1">{value}</p>
      {pnl !== undefined && (
        <p className={`text-xs mono mt-0.5 ${pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
          {formatUsd(pnl)}
        </p>
      )}
    </div>
  );
}
