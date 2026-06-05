'use client';

import { useCallback, useState } from 'react';
import { useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd, type BotStatus } from '@/lib/types';

export default function BotPage() {
  const { data: bot, refetch } = useFetch<BotStatus>(
    useCallback(() => api.botStatus(), []),
    2_000,
  );
  const [busy, setBusy] = useState(false);

  const runAction = async (action: 'start' | 'stop') => {
    setBusy(true);
    try {
      if (action === 'start') await api.startSystem();
      else await api.stopSystem();
      await refetch();
    } finally {
      setBusy(false);
    }
  };

  const statusColor =
    bot?.trading_halted ? 'text-red-400' :
    bot?.running ? 'text-green-400' : 'text-slate-400';

  return (
    <div className="max-w-lg mx-auto space-y-4 pb-8">
      <div className="text-center pt-2">
        <h1 className="text-lg font-semibold text-slate-100">Trading Bot</h1>
        <p className="text-slate-400 text-xs mt-1">Scalping + DEX-to-DEX · Paper mode</p>
      </div>

      <div className={`rounded-xl border p-4 text-center ${
        bot?.running ? 'border-green-500/30 bg-green-500/5' : 'border-slate-600 bg-slate-800/50'
      }`}>
        <p className={`text-sm font-medium capitalize ${statusColor}`}>
          {bot?.trading_halted ? 'Halted' : bot?.running ? 'Running' : 'Stopped'}
        </p>
        {bot?.halt_reason && (
          <p className="text-xs text-red-400 mt-1">{bot.halt_reason}</p>
        )}
        <p className="text-2xl font-semibold mono text-slate-100 mt-2">
          {bot ? formatUsd(bot.net_pnl_usd) : '–'}
        </p>
        <p className="text-xs text-slate-500">Net PnL (paper)</p>
      </div>

      <div className="flex gap-3">
        <button
          onClick={() => runAction('start')}
          disabled={busy || bot?.running}
          className="flex-1 py-3 rounded-xl bg-green-600/20 border border-green-500/40 text-green-400 font-medium text-sm disabled:opacity-40"
        >
          Start Bot
        </button>
        <button
          onClick={() => runAction('stop')}
          disabled={busy || !bot?.running}
          className="flex-1 py-3 rounded-xl bg-red-600/20 border border-red-500/40 text-red-400 font-medium text-sm disabled:opacity-40"
        >
          Stop Bot
        </button>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <StatCard label="Scalp trades" value={bot?.scalp_trades ?? 0} pnl={bot?.scalp_pnl_usd} />
        <StatCard label="Arb trades" value={bot?.arb_trades ?? 0} pnl={bot?.arb_pnl_usd} />
        <StatCard label="Win rate" value={`${((bot?.win_rate ?? 0) * 100).toFixed(0)}%`} />
        <StatCard label="Risk" value={bot?.risk_status ?? '–'} />
      </div>

      <div className="rounded-xl border border-slate-700 bg-slate-800/50 p-4 space-y-2 text-xs text-slate-400">
        <p className="font-medium text-slate-300">Active strategies</p>
        <div className="flex gap-2">
          <Badge on={bot?.scalp_enabled} label="Scalping" />
          <Badge on={bot?.arb_enabled} label="DEX Arb" />
          <Badge on={bot?.mode === 'paper'} label="Paper" />
        </div>
        <p className="pt-2">
          Daily loss: {bot ? formatUsd(bot.daily_loss_usd) : '–'} ·
          Auto-halt on limit breach
        </p>
      </div>
    </div>
  );
}

function StatCard({ label, value, pnl }: { label: string; value: string | number; pnl?: number }) {
  return (
    <div className="rounded-xl border border-slate-700 bg-slate-800/50 p-3">
      <p className="text-[10px] uppercase tracking-wider text-slate-500">{label}</p>
      <p className="text-lg font-medium text-slate-100 mono mt-1">{value}</p>
      {pnl !== undefined && (
        <p className={`text-xs mono mt-0.5 ${pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
          {formatUsd(pnl)}
        </p>
      )}
    </div>
  );
}

function Badge({ on, label }: { on?: boolean; label: string }) {
  return (
    <span className={`px-2 py-1 rounded-md text-[10px] font-medium ${
      on ? 'bg-green-500/15 text-green-400' : 'bg-slate-700 text-slate-500'
    }`}>
      {label}
    </span>
  );
}
