'use client';

import { useCallback, useMemo, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';

import { formatPrice } from '@/lib/formatters';
import { useLiveSwap } from '@/lib/hooks/useLiveSwap';
import { computeFrontendTradeSize } from '@/lib/solana/tradeSizer';
import { SOL_MINT, tokenSymbol, tokenMeta } from '@/lib/terminal/tokens';
import { useNetworkStore } from '@/stores/networkStore';
import { useMarketStore } from '@/stores/marketStore';
import LiveTradeConfirmDialog from '@/components/trading/LiveTradeConfirmDialog';

// ── Filter/sort types ──────────────────────────────────────────────────────────
type SortKey = 'spread' | 'profit' | 'age';
const MIN_SPREAD_OPTIONS = [0, 10, 20, 30, 50] as const;
const MIN_PROFIT_OPTIONS = [0, 0.1, 0.25, 0.5, 1] as const;

function sourceLabel(source?: string): string {
  if (source === 'dexscreener') return 'DS';
  if (source === 'jupiter') return 'JUP';
  return 'WS';
}

function spreadQuality(bps: number): string {
  if (bps >= 50) return 'text-ds-green';
  if (bps >= 30) return 'text-ds-amber';
  return 'text-ds-text-secondary';
}

export default function ArbitragePanel() {
  const opportunities = useMarketStore((s) => s.arbOpportunities);
  const solPriceUsd = useMarketStore((s) => s.prices[SOL_MINT]?.price_usd ?? 180);
  const cluster = useNetworkStore((s) => s.cluster);
  const { connected } = useWallet();
  const liveAvailable = cluster === 'mainnet-beta' && connected;

  // ── Filter + sort state ────────────────────────────────────────────────────
  const [minSpreadBps, setMinSpreadBps] = useState<number>(0);
  const [minProfitUsd, setMinProfitUsd] = useState<number>(0);
  const [sortKey, setSortKey] = useState<SortKey>('spread');

  const filtered = useMemo(() => {
    let list = opportunities.filter((a) => {
      if (a.spreadBps < minSpreadBps) return false;
      if (minProfitUsd > 0 && (a.estimatedProfitUsd ?? 0) < minProfitUsd) return false;
      return true;
    });
    if (sortKey === 'spread') list = [...list].sort((a, b) => b.spreadBps - a.spreadBps);
    if (sortKey === 'profit') list = [...list].sort((a, b) => (b.estimatedProfitUsd ?? 0) - (a.estimatedProfitUsd ?? 0));
    // 'age' = default insertion order (newest first from store)
    return list.slice(0, 30);
  }, [opportunities, minSpreadBps, minProfitUsd, sortKey]);

  // ── Live execute state ─────────────────────────────────────────────────────
  const [activeArbId, setActiveArbId] = useState<string | null>(null);
  const [activeArbToken, setActiveArbToken] = useState<string | null>(null);
  const [activeSizeSol, setActiveSizeSol] = useState<number>(0.25);
  const swap = useLiveSwap();

  const handleExecute = useCallback(
    async (arbId: string, tokenMint: string, spreadBps: number, liquidityUsd?: number) => {
      if (!liveAvailable) return;
      const sizing = computeFrontendTradeSize(spreadBps, solPriceUsd, { liquidityUsd });
      setActiveArbId(arbId);
      setActiveArbToken(tokenMint);
      setActiveSizeSol(sizing.solAmount);
      await swap.fetchQuote(SOL_MINT, tokenMint, Math.floor(sizing.solAmount * 1e9), 50, 'ExactIn');
    },
    [liveAvailable, solPriceUsd, swap],
  );

  const handleDialogCancel = useCallback(() => {
    swap.reset();
    setActiveArbId(null);
    setActiveArbToken(null);
  }, [swap]);

  const activeToken = activeArbToken ? tokenMeta(activeArbToken) : undefined;
  const activeSymbol = activeArbToken ? (activeToken?.symbol ?? tokenSymbol(activeArbToken)) : '?';
  const showDialog = liveAvailable && activeArbId !== null && swap.status !== 'idle' && swap.status !== 'quoting';

  return (
    <>
      {showDialog && (
        <LiveTradeConfirmDialog
          state={swap}
          inputSymbol="SOL"
          outputSymbol={activeSymbol}
          inputDecimals={9}
          outputDecimals={activeToken?.decimals ?? 6}
          onConfirm={swap.execute}
          onCancel={handleDialogCancel}
        />
      )}

      <section className="flex flex-col h-full min-h-0 bg-ds-surface overflow-hidden">
        {/* ── Header ────────────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between h-8 px-3 border-b border-ds-border shrink-0">
          <span className="text-[11px] font-medium uppercase tracking-[0.1em] text-ds-text-secondary">
            Cross-DEX Arb
          </span>
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-mono text-ds-text-muted">
              {filtered.length}/{opportunities.length}
            </span>
            {liveAvailable && (
              <span className="text-[9px] text-ds-green border border-ds-green/30 px-1 rounded">LIVE</span>
            )}
          </div>
        </div>

        {/* ── Filter bar ────────────────────────────────────────────────────── */}
        <div className="terminal-filterbar shrink-0">
          {/* Min spread */}
          <span className="text-ds-text-muted shrink-0">≥bps</span>
          {MIN_SPREAD_OPTIONS.map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => setMinSpreadBps(v)}
              className={`filter-pill ${minSpreadBps === v ? 'active' : ''}`}
            >
              {v === 0 ? 'any' : `${v}`}
            </button>
          ))}

          <span className="w-px h-3.5 bg-ds-border shrink-0 mx-0.5" />

          {/* Min profit */}
          <span className="text-ds-text-muted shrink-0">P≥</span>
          {MIN_PROFIT_OPTIONS.map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => setMinProfitUsd(v)}
              className={`filter-pill ${minProfitUsd === v ? 'active-green' : ''}`}
            >
              {v === 0 ? 'any' : `$${v}`}
            </button>
          ))}

          <span className="w-px h-3.5 bg-ds-border shrink-0 mx-0.5" />

          {/* Sort */}
          <span className="text-ds-text-muted shrink-0">sort</span>
          {(['spread', 'profit', 'age'] as SortKey[]).map((k) => (
            <button
              key={k}
              type="button"
              onClick={() => setSortKey(k)}
              className={`filter-pill ${sortKey === k ? 'active' : ''}`}
            >
              {k}
            </button>
          ))}
        </div>

        {/* ── Connect nudge ─────────────────────────────────────────────────── */}
        {!liveAvailable && (
          <div className="px-3 py-1 text-[10px] text-ds-text-muted border-b border-ds-border bg-ds-elevated/40 shrink-0">
            {!connected ? 'Connect wallet on mainnet to execute' : 'Switch to mainnet to execute'}
          </div>
        )}

        {/* ── Opportunity list ──────────────────────────────────────────────── */}
        <ul className="flex-1 overflow-y-auto terminal-scroll min-h-0">
          {filtered.length === 0 ? (
            <li className="text-[11px] text-ds-text-muted text-center py-10 px-3 leading-relaxed">
              {opportunities.length === 0
                ? <>Scanning Raydium · Orca · Jupiter · DexScreener…<br /><span className="text-[10px]">Requires stream-api swaps or DexScreener pairs</span></>
                : `No opportunities ≥ ${minSpreadBps} bps${minProfitUsd > 0 ? ` / ≥ $${minProfitUsd}` : ''}`}
            </li>
          ) : (
            filtered.map((arb) => {
              const sizing = computeFrontendTradeSize(arb.spreadBps, solPriceUsd, { liquidityUsd: arb.minLiquidityUsd });
              const isExecuting = activeArbId === arb.id && (swap.status === 'quoting' || swap.status === 'signing' || swap.status === 'sending' || swap.status === 'confirming');
              const isConfirmed = activeArbId === arb.id && swap.status === 'confirmed';

              return (
                <li
                  key={arb.id}
                  className="px-3 py-2 border-b border-ds-border hover:bg-ds-elevated/50 transition-colors"
                >
                  {/* Row 1: symbol + spread + exec */}
                  <div className="flex justify-between items-center gap-2">
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="font-mono text-[11px] text-ds-text-primary truncate">
                        {arb.symbol ?? tokenSymbol(arb.token)}
                      </span>
                      <span className="text-[8px] px-1 py-0.5 rounded border border-ds-border text-ds-text-muted font-mono shrink-0">
                        {sourceLabel(arb.source)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <span className={`font-mono text-[11px] font-semibold ${spreadQuality(arb.spreadBps)}`}>
                        +{arb.spreadBps.toFixed(1)} bps
                      </span>
                      {liveAvailable && (
                        <button
                          type="button"
                          disabled={isExecuting || swap.status === 'quoting'}
                          onClick={() => handleExecute(arb.id, arb.token, arb.spreadBps, arb.minLiquidityUsd)}
                          title={sizing.label}
                          className={[
                            'text-[9px] px-1.5 py-0.5 rounded border font-mono transition-colors',
                            isConfirmed
                              ? 'border-ds-green/40 text-ds-green bg-ds-green/10 cursor-default'
                              : isExecuting
                                ? 'border-ds-border text-ds-text-muted cursor-wait'
                                : 'border-ds-blue/40 text-ds-blue hover:bg-ds-blue/10 cursor-pointer',
                          ].join(' ')}
                        >
                          {isConfirmed ? '✓' : isExecuting ? '…' : `${sizing.solAmount}◎`}
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Row 2: venues */}
                  <div className="mt-1 text-[10px] text-ds-text-muted flex justify-between font-mono gap-2">
                    <span className="truncate">
                      Buy <span className="text-ds-text-secondary">{arb.buyDexLabel ?? arb.buyDex}</span>{' '}
                      {formatPrice(arb.buyPrice)}
                    </span>
                    <span className="truncate text-right">
                      Sell <span className="text-ds-text-secondary">{arb.sellDexLabel ?? arb.sellDex}</span>{' '}
                      {formatPrice(arb.sellPrice)}
                    </span>
                  </div>

                  {/* Row 3: profit + MEV size */}
                  <div className="mt-1 flex justify-between text-[9px] font-mono text-ds-text-muted">
                    <span>
                      Est{' '}
                      <span className={arb.estimatedProfitUsd && arb.estimatedProfitUsd > 0 ? 'text-ds-green' : ''}>
                        ${(arb.estimatedProfitUsd ?? 0).toFixed(2)}
                      </span>
                      {arb.venueCount != null && <span> · {arb.venueCount}v</span>}
                    </span>
                    <span className="flex items-center gap-1.5">
                      {arb.winProbability != null && (
                        <span className="text-ds-blue">{(arb.winProbability * 100).toFixed(0)}% fill</span>
                      )}
                      {liveAvailable && (
                        <span
                          className={sizing.binding === 'min_clamp' ? 'text-ds-red' : sizing.binding === 'mev_threshold' ? 'text-ds-amber' : 'text-ds-text-muted'}
                          title={sizing.label}
                        >
                          safe {sizing.mevSafeSol.toFixed(2)}◎
                        </span>
                      )}
                    </span>
                  </div>

                  {/* Confirmed tx */}
                  {isConfirmed && swap.signature && (
                    <div className="mt-1 text-[9px] text-ds-green font-mono truncate">
                      ✓{' '}
                      <a href={`https://solscan.io/tx/${swap.signature}`} target="_blank" rel="noopener noreferrer" className="hover:underline">
                        {swap.signature.slice(0, 28)}…
                      </a>{' '}
                      ({activeSizeSol}◎)
                    </div>
                  )}
                </li>
              );
            })
          )}
        </ul>
      </section>
    </>
  );
}
