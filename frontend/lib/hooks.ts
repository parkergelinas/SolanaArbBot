'use client';

import { useSyncExternalStore } from 'react';
import { intelligenceStore } from './stream-store';
import type { SwapEvent, WalletSnapshot, WhaleAlert, SmartMoneyAlert } from './types';

export function useConnected() {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.connectedSnapshot,
    () => false,
  );
}

export function useSwaps(): SwapEvent[] {
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

export function useWallets(): WalletSnapshot[] {
  return useSyncExternalStore(
    (on) => intelligenceStore.subscribe(on),
    () => intelligenceStore.walletsSnapshot,
    () => [],
  );
}
