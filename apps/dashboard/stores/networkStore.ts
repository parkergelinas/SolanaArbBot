import { create } from 'zustand';

import { invalidateConnectionCache } from '@/lib/solana/connection';
import {
  clusterLabel,
  clusterToExpectedNetwork,
  defaultCluster,
  loadPersistedCluster,
  persistCluster,
  rpcUrlForCluster,
  type SolanaCluster,
} from '@/lib/solana/network';

interface NetworkState {
  cluster: SolanaCluster;
  rpcUrl: string;
  switching: boolean;

  setCluster: (cluster: SolanaCluster) => void;
  hydrate: () => void;
}

export const useNetworkStore = create<NetworkState>((set, get) => ({
  cluster: defaultCluster(),
  rpcUrl: rpcUrlForCluster(defaultCluster()),
  switching: false,

  hydrate: () => {
    const cluster = loadPersistedCluster();
    set({ cluster, rpcUrl: rpcUrlForCluster(cluster) });
  },

  setCluster: (cluster) => {
    if (get().cluster === cluster) return;
    set({ switching: true });
    persistCluster(cluster);
    invalidateConnectionCache();
    set({
      cluster,
      rpcUrl: rpcUrlForCluster(cluster),
      switching: false,
    });
  },
}));

export { clusterLabel, clusterToExpectedNetwork };
export type { SolanaCluster };
