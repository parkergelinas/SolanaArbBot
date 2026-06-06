'use client';

import { useEffect, useState } from 'react';

import { timeAgo } from '@/lib/format/time';
import { useIntelConnected } from '@/lib/intelligence/hooks';
import { useFeedsStore } from '@/stores/feedsStore';
import { useStreamStore } from '@/stores/streamStore';

function FeedPill({
  label,
  status,
  lastOkAt,
  compact,
}: {
  label: string;
  status: 'online' | 'offline' | 'degraded';
  lastOkAt: number;
  compact?: boolean;
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

  if (compact) {
    return (
      <span
        className="inline-flex items-center justify-center w-2 h-2 rounded-full shrink-0"
        title={`${label}: ${status}${lastOkAt ? ` · ${timeAgo(lastOkAt)}` : ''}`}
      >
        <span className={`w-full h-full rounded-full ${dot}`} />
      </span>
    );
  }

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
  const [expanded, setExpanded] = useState(false);
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

  const feeds = [
    { label: 'Stream', status: streamFeed.status, lastOkAt: streamFeed.lastOkAt },
    { label: 'Jupiter', status: streamFeed.status, lastOkAt: lastMessageAt },
    { label: 'Hub', status: hubFeed.status, lastOkAt: hubFeed.lastOkAt },
    { label: 'DexScr', status: dexFeed.status, lastOkAt: dexFeed.lastOkAt },
    { label: 'Intel', status: intelFeed.status, lastOkAt: intelFeed.lastOkAt },
  ] as const;

  const onlineCount = feeds.filter((f) => f.status === 'online').length;

  return (
    <div className="shrink-0 border-b border-ds-border bg-ds-surface">
      <div className="lg:hidden flex items-center gap-2 px-3 h-8">
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          className="touch-target flex items-center gap-2 text-[10px] font-mono uppercase tracking-wider text-ds-text-secondary"
          aria-expanded={expanded}
        >
          <span>Feeds</span>
          <span className="text-ds-text-muted">{onlineCount}/{feeds.length}</span>
          <span className="text-ds-text-muted">{expanded ? '▴' : '▾'}</span>
        </button>
        <div className="flex items-center gap-1.5 flex-1 min-w-0">
          {feeds.map((f) => (
            <FeedPill key={f.label} {...f} compact />
          ))}
        </div>
      </div>

      {expanded && (
        <div className="lg:hidden flex flex-wrap items-center gap-1.5 px-3 pb-2 border-t border-ds-border/50">
          {feeds.map((f) => (
            <FeedPill key={`${f.label}-full`} {...f} />
          ))}
        </div>
      )}

      <div className="hidden lg:flex flex-wrap items-center gap-1.5 px-3 h-7">
        <span className="text-[9px] text-ds-text-muted uppercase tracking-widest mr-1">Feeds</span>
        {feeds.map((f) => (
          <FeedPill key={f.label} {...f} />
        ))}
      </div>
    </div>
  );
}
