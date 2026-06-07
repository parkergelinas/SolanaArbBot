'use client';

/**
 * LiveTradeConfirmDialog
 *
 * Full-screen overlay that shows a Jupiter quote preview and lets the user
 * confirm or cancel before signing a live on-chain transaction.
 *
 * Rendered by both OrderTicket (manual swaps) and ArbitragePanel (arb execution).
 */

import { useEffect } from 'react';
import type { LiveSwapState } from '@/lib/hooks/useLiveSwap';
import { atomicToUi, parsePriceImpact, quoteRouteLabel } from '@/lib/solana/jupiter';

export interface LiveTradeConfirmDialogProps {
  state: LiveSwapState;
  inputSymbol: string;
  outputSymbol: string;
  inputDecimals: number;
  outputDecimals: number;
  onConfirm: () => void;
  onCancel: () => void;
}

const STATUS_LABEL: Record<string, string> = {
  ready: 'Review your trade',
  signing: 'Waiting for wallet…',
  sending: 'Broadcasting transaction…',
  confirming: 'Confirming on-chain…',
  confirmed: 'Trade confirmed ✓',
  error: 'Trade failed',
};

export default function LiveTradeConfirmDialog({
  state,
  inputSymbol,
  outputSymbol,
  inputDecimals,
  outputDecimals,
  onConfirm,
  onCancel,
}: LiveTradeConfirmDialogProps) {
  const { status, quote, signature, error, priorityFeeLamports } = state;
  const isProcessing = status === 'signing' || status === 'sending' || status === 'confirming';
  const isDone = status === 'confirmed' || status === 'error';

  // Close on Escape key (only when not processing)
  useEffect(() => {
    if (isProcessing) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isProcessing, onCancel]);

  if (status === 'idle' || status === 'quoting') return null;

  const inAmountUi = quote ? atomicToUi(quote.inAmount, inputDecimals) : 0;
  const outAmountUi = quote ? atomicToUi(quote.outAmount, outputDecimals) : 0;
  const priceImpact = quote ? parsePriceImpact(quote.priceImpactPct) : 0;
  const routeLabel = quote ? quoteRouteLabel(quote) : '';
  const slippagePct = quote ? (quote.slippageBps / 100).toFixed(1) : '0.5';

  // Convert total priority fee estimate to SOL for display.
  const feeEstimateSol = (priorityFeeLamports / 1e9).toFixed(6);

  const explorerUrl = signature
    ? `https://solscan.io/tx/${signature}`
    : null;

  return (
    // Backdrop
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={isProcessing ? undefined : onCancel}
    >
      {/* Modal card */}
      <div
        className="relative bg-ds-surface border border-ds-border rounded-lg shadow-2xl w-full max-w-sm mx-4 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-ds-border">
          <span className="text-[12px] font-semibold uppercase tracking-wider text-ds-text-secondary">
            {STATUS_LABEL[status] ?? 'Live Trade'}
          </span>
          {!isProcessing && (
            <button
              type="button"
              onClick={onCancel}
              className="text-ds-text-muted hover:text-ds-text-secondary transition-colors text-lg leading-none"
              aria-label="Close"
            >
              ×
            </button>
          )}
        </div>

        {/* Quote details */}
        {quote && (
          <div className="px-4 py-3 space-y-2.5">
            {/* Swap summary */}
            <div className="flex items-center justify-between">
              <div className="text-center">
                <div className="text-[20px] font-mono font-semibold text-ds-text-primary tabular-nums">
                  {inAmountUi.toLocaleString(undefined, { maximumFractionDigits: 6 })}
                </div>
                <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mt-0.5">
                  {inputSymbol}
                </div>
              </div>
              <div className="text-ds-text-muted text-[18px] px-3">→</div>
              <div className="text-center">
                <div className="text-[20px] font-mono font-semibold text-ds-green tabular-nums">
                  {outAmountUi.toLocaleString(undefined, { maximumFractionDigits: 6 })}
                </div>
                <div className="text-[10px] text-ds-text-muted uppercase tracking-wider mt-0.5">
                  {outputSymbol}
                </div>
              </div>
            </div>

            {/* Fee / route details */}
            <div className="bg-ds-elevated rounded-md px-3 py-2.5 space-y-1.5 text-[11px] font-mono">
              <Row
                label="Route"
                value={routeLabel}
                valueClass="text-ds-blue"
              />
              <Row
                label="Price impact"
                value={`${priceImpact.toFixed(3)}%`}
                valueClass={priceImpact > 1 ? 'text-ds-red' : priceImpact > 0.3 ? 'text-ds-yellow' : 'text-ds-text-secondary'}
              />
              <Row label="Slippage" value={`${slippagePct}%`} />
              <Row
                label="Priority fee"
                value={`~${feeEstimateSol} SOL`}
                valueClass="text-ds-text-muted"
              />
            </div>

            {/* High price impact warning */}
            {priceImpact > 1 && (
              <div className="text-[10px] text-ds-red bg-ds-red/10 rounded px-2 py-1.5 border border-ds-red/20">
                ⚠ Price impact &gt; 1% — trade size may be too large for this pool's liquidity.
              </div>
            )}

            {/* Mainnet risk notice */}
            <div className="text-[10px] text-ds-text-muted bg-ds-elevated/60 rounded px-2 py-1.5">
              This executes a real transaction on Solana mainnet using your connected wallet.
              Funds cannot be recovered once signed.
            </div>
          </div>
        )}

        {/* Processing spinner / confirmed state / error */}
        {isProcessing && (
          <div className="px-4 pb-4">
            <div className="flex items-center gap-2 text-[11px] text-ds-text-secondary">
              <Spinner />
              <span>{STATUS_LABEL[status]}</span>
            </div>
          </div>
        )}

        {status === 'confirmed' && signature && (
          <div className="px-4 pb-4 space-y-2">
            <div className="text-[11px] text-ds-green font-mono break-all">{signature}</div>
            {explorerUrl && (
              <a
                href={explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-[10px] text-ds-blue hover:underline"
              >
                View on Solscan →
              </a>
            )}
          </div>
        )}

        {status === 'error' && error && (
          <div className="px-4 pb-3 text-[11px] text-ds-red font-mono break-all">{error}</div>
        )}

        {/* Action buttons */}
        <div className="px-4 pb-4 flex gap-2">
          {status === 'ready' && (
            <>
              <button
                type="button"
                onClick={onCancel}
                className="flex-1 terminal-btn text-[12px] !py-2.5"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={onConfirm}
                className="flex-1 terminal-btn terminal-btn-buy text-[12px] !py-2.5 font-semibold"
              >
                Confirm Swap
              </button>
            </>
          )}

          {isDone && (
            <button
              type="button"
              onClick={onCancel}
              className="flex-1 terminal-btn text-[12px] !py-2.5"
            >
              {status === 'confirmed' ? 'Done' : 'Dismiss'}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function Row({
  label,
  value,
  valueClass = 'text-ds-text-secondary',
}: {
  label: string;
  value: string;
  valueClass?: string;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-ds-text-muted">{label}</span>
      <span className={valueClass}>{value}</span>
    </div>
  );
}

function Spinner() {
  return (
    <svg
      className="animate-spin w-3.5 h-3.5 text-ds-blue shrink-0"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
      />
    </svg>
  );
}
