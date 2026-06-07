'use client';

import { useMemo, useState, useCallback } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';

import { formatPnL, formatPrice } from '@/lib/formatters';
import { useResolvedPrices } from '@/lib/hooks/useResolvedPrice';
import { useLiveSwap } from '@/lib/hooks/useLiveSwap';
import { SOL_MINT, tokenSymbol, tokenMeta } from '@/lib/terminal/tokens';
import { useNetworkStore } from '@/stores/networkStore';
import { usePaperStore } from '@/stores/paperStore';
import { useUiStore } from '@/stores/uiStore';
import LiveTradeConfirmDialog from './LiveTradeConfirmDialog';

const SIZES = [0.1, 0.25, 0.5, 1] as const;

type TradeMode = 'paper' | 'live';

export default function OrderTicket() {
  const cluster = useNetworkStore((s) => s.cluster);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const portfolio = usePaperStore((s) => s.portfolio);
  const enabled = usePaperStore((s) => s.enabled);
  const autoTrade = usePaperStore((s) => s.autoTrade);
  const lastFill = usePaperStore((s) => s.lastFill);
  const error = usePaperStore((s) => s.error);
  const setEnabled = usePaperStore((s) => s.setEnabled);
  const setAutoTrade = usePaperStore((s) => s.setAutoTrade);
  const quickTrade = usePaperStore((s) => s.quickTrade);
  const reset = usePaperStore((s) => s.reset);
  const equityUsd = usePaperStore((s) => s.equityUsd);

  const { connected } = useWallet();
  const swap = useLiveSwap();

  // Live mode is only available on mainnet with a connected wallet.
  const liveAvailable = cluster === 'mainnet-beta' && connected;
  const [mode, setMode] = useState<TradeMode>('paper');

  // Ensure we can't be in live mode without a wallet/mainnet.
  const effectiveMode: TradeMode = liveAvailable ? mode : 'paper';

  const priceMints = useMemo(() => {
    const mints = new Set<string>([SOL_MINT]);
    if (selectedMint) mints.add(selectedMint);
    if (portfolio) {
      for (const mint of Object.keys(portfolio.balances)) mints.add(mint);
    }
    return Array.from(mints);
  }, [selectedMint, portfolio]);

  const resolved = useResolvedPrices(priceMints);
  const prices = useMemo(() => {
    const out: Record<string, number> = {};
    for (const [mint, r] of Object.entries(resolved)) out[mint] = r.priceUsd;
    return out;
  }, [resolved]);

  const equity = equityUsd(prices);
  const solBal = portfolio?.balances[SOL_MINT] ?? 0;
  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const canTrade = enabled && selectedMint && !selectedMint.startsWith('So1111');
  const selectedMeta = selectedMint ? tokenMeta(selectedMint) : undefined;

  // ── Paper trade handler ────────────────────────────────────────────────────
  const paperTrade = (side: 'buy' | 'sell', sol: number) => {
    if (!selectedMint || !canTrade) return;
    quickTrade(selectedMint, side, sol, prices);
  };

  // ── Live trade handlers ────────────────────────────────────────────────────
  const liveBuy = useCallback(
    async (solAmount: number) => {
      if (!selectedMint) return;
      // ExactIn: spend exactly X SOL, get as much token as possible.
      await swap.fetchQuote(
        SOL_MINT,
        selectedMint,
        Math.floor(solAmount * 1e9),
        50,
        'ExactIn',
      );
    },
    [selectedMint, swap],
  );

  const liveSell = useCallback(
    async (solAmount: number) => {
      if (!selectedMint) return;
      // ExactOut: receive exactly X SOL, spend however many tokens needed.
      await swap.fetchQuote(
        selectedMint,
        SOL_MINT,
        Math.floor(solAmount * 1e9),
        50,
        'ExactOut',
      );
    },
    [selectedMint, swap],
  );

  // When status moves to 'confirmed', auto-close the dialog after 3 s.
  const handleConfirmDialogCancel = useCallback(() => {
    swap.reset();
  }, [swap]);

  // Determine input/output symbols for the confirm dialog.
  const dialogInputSymbol =
    swap.quote?.swapMode === 'ExactOut'
      ? (selectedMeta?.symbol ?? symbol)
      : 'SOL';
  const dialogOutputSymbol =
    swap.quote?.swapMode === 'ExactOut'
      ? 'SOL'
      : (selectedMeta?.symbol ?? symbol);
  const dialogInputDecimals =
    swap.quote?.swapMode === 'ExactOut'
      ? (selectedMeta?.decimals ?? 6)
      : 9; // SOL = 9 decimals
  const dialogOutputDecimals =
    swap.quote?.swapMode === 'ExactOut'
      ? 9
      : (selectedMeta?.decimals ?? 6);

  const showDialog =
    effectiveMode === 'live' &&
    swap.status !== 'idle' &&
    swap.status !== 'quoting';

  return (
    <>
      {/* Confirm dialog — fixed overlay, portal-like behaviour */}
      {showDialog && (
        <LiveTradeConfirmDialog
          state={swap}
          inputSymbol={dialogInputSymbol}
          outputSymbol={dialogOutputSymbol}
          inputDecimals={dialogInputDecimals}
          outputDecimals={dialogOutputDecimals}
          onConfirm={swap.execute}
          onCancel={handleConfirmDialogCancel}
        />
      )}

      <div className="flex flex-col h-full min-h-0 text-[11px]">
        {/* ── Metrics row ────────────────────────────────────────────────────── */}
        <div className="grid grid-cols-2 gap-2 p-3 border-b border-ds-border">
          <Metric label="Equity" value={formatPnL(equity, { signed: false })} />
          <Metric label="SOL" value={formatPrice(solBal, 3)} />
          <Metric
            label="Realized"
            value={formatPnL(portfolio?.realizedPnlUsd ?? 0)}
            colored={portfolio?.realizedPnlUsd}
          />
          <Metric label="Mode" value={cluster === 'devnet' ? 'Devnet' : 'Mainnet'} />
        </div>

        {/* ── Mode toggle ─────────────────────────────────────────────────────── */}
        <div className="px-3 py-2 border-b border-ds-border flex items-center justify-between">
          <span className="text-ds-text-secondary uppercase tracking-wider text-[10px]">
            {effectiveMode === 'live' ? (
              <span className="text-ds-green">⬤ Live</span>
            ) : (
              'Paper'
            )}{' '}
            · {symbol}
          </span>
          <div className="flex items-center gap-2">
            {liveAvailable && (
              <div className="flex rounded overflow-hidden border border-ds-border text-[9px]">
                <ModeTab
                  label="Paper"
                  active={effectiveMode === 'paper'}
                  onClick={() => {
                    setMode('paper');
                    swap.reset();
                  }}
                />
                <ModeTab
                  label="Live"
                  active={effectiveMode === 'live'}
                  onClick={() => setMode('live')}
                  danger
                />
              </div>
            )}
            {effectiveMode === 'paper' && (
              <label className="flex items-center gap-1.5 cursor-pointer text-ds-text-secondary">
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(e) => setEnabled(e.target.checked)}
                  className="accent-ds-blue w-3 h-3"
                />
                <span>On</span>
              </label>
            )}
          </div>
        </div>

        {/* Paper error */}
        {effectiveMode === 'paper' && error && (
          <p className="px-3 py-1 text-ds-red text-[10px]">{error}</p>
        )}

        {/* Live swap quoting spinner */}
        {effectiveMode === 'live' && swap.status === 'quoting' && (
          <div className="px-3 py-2 flex items-center gap-2 text-[10px] text-ds-text-muted">
            <SpinnerInline />
            Fetching best route…
          </div>
        )}

        {/* Live swap error inline (when no dialog is showing) */}
        {effectiveMode === 'live' && swap.status === 'error' && !showDialog && (
          <div className="px-3 py-1.5 text-ds-red text-[10px] break-all">
            {swap.error}
            <button
              type="button"
              onClick={swap.reset}
              className="ml-2 underline text-ds-text-muted hover:text-ds-text-secondary"
            >
              dismiss
            </button>
          </div>
        )}

        {/* Live mode mainnet warning */}
        {effectiveMode === 'live' && (
          <div className="px-3 py-1.5 text-[10px] text-ds-yellow bg-ds-yellow/5 border-b border-ds-border">
            ⚠ Live mode — trades use real SOL from your wallet
          </div>
        )}

        {/* ── Trade buttons ───────────────────────────────────────────────────── */}
        <div className="p-3 space-y-2 flex-1 min-h-0 overflow-y-auto terminal-scroll">
          {/* Buy */}
          <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mb-1">Buy</div>
          <div className="grid grid-cols-2 gap-1.5">
            {SIZES.map((amt) =>
              effectiveMode === 'live' ? (
                <button
                  key={`lb-${amt}`}
                  type="button"
                  disabled={!selectedMint || swap.status === 'quoting'}
                  onClick={() => liveBuy(amt)}
                  className="terminal-btn terminal-btn-buy lg:!py-1.5 !py-2.5 text-[12px] lg:text-[11px] min-h-[44px] lg:min-h-0"
                >
                  {amt} SOL
                </button>
              ) : (
                <button
                  key={`pb-${amt}`}
                  type="button"
                  disabled={!canTrade}
                  onClick={() => paperTrade('buy', amt)}
                  className="terminal-btn terminal-btn-buy lg:!py-1.5 !py-2.5 text-[12px] lg:text-[11px] min-h-[44px] lg:min-h-0"
                >
                  {amt} SOL
                </button>
              ),
            )}
          </div>

          {/* Sell */}
          <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mb-1 pt-1">
            Sell → SOL
          </div>
          <div className="grid grid-cols-2 gap-1.5">
            {SIZES.slice(0, 2).map((amt) =>
              effectiveMode === 'live' ? (
                <button
                  key={`ls-${amt}`}
                  type="button"
                  disabled={!selectedMint || swap.status === 'quoting'}
                  onClick={() => liveSell(amt)}
                  className="terminal-btn terminal-btn-sell lg:!py-1.5 !py-2.5 text-[12px] lg:text-[11px] min-h-[44px] lg:min-h-0"
                >
                  {amt} SOL
                </button>
              ) : (
                <button
                  key={`ps-${amt}`}
                  type="button"
                  disabled={!canTrade}
                  onClick={() => paperTrade('sell', amt)}
                  className="terminal-btn terminal-btn-sell lg:!py-1.5 !py-2.5 text-[12px] lg:text-[11px] min-h-[44px] lg:min-h-0"
                >
                  {amt} SOL
                </button>
              ),
            )}
          </div>

          {/* Connect wallet nudge if on mainnet but not connected */}
          {!liveAvailable && cluster === 'mainnet-beta' && (
            <p className="text-[10px] text-ds-text-muted pt-1">
              Connect wallet to enable live trading
            </p>
          )}
        </div>

        {/* ── Footer ──────────────────────────────────────────────────────────── */}
        <div className="p-3 border-t border-ds-border space-y-2 shrink-0">
          {effectiveMode === 'paper' && (
            <label className="flex items-center gap-2 cursor-pointer text-ds-text-secondary text-[10px]">
              <input
                type="checkbox"
                checked={autoTrade}
                onChange={(e) => setAutoTrade(e.target.checked)}
                className="accent-ds-blue w-3 h-3"
              />
              Auto-trade on signals
            </label>
          )}
          {lastFill && effectiveMode === 'paper' && (
            <div className="font-mono text-[10px] text-ds-text-secondary truncate">
              Last: {lastFill.side} {tokenSymbol(lastFill.tokenOut)}{' '}
              <span className={lastFill.pnlUsd >= 0 ? 'text-ds-green' : 'text-ds-red'}>
                {formatPnL(lastFill.pnlUsd)}
              </span>
            </div>
          )}
          {effectiveMode === 'live' && swap.status === 'confirmed' && swap.signature && (
            <div className="text-[10px] text-ds-green font-mono truncate">
              ✓{' '}
              <a
                href={`https://solscan.io/tx/${swap.signature}`}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:underline"
              >
                {swap.signature.slice(0, 20)}…
              </a>
            </div>
          )}
          <button
            type="button"
            onClick={() => reset(cluster)}
            className="text-[10px] text-ds-text-muted hover:text-ds-red transition-colors"
          >
            Reset portfolio
          </button>
        </div>
      </div>
    </>
  );
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function Metric({
  label,
  value,
  colored,
}: {
  label: string;
  value: string;
  colored?: number;
}) {
  const color =
    colored != null
      ? colored > 0
        ? 'text-ds-green'
        : colored < 0
          ? 'text-ds-red'
          : 'text-ds-text-secondary'
      : 'text-ds-text-primary';
  return (
    <div>
      <div className="text-[9px] text-ds-text-muted uppercase tracking-wider">{label}</div>
      <div className={`text-[12px] font-mono tabular-nums ${color}`}>{value}</div>
    </div>
  );
}

function ModeTab({
  label,
  active,
  onClick,
  danger = false,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        'px-2 py-0.5 text-[9px] uppercase tracking-wider transition-colors',
        active
          ? danger
            ? 'bg-ds-red/20 text-ds-red'
            : 'bg-ds-blue/20 text-ds-blue'
          : 'text-ds-text-muted hover:text-ds-text-secondary',
      ].join(' ')}
    >
      {label}
    </button>
  );
}

function SpinnerInline() {
  return (
    <svg
      className="animate-spin w-3 h-3 text-ds-blue shrink-0"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
  );
}
