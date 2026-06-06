'use client';

import { useWsContext } from './WebSocketProvider';
import { useStreamLatest } from '@/lib/hooks';
import type { SystemStatus } from '@/lib/types';

export default function SystemStatusBadge() {
  const { connected } = useWsContext();
  const status = useStreamLatest<SystemStatus>('status');

  const running = status?.running ?? false;
  const mode = status?.mode ?? 'paper';

  return (
    <div className="flex items-center gap-2 flex-wrap">
      <span
        className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-terminal border text-[10px] font-mono uppercase ${
          connected
            ? 'border-ds-green/30 text-ds-green bg-ds-green/5'
            : 'border-ds-red/30 text-ds-red bg-ds-red/5'
        }`}
      >
        <span className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-ds-green live-pulse' : 'bg-ds-red'}`} />
        {connected ? 'WS live' : 'WS off'}
      </span>

      <span
        className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-terminal border text-[10px] font-mono uppercase ${
          running
            ? 'border-ds-green/30 text-ds-green bg-ds-green/5'
            : 'border-ds-border text-ds-text-muted bg-ds-elevated/30'
        }`}
      >
        <span className={`w-1.5 h-1.5 rounded-full ${running ? 'bg-ds-green' : 'bg-ds-text-muted'}`} />
        {running ? `${mode}` : 'Stopped'}
      </span>
    </div>
  );
}
