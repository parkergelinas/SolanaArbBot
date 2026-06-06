import { clusterApiUrl, type Cluster } from '@solana/web3.js';

import { defaultSolanaNetwork } from '@/lib/config/env';

export type SolanaCluster = 'mainnet-beta' | 'devnet';

const STORAGE_KEY = 'solarb_cluster';

export function clusterLabel(cluster: SolanaCluster): string {
  return cluster === 'devnet' ? 'Devnet' : 'Mainnet';
}

export function clusterToExpectedNetwork(cluster: SolanaCluster): 'mainnet' | 'devnet' {
  return cluster === 'devnet' ? 'devnet' : 'mainnet';
}

export function defaultCluster(): SolanaCluster {
  return defaultSolanaNetwork();
}

export function loadPersistedCluster(): SolanaCluster {
  if (typeof window === 'undefined') return defaultCluster();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === 'devnet' || raw === 'mainnet-beta') return raw;
  } catch {
    /* ignore */
  }
  return defaultCluster();
}

export function persistCluster(cluster: SolanaCluster): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, cluster);
  } catch {
    /* ignore */
  }
}

export function rpcUrlForCluster(cluster: SolanaCluster): string {
  if (cluster === 'mainnet-beta' && process.env.NEXT_PUBLIC_SOLANA_RPC_MAINNET) {
    return process.env.NEXT_PUBLIC_SOLANA_RPC_MAINNET;
  }
  if (cluster === 'devnet' && process.env.NEXT_PUBLIC_SOLANA_RPC_DEVNET) {
    return process.env.NEXT_PUBLIC_SOLANA_RPC_DEVNET;
  }
  if (process.env.NEXT_PUBLIC_SOLANA_RPC) {
    return process.env.NEXT_PUBLIC_SOLANA_RPC;
  }
  return clusterApiUrl(cluster as Cluster);
}
