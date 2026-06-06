'use client';

import { createContext, useContext, type ReactNode } from 'react';

import { useDexScreenerPair } from '@/lib/hooks/useDexScreenerPair';
import type { DexPairSnapshot } from '@/lib/dexscreener/types';
import { useUiStore } from '@/stores/uiStore';

export interface DexScreenerContextValue {
  mint: string | null;
  snapshot: DexPairSnapshot | null;
  loading: boolean;
  error: string | null;
}

const DexScreenerContext = createContext<DexScreenerContextValue | null>(null);

/** Single pair lookup shared by chart + stats bar (one API call). */
export function DexScreenerProvider({ children }: { children: ReactNode }) {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const { snapshot, loading, error } = useDexScreenerPair(selectedMint);

  return (
    <DexScreenerContext.Provider
      value={{ mint: selectedMint, snapshot, loading, error }}
    >
      {children}
    </DexScreenerContext.Provider>
  );
}

export function useDexScreenerContext(): DexScreenerContextValue {
  const ctx = useContext(DexScreenerContext);
  if (!ctx) {
    throw new Error('useDexScreenerContext requires DexScreenerProvider');
  }
  return ctx;
}
