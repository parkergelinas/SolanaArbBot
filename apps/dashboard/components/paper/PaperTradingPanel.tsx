'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';

import { DsPanel } from '@/components/layout/PageShell';
import { fetchTokenBalances } from '@/lib/solana/connection';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useNetworkStore } from '@/stores/networkStore';
import { usePaperStore } from '@/stores/paperStore';
import { useUiStore } from '@/stores/uiStore';

const TRADE_SIZES = [0.1, 0.25, 0.5, 1];

export default function PaperTradingPanel({ compact = false }: { compact?: boolean }) {
  const cluster = useNetworkStore((s) => s.cluster);
  const { publicKey } = useWallet();
  const { connection } = useConnection();

  const portfolio = usePaperStore((s) => s.portfolio);
  const enabled = usePaperStore((s) => s.enabled);
  const autoTrade = usePaperStore((s) => s.autoTrade);
  const lastFill = usePaperStore((s) => s.lastFill);
  const error = usePaperStore((s) => s.error);
  const initForCluster = usePaperStore((s) => s.initForCluster);
  const setEnabled = usePaperStore((s) => s.setEnabled);
  const setAutoTrade = usePaperStore((s) => s.setAutoTrade);
  const quickTrade = usePaperStore((s) => s.quickTrade);
  const reset = usePaperStore((s) => s.reset);
  const equityUsd = usePaperStore((s) => s.equityUsd);

  const selectedMint = useUiStore((s) => s.selectedMint);
  const priceRecords = useMarketStore((s) => s.prices);
  const tokenRecords = useMarketStore((s) => s.tokens);
  const prices = useMemo(() => {
    const out: Record<string, number> = {};
    for (const [mint, p] of Object.entries(priceRecords)) out[mint] = p.price_usd;
    for (const [mint, t] of Object.entries(tokenRecords)) {
      if (!out[mint]) out[mint] = t.price_usd;
    }
    return out;
  }, [priceRecords, tokenRecords]);

  const [chainSol, setChainSol] = useState<number | null>(null);
  const [chainTokens, setChainTokens] = useState(0);

  useEffect(() => {
    initForCluster(cluster);
  }, [cluster, initForCluster]);

  const refreshChain = useCallback(async () => {
    if (!publicKey) {
      setChainSol(null);
      setChainTokens(0);
      return;
    }
    try {
      const lamports = await connection.getBalance(publicKey);
      setChainSol(lamports / 1e9);
      const tokens = await fetchTokenBalances(cluster, publicKey);
      setChainTokens(tokens.length);
    } catch {
      setChainSol(null);
      setChainTokens(0);
    }
  }, [publicKey, connection, cluster]);

  useEffect(() => {
    refreshChain();
    const id = setInterval(refreshChain, 15_000);
    return () => clearInterval(id);
  }, [refreshChain]);

  const equity = useMemo(() => equityUsd(prices), [equityUsd, prices, portfolio]);

  const paperSol = portfolio?.balances['So11111111111111111111111111111111111111112'] ?? 0;
  const targetMint = selectedMint ?? 'So11111111111111111111111111111111111111112';
  const targetSymbol = tokenSymbol(targetMint);

  const onTrade = (side: 'buy' | 'sell', solAmt: number) => {
    if (targetMint.startsWith('So1111')) return;
    quickTrade(targetMint, side, solAmt, prices);
  };

  if (compact) {
    return (
      <div className="px-2 py-1.5 border-t border-ds-border bg-ds-surface text-[10px]">
        <div className="flex items-center justify-between gap-2">
          <span className="text-ds-text-muted uppercase tracking-wider">Paper</span>
          <span className="font-mono text-ds-blue font-medium tabular-nums">
            ${equity.toLocaleString(undefined, { maximumFractionDigits: 0 })}
          </span>
        </div>
        {lastFill && (
          <p className="text-[9px] text-ds-text-muted truncate mt-0.5 font-mono">
            {lastFill.side} {tokenSymbol(lastFill.tokenOut)} · {lastFill.pnlUsd >= 0 ? '+' : ''}
            {lastFill.pnlUsd.toFixed(2)} USD
          </p>
        )}
      </div>
    );
  }

  return (
    <DsPanel
      title="Paper trading"
      compact
      action={
        <label className="flex items-center gap-1.5 cursor-pointer">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
            className="rounded border-ds-border accent-ds-blue"
          />
          <span className="text-[10px] text-ds-text-secondary uppercase tracking-wider">Enabled</span>
        </label>
      }
    >
      <p className="text-[10px] text-ds-text-muted mb-3 -mt-1">
        Simulated fills · {cluster === 'devnet' ? 'Devnet' : 'Mainnet'} cluster
      </p>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
        <Metric label="Paper equity" value={`$${equity.toFixed(2)}`} accent />
        <Metric label="Paper SOL" value={paperSol.toFixed(3)} />
        <Metric
          label="Real SOL"
          value={chainSol !== null ? chainSol.toFixed(4) : '—'}
          hint={publicKey ? `${chainTokens} tokens` : 'No wallet'}
        />
        <Metric
          label="Realized PnL"
          value={`$${(portfolio?.realizedPnlUsd ?? 0).toFixed(2)}`}
          positive={(portfolio?.realizedPnlUsd ?? 0) >= 0}
        />
      </div>

      {error && <p className="text-[11px] text-ds-red mt-2">{error}</p>}

      <div className="flex flex-wrap items-center gap-2 mt-3">
        <span className="text-[10px] text-ds-text-muted uppercase tracking-wider">Quick {targetSymbol}</span>
        {TRADE_SIZES.map((amt) => (
          <button
            key={`buy-${amt}`}
            type="button"
            disabled={!enabled || targetMint.startsWith('So1111')}
            onClick={() => onTrade('buy', amt)}
            className="px-2 py-1 rounded-terminal text-[10px] font-mono border border-ds-green/30 text-ds-green hover:bg-ds-green/10 disabled:opacity-40"
          >
            Buy {amt} SOL
          </button>
        ))}
        {TRADE_SIZES.slice(0, 2).map((amt) => (
          <button
            key={`sell-${amt}`}
            type="button"
            disabled={!enabled || targetMint.startsWith('So1111')}
            onClick={() => onTrade('sell', amt)}
            className="px-2 py-1 rounded-terminal text-[10px] font-mono border border-ds-red/30 text-ds-red hover:bg-ds-red/10 disabled:opacity-40"
          >
            Sell → {amt} SOL
          </button>
        ))}
      </div>

      <div className="flex items-center justify-between pt-3 mt-3 border-t border-ds-border">
        <label className="flex items-center gap-2 cursor-pointer text-[11px] text-ds-text-secondary">
          <input
            type="checkbox"
            checked={autoTrade}
            onChange={(e) => setAutoTrade(e.target.checked)}
            className="rounded border-ds-border accent-ds-blue"
          />
          Auto paper on stream signals
        </label>
        <button
          type="button"
          onClick={() => reset(cluster)}
          className="text-[10px] text-ds-text-muted hover:text-ds-red transition-colors"
        >
          Reset portfolio
        </button>
      </div>

      {lastFill && (
        <div className="rounded-terminal border border-ds-border bg-ds-elevated/40 px-3 py-2 text-[11px] mt-2">
          <span className="text-ds-text-muted">Last fill · </span>
          <span className={`font-mono ${lastFill.pnlUsd >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
            {tokenSymbol(lastFill.tokenIn)} → {tokenSymbol(lastFill.tokenOut)} ·{' '}
            {lastFill.pnlUsd >= 0 ? '+' : ''}
            {lastFill.pnlUsd.toFixed(2)} USD
          </span>
        </div>
      )}
    </DsPanel>
  );
}

function Metric({
  label,
  value,
  hint,
  accent,
  positive,
}: {
  label: string;
  value: string;
  hint?: string;
  accent?: boolean;
  positive?: boolean;
}) {
  return (
    <div className="rounded-terminal border border-ds-border bg-ds-elevated/30 px-2.5 py-2">
      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted">{label}</p>
      <p
        className={`text-sm font-semibold font-mono tabular-nums mt-0.5 ${
          positive === true
            ? 'text-ds-green'
            : positive === false
              ? 'text-ds-red'
              : accent
                ? 'text-ds-blue'
                : 'text-ds-text-primary'
        }`}
      >
        {value}
      </p>
      {hint && <p className="text-[9px] text-ds-text-muted">{hint}</p>}
    </div>
  );
}
