'use client';

import { useEffect } from 'react';

import { api } from '@/lib/api';
import { signalEventToStreamSignal } from '@/lib/terminal/hubSignals';
import type { WSMessage } from '@/lib/stream/types';
import { useFeedsStore } from '@/stores/feedsStore';
import { useMarketStore } from '@/stores/marketStore';

const POLL_MS = 2_000;

/**
 * Merges control-api `/api/live-signals` into the terminal signal panel
 * when the signal hub is available (complements stream WS signals).
 */
export default function LiveHubBootstrap() {
  const applyMessages = useMarketStore((s) => s.applyMessages);
  const setSignalHub = useFeedsStore((s) => s.setSignalHub);

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      try {
        const events = await api.liveSignals({ limit: 40 });
        if (cancelled) return;

        setSignalHub({
          status: 'online',
          lastOkAt: Date.now(),
          detail: `${events.length} signals`,
        });

        if (events.length === 0) return;

        const messages: WSMessage[] = events.map((e) => ({
          type: 'signal' as const,
          payload: signalEventToStreamSignal(e),
        }));

        applyMessages(messages, { seq: 0, ts_ms: Date.now() });
      } catch {
        setSignalHub({ status: 'offline', lastOkAt: Date.now() });
      }
    };

    poll();
    const id = setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [applyMessages, setSignalHub]);

  return null;
}
