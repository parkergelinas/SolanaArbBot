'use client';

import Link from 'next/link';

import { useIntelConnected } from '@/lib/intelligence/hooks';
import { timeAgo } from '@/lib/format/time';
import { useFeedsStore } from '@/stores/feedsStore';
import { useStreamStore } from '@/stores/streamStore';
import { useWsContext } from './WebSocketProvider';

function Row({ label, ok, sub }: { label: string; ok: boolean; sub?: string }) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-platform-border/50 last:border-0">
      <span className="text-sm text-slate-300">{label}</span>
      <div className="text-right">
        <span className={`text-xs font-medium ${ok ? 'text-green-400' : 'text-slate-500'}`}>
          {ok ? 'Connected' : 'Offline'}
        </span>
        {sub && <div className="text-[10px] text-platform-muted">{sub}</div>}
      </div>
    </div>
  );
}

/** Shared live-data status for Overview / Bot pages. */
export default function LiveDataStatusCard() {
  const { connected: controlWs } = useWsContext();
  const streamConnected = useStreamStore((s) => s.connected);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const lastMessageAt = useStreamStore((s) => s.lastMessageAt);
  const hub = useFeedsStore((s) => s.signalHub);
  const dex = useFeedsStore((s) => s.dexScreener);
  const intelConnected = useIntelConnected();

  const liveStack =
    streamConnected && (connectionMode === 'live' || connectionMode === 'degraded');

  return (
    <div className="surface-card p-4">
      <div className="flex items-center justify-between mb-3">
        <div>
          <h2 className="text-sm font-medium text-slate-200">Live data stack</h2>
          <p className="text-[11px] text-platform-muted mt-0.5">
            {liveStack ? 'Market stream active' : 'Start stream-api for live prices'}
          </p>
        </div>
        <Link
          href="/terminal"
          className="text-xs px-2.5 py-1 rounded-lg border border-platform-border text-platform-accent hover:bg-platform-accent/10"
        >
          Open terminal →
        </Link>
      </div>
      <Row
        label="Control API"
        ok={controlWs}
        sub="Bot status, portfolio, config"
      />
      <Row
        label="Stream API (WS)"
        ok={streamConnected}
        sub={lastMessageAt ? `Last tick ${timeAgo(lastMessageAt)} · ${connectionMode.toUpperCase()}` : 'ws://8080/stream'}
      />
      <Row
        label="Signal hub"
        ok={hub.status === 'online'}
        sub={hub.lastOkAt ? timeAgo(hub.lastOkAt) : 'control-api /api/live-signals'}
      />
      <Row
        label="DexScreener"
        ok={dex.status === 'online'}
        sub={dex.lastOkAt ? `Watchlist ${timeAgo(dex.lastOkAt)}` : 'Charts + 24h stats'}
      />
      <Row label="Intelligence WS" ok={intelConnected} sub="Whale / smart money" />
    </div>
  );
}
