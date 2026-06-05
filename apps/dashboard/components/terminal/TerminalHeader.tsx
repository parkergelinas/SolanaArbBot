'use client';

import { useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';
import { useMarketStore, shortMint } from '@/stores/marketStore';

export default function TerminalHeader() {
  const connected = useStreamStore((s) => s.connected);
  const messagesApplied = useStreamStore((s) => s.messagesApplied);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const price = useMarketStore((s) => (selectedMint ? s.tokens[selectedMint] : undefined));

  return (
    <header className="flex items-center justify-between px-3 py-2 border-b border-terminal-border bg-terminal-panel">
      <div className="flex items-center gap-3">
        <div>
          <div className="text-xs font-bold tracking-[0.2em] text-terminal-live">
            SOLARB TERMINAL
          </div>
          <div className="text-[9px] text-terminal-muted uppercase tracking-wider">
            Institutional · Stream v1
          </div>
        </div>
        <div
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded border text-[10px] mono ${
            connected
              ? 'border-terminal-live/40 text-terminal-live bg-terminal-live/5'
              : 'border-flow-sell/40 text-flow-sell'
          }`}
        >
          <span className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-terminal-live live-pulse' : 'bg-flow-sell'}`} />
          {connected ? 'STREAM LIVE' : 'OFFLINE'}
        </div>
      </div>

      {selectedMint && (
        <div className="text-center hidden sm:block">
          <div className="text-[10px] text-terminal-muted mono">{shortMint(selectedMint, 6, 4)}</div>
          <div className="text-xl font-mono text-terminal-live tabular-nums">
            ${price ? price.price_usd.toFixed(4) : '—'}
          </div>
          {price && (
            <div
              className={`text-[10px] mono ${
                price.changePct >= 0 ? 'text-flow-buy' : 'text-flow-sell'
              }`}
            >
              {price.changePct >= 0 ? '+' : ''}
              {price.changePct.toFixed(2)}%
            </div>
          )}
        </div>
      )}

      <div className="text-right text-[10px] mono text-terminal-muted">
        <div>{messagesApplied.toLocaleString()} events</div>
        <div className="text-terminal-live/70">ws://8080/stream</div>
      </div>
    </header>
  );
}
