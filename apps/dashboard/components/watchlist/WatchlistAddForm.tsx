'use client';

import { useState } from 'react';

import { resolvePairForMint } from '@/lib/dexscreener/client';
import { usePriceAnchorStore } from '@/stores/priceAnchorStore';
import { useWatchlistStore } from '@/stores/watchlistStore';

export default function WatchlistAddForm() {
  const addOpen = useWatchlistStore((s) => s.addOpen);
  const setAddOpen = useWatchlistStore((s) => s.setAddOpen);
  const addToken = useWatchlistStore((s) => s.addToken);
  const refreshMint = usePriceAnchorStore((s) => s.refreshMint);

  const [input, setInput] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  if (!addOpen) return null;

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const mint = input.trim();
    if (mint.length < 32) {
      setError('Enter a valid Solana mint address');
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const snap = await resolvePairForMint(mint);
      if (!snap) {
        setError('No DexScreener pair found for this mint');
        setLoading(false);
        return;
      }
      const ok = addToken({
        mint,
        symbol: snap.baseSymbol,
        name: `${snap.baseSymbol}/${snap.quoteSymbol}`,
        decimals: 6,
      });
      if (!ok) {
        setError('Token already on watchlist');
      } else {
        await refreshMint(mint);
        setInput('');
        setAddOpen(false);
      }
    } catch {
      setError('Lookup failed — check mint address');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={onSubmit}
      className="p-2 border-b border-ds-border bg-ds-elevated shrink-0 space-y-1.5"
    >
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Paste mint address…"
        className="w-full h-7 px-2 text-[11px] font-mono bg-ds-base border border-ds-border rounded-terminal text-ds-text-primary placeholder:text-ds-text-muted focus:outline-none focus:border-ds-blue"
        autoFocus
      />
      {error && <p className="text-[10px] text-ds-red">{error}</p>}
      <div className="flex gap-1.5">
        <button
          type="submit"
          disabled={loading}
          className="flex-1 h-6 text-[10px] uppercase tracking-wider bg-ds-blue text-white rounded-terminal disabled:opacity-40"
        >
          {loading ? '…' : 'Add'}
        </button>
        <button
          type="button"
          onClick={() => {
            setAddOpen(false);
            setError(null);
            setInput('');
          }}
          className="px-2 h-6 text-[10px] text-ds-text-muted hover:text-ds-text-primary"
        >
          Cancel
        </button>
      </div>
    </form>
  );
}
