'use client';

import { useEffect } from 'react';

import { startDemoEngine } from '@/lib/terminal/demoEngine';
import { useMarketStore } from '@/stores/marketStore';
import { useStreamStore } from '@/stores/streamStore';

const DEMO_ENABLED =
  typeof process !== 'undefined' &&
  process.env.NEXT_PUBLIC_TERMINAL_DEMO !== '0';

const CLEAR_ON_LIVE =
  typeof process !== 'undefined' &&
  process.env.NEXT_PUBLIC_TERMINAL_CLEAR_ON_LIVE !== '0';

/**
 * Seeds and simulates market data when the live stream is offline.
 * Live stream takes priority — demo pauses while connected.
 */
export default function DemoBootstrap() {
  const connected = useStreamStore((s) => s.connected);
  const applyMessages = useMarketStore((s) => s.applyMessages);
  const clearMarket = useMarketStore((s) => s.clear);

  useEffect(() => {
    if (!DEMO_ENABLED) return;

    let engine: ReturnType<typeof startDemoEngine> | null = null;
    let grace: ReturnType<typeof setTimeout> | null = null;

    const start = () => {
      if (engine) return;
      engine = startDemoEngine(applyMessages, { intervalMs: 100, seed: true });
    };

    const stop = () => {
      engine?.stop();
      engine = null;
    };

    if (!connected) {
      grace = setTimeout(start, 400);
    } else {
      stop();
      if (CLEAR_ON_LIVE) clearMarket();
    }

    return () => {
      if (grace) clearTimeout(grace);
      stop();
    };
  }, [connected, applyMessages, clearMarket]);

  return null;
}
