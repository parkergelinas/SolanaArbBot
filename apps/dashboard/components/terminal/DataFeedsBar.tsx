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
  const dot =
    status === 'online'
      ? 'bg-ds-green'
      : status === 'degraded'
        ? 'bg-ds-amber'
        : 'bg-ds-text-muted';
  const text =
    status === 'online'
      ? 'text-ds-green border-ds-green/25 bg-ds-green-dim'
      : status === 'degraded'
        ? 'text-ds-amber border-ds-amber/25'
        : 'text-ds-text-muted border-ds-border';

  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-px border rounded-terminal text-[9px] font-mono uppercase tracking-wide ${text}`}
      title={lastOkAt ? `Last OK ${timeAgo(lastOkAt)}` : 'No data yet'}
    >
      <span className={`w-1 h-1 rounded-full ${dot}`} />
      {label}
    </span>
  );
}

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
    <div className="flex flex-wrap items-center gap-1.5 px-3 h-7 border-b border-ds-border bg-ds-surface shrink-0">
      <span className="text-[9px] text-ds-text-muted uppercase tracking-widest mr-1">Feeds</span>
      <FeedPill label="Stream" status={streamFeed.status} lastOkAt={streamFeed.lastOkAt} />
      <FeedPill label="Jupiter" status={streamFeed.status} lastOkAt={lastMessageAt} />
      <FeedPill label="Hub" status={hubFeed.status} lastOkAt={hubFeed.lastOkAt} />
      <FeedPill label="DexScr" status={dexFeed.status} lastOkAt={dexFeed.lastOkAt} />
      <FeedPill label="Intel" status={intelFeed.status} lastOkAt={intelFeed.lastOkAt} />
    </div>
  );
}
