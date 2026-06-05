'use client';

import { useSyncExternalStore } from 'react';

import { intelligenceStore } from './store';
import type { SmartMoneyAlert, SwapEvent, WalletSnapshot, WhaleAlert } from './types';

export function useIntelConnected() {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.connectedSnapshot,
    () => false,
  );
}

export function useIntelSwaps(): SwapEvent[] {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.swapsSnapshot,
    () => [],
  );
}

export function useWhales(): WhaleAlert[] {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.whalesSnapshot,
    () => [],
  );
}

export function useSmartMoney(): SmartMoneyAlert[] {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.smartSnapshot,
    () => [],
  );
}

export function useTrackedWallets(): WalletSnapshot[] {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.walletsSnapshot,
    () => [],
  );
}
