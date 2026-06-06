import { create } from 'zustand';

export type FeedStatus = 'online' | 'offline' | 'degraded';

export interface FeedHealth {
  status: FeedStatus;
  lastOkAt: number;
  detail?: string;
}

interface FeedsState {
  stream: FeedHealth;
  signalHub: FeedHealth;
  dexScreener: FeedHealth;
  intelligence: FeedHealth;

  setStream: (patch: Partial<FeedHealth>) => void;
  setSignalHub: (patch: Partial<FeedHealth>) => void;
  setDexScreener: (patch: Partial<FeedHealth>) => void;
  setIntelligence: (patch: Partial<FeedHealth>) => void;
}

const offline = (): FeedHealth => ({ status: 'offline', lastOkAt: 0 });

export const useFeedsStore = create<FeedsState>((set) => ({
  stream: offline(),
  signalHub: offline(),
  dexScreener: offline(),
  intelligence: offline(),

  setStream: (patch) =>
    set((s) => ({ stream: { ...s.stream, ...patch } })),
  setSignalHub: (patch) =>
    set((s) => ({ signalHub: { ...s.signalHub, ...patch } })),
  setDexScreener: (patch) =>
    set((s) => ({ dexScreener: { ...s.dexScreener, ...patch } })),
  setIntelligence: (patch) =>
    set((s) => ({ intelligence: { ...s.intelligence, ...patch } })),
}));
