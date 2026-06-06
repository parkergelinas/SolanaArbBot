'use client';

import { useEffect } from 'react';

import { timeAgo } from '@/lib/format/time';
import { useIntelConnected } from '@/lib/intelligence/hooks';
import { useFeedsStore } from '@/stores/feedsStore';
import { useStreamStore } from '@/stores/streamStore';

function FeedPill({
  label,
  status,
  lastOkAt,
}: {
  label: string;
  status: 'online' | 'offline' | 'degraded';
  lastOkAt: number;
}) {
  const color =
    status === 'online'
      ? 'text-flow-buy border-flow-buy/30 bg-flow-buy/5'
      : status === 'degraded'
        ? 'text-terminal-warn border-terminal-warn/30 bg-terminal-warn/5'
        : 'text-terminal-muted border-terminal-border bg-terminal-bg';

  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-[9px] mono uppercase tracking-wide ${color}`}
      title={lastOkAt ? `Last OK ${timeAgo(lastOkAt)}` : 'No data yet'}
    >
      <span
        className={`w-1 h-1 rounded-full ${
          status === 'online' ? 'bg-flow-buy' : status === 'degraded' ? 'bg-terminal-warn' : 'bg-slate-600'
        }`}
      />
      {label}
    </span>
  );
}

/** Compact feed health strip for the terminal. */
export default function DataFeedsBar() {
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const lastMessageAt = useStreamStore((s) => s.lastMessageAt);
  const streamFeed = useFeedsStore((s) => s.stream);
  const hubFeed = useFeedsStore((s) => s.signalHub);
  const dexFeed = useFeedsStore((s) => s.dexScreener);
  const intelFeed = useFeedsStore((s) => s.intelligence);
  const setStream = useFeedsStore((s) => s.setStream);
  const setIntelligence = useFeedsStore((s) => s.setIntelligence);
  const intelConnected = useIntelConnected();

  useEffect(() => {
    const status =
      connectionMode === 'live'
        ? 'online'
        : connectionMode === 'degraded'
          ? 'degraded'
          : 'offline';
    setStream({
      status,
      lastOkAt: lastMessageAt || streamFeed.lastOkAt,
      detail: connectionMode,
    });
  }, [connectionMode, lastMessageAt, setStream, streamFeed.lastOkAt]);

  useEffect(() => {
    setIntelligence({
      status: intelConnected ? 'online' : 'offline',
      lastOkAt: intelConnected ? Date.now() : intelFeed.lastOkAt,
    });
  }, [intelConnected, setIntelligence, intelFeed.lastOkAt]);

  return (
    <div className="flex flex-wrap items-center gap-1.5 px-3 py-1 border-b border-terminal-border bg-terminal-panel/60 text-[9px]">
      <span className="text-terminal-muted uppercase tracking-widest mr-1">Feeds</span>
      <FeedPill label="Stream" status={streamFeed.status} lastOkAt={streamFeed.lastOkAt} />
      <FeedPill label="Jupiter" status={streamFeed.status} lastOkAt={lastMessageAt} />
      <FeedPill label="Hub" status={hubFeed.status} lastOkAt={hubFeed.lastOkAt} />
      <FeedPill label="DexScr" status={dexFeed.status} lastOkAt={dexFeed.lastOkAt} />
      <FeedPill label="Intel" status={intelFeed.status} lastOkAt={intelFeed.lastOkAt} />
    </div>
  );
}
