'use client';

import { useWsContext } from './WebSocketProvider';
import { useLatestWsEvent } from '@/lib/hooks';
import type { SystemStatus } from '@/lib/types';

export default function SystemStatusBadge() {
  const { connected } = useWsContext();
  const status = useLatestWsEvent<SystemStatus>('status');

  const running = status?.running ?? false;
  const mode    = status?.mode    ?? 'paper';

  return (
    <div className="flex items-center gap-3">
      {/* WS connection */}
      <div className="flex items-center gap-1.5">
        <span className={`w-2 h-2 rounded-full ${connected ? 'bg-green-400 live-pulse' : 'bg-red-500'}`} />
        <span className="text-xs text-slate-400">{connected ? 'WS live' : 'WS offline'}</span>
      </div>

      {/* Engine state */}
      <div className="flex items-center gap-1.5">
        <span className={`w-2 h-2 rounded-full ${running ? 'bg-green-400' : 'bg-slate-500'}`} />
        <span className="text-xs text-slate-400 capitalize">
          {running ? `Running (${mode})` : 'Stopped'}
        </span>
      </div>
    </div>
  );
}
