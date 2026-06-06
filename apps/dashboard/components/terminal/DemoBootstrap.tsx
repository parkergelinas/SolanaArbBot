'use client';

import { useEffect } from 'react';

import { isTerminalDemoEnabled } from '@/lib/config/env';
import { startDemoEngine } from '@/lib/terminal/demoEngine';
import { useMarketStore } from '@/stores/marketStore';
import { usePriceAnchorStore } from '@/stores/priceAnchorStore';
import { useStreamStore } from '@/stores/streamStore';

/**
 * Optional demo simulator — only when NEXT_PUBLIC_TERMINAL_DEMO=1.
 * Production / devnet testing should leave this disabled and run stream-api.
 */
export default function DemoBootstrap() {
  const connected = useStreamStore((s) => s.connected);
  const priceReady = usePriceAnchorStore((s) => s.ready);
  const applyMessages = useMarketStore((s) => s.applyMessages);

  useEffect(() => {
    if (!isTerminalDemoEnabled() || !priceReady) return;

    let engine: ReturnType<typeof startDemoEngine> | null = null;
    let grace: ReturnType<typeof setTimeout> | null = null;

    const start = () => {
      if (engine) return;
      engine = startDemoEngine(applyMessages, { intervalMs: 100, seed: false });
    };

    const stop = () => {
      engine?.stop();
      engine = null;
    };

    if (!connected) {
      grace = setTimeout(start, 200);
    } else {
      stop();
    }

    return () => {
      if (grace) clearTimeout(grace);
      stop();
    };
  }, [connected, priceReady, applyMessages]);

  return null;
}
