'use client';

import { memo } from 'react';

import { formatPnL, formatPrice, formatSize, formatTime } from '@/lib/formatters';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { swapDirection } from '@/lib/terminal/swaps';
import { amountToHuman } from '@/stores/marketStore';
import type { TradeLogEntry } from './TradeLog';

interface TradeLogRowProps {
  entry: TradeLogEntry;
  onPause: () => void;
}

function strategyLabel(entry: TradeLogEntry): string {
  if (entry.kind === 'fill' && entry.fill) {
    if (entry.fill.source === 'signal') return 'SNIPER';
    if (entry.fill.source === 'stream') return 'ARB';
    return 'PAPER';
  }
  if (entry.kind === 'signal' && entry.signal) {
    return entry.signal.kind.toUpperCase().slice(0, 6);
  }
  return 'ARB';
}

function actionLabel(entry: TradeLogEntry): { text: string; className: string } {
  if (entry.kind === 'fill' && entry.fill) {
    const buy = entry.fill.side === 'buy';
    return {
      text: buy ? 'BUY' : 'SELL',
      className: buy ? 'text-ds-blue' : 'text-ds-text-secondary',
    };
  }
  if (entry.kind === 'swap' && entry.swap) {
    const dir = swapDirection(entry.swap);
    return {
      text: dir === 'buy' ? 'BUY' : dir === 'sell' ? 'SELL' : 'SWAP',
      className: dir === 'buy' ? 'text-ds-blue' : 'text-ds-text-secondary',
    };
  }
  if (entry.kind === 'signal') {
    return { text: 'SIG', className: 'text-ds-text-secondary' };
  }
  return { text: '—', className: 'text-ds-text-muted' };
}

function pairLabel(entry: TradeLogEntry): string {
  if (entry.kind === 'fill' && entry.fill) {
    return `${tokenSymbol(entry.fill.tokenIn)}/${tokenSymbol(entry.fill.tokenOut)}`;
  }
  if (entry.kind === 'swap' && entry.swap) {
    return `${tokenSymbol(entry.swap.token_in)}/${tokenSymbol(entry.swap.token_out)}`;
  }
  if (entry.kind === 'signal' && entry.signal) {
    return tokenSymbol(entry.signal.mint);
  }
  return '—';
}

function amountLabel(entry: TradeLogEntry): string {
  if (entry.kind === 'fill' && entry.fill) {
    return formatSize(entry.fill.amountIn);
  }
  if (entry.kind === 'swap' && entry.swap) {
    return formatSize(amountToHuman(entry.swap.token_in, entry.swap.amount_in));
  }
  return '—';
}

function priceLabel(entry: TradeLogEntry): string {
  if (entry.kind === 'fill' && entry.fill) {
    return formatPrice(entry.fill.priceUsd);
  }
  if (entry.kind === 'swap' && entry.swap) {
    const out = amountToHuman(entry.swap.token_out, entry.swap.amount_out);
    if (out <= 0) return '—';
    const inp = amountToHuman(entry.swap.token_in, entry.swap.amount_in);
    return formatPrice(inp / out);
  }
  return '—';
}

function pnlLabel(entry: TradeLogEntry): { text: string; className: string } {
  if (entry.status === 'pending') {
    return { text: 'pending', className: 'text-ds-amber' };
  }
  if (entry.status === 'reverted') {
    return { text: 'reverted', className: 'text-ds-text-muted line-through' };
  }
  if (entry.kind === 'fill' && entry.fill) {
    const v = entry.fill.pnlUsd;
    return {
      text: formatPnL(v),
      className: v > 0 ? 'text-ds-green' : v < 0 ? 'text-ds-red' : 'text-ds-text-secondary',
    };
  }
  return { text: '—', className: 'text-ds-text-muted' };
}

const TradeLogRow = memo(function TradeLogRow({ entry, onPause }: TradeLogRowProps) {
  const action = actionLabel(entry);
  const pnl = pnlLabel(entry);
  const strategy = strategyLabel(entry);

  return (
    <button
      type="button"
      onClick={onPause}
      className="w-full grid grid-cols-[6.5rem_4rem_3rem_5rem_3.5rem_4rem_4.5rem] gap-1 px-2 text-[11px] font-mono tabular-nums text-left whitespace-nowrap overflow-hidden border-b border-ds-border hover:bg-ds-elevated"
      style={{ height: 22, lineHeight: '22px' }}
    >
      <span className="text-ds-text-muted truncate">{formatTime(entry.timestampMs)}</span>
      <span className="text-ds-text-secondary truncate">{strategy}</span>
      <span className={action.className}>{action.text}</span>
      <span className="text-ds-text-primary truncate">{pairLabel(entry)}</span>
      <span className="text-ds-text-primary text-right">{amountLabel(entry)}</span>
      <span className="text-ds-text-primary text-right">{priceLabel(entry)}</span>
      <span className={`text-right ${pnl.className}`}>{pnl.text}</span>
    </button>
  );
});

export default TradeLogRow;
