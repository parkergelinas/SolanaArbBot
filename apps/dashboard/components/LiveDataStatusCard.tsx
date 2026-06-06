'use client';

import Link from 'next/link';

import { DsPanel } from '@/components/layout/PageShell';
import { useIntelConnected } from '@/lib/intelligence/hooks';
import { timeAgo } from '@/lib/format/time';
import { useFeedsStore } from '@/stores/feedsStore';
import { useStreamStore } from '@/stores/streamStore';
import { useWsContext } from './WebSocketProvider';

function Row({ label, ok, sub }: { label: string; ok: boolean; sub?: string }) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-ds-border/50 last:border-0">
      <span className="text-[11px] text-ds-text-secondary">{label}</span>
      <div className="text-right">
        <span className={`text-[10px] font-medium uppercase tracking-wider ${ok ? 'text-ds-green' : 'text-ds-text-muted'}`}>
          {ok ? 'Connected' : 'Offline'}
        </span>
        {sub && <div className="text-[9px] font-mono text-ds-text-muted mt-0.5">{sub}</div>}
      </div>
    </div>
  );
}

/** Shared live-data status for Overview / Bot pages. */
export default function LiveDataStatusCard({ compact }: { compact?: boolean }) {
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
    <DsPanel
      compact
      title="Live data stack"
      action={
        <Link
          href="/terminal"
          className="text-[10px] px-2 py-0.5 rounded-terminal border border-ds-border text-ds-blue hover:bg-ds-blue/10 transition-colors"
        >
          Terminal →
        </Link>
      }
      className={compact ? '' : undefined}
    >
      {!compact && (
        <p className="text-[10px] text-ds-text-muted mb-2 -mt-1">
          {liveStack ? 'Market stream active' : 'Start stream-api for live prices'}
        </p>
      )}
      <Row label="Control API" ok={controlWs} sub="Bot status, portfolio, config" />
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
    </DsPanel>
  );
}
