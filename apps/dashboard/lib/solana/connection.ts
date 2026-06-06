import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  type Commitment,
} from '@solana/web3.js';

import { rpcUrlForCluster, type SolanaCluster } from './network';

const DEFAULT_COMMITMENT: Commitment = 'confirmed';

const connectionCache = new Map<SolanaCluster, Connection>();

export function getConnection(cluster: SolanaCluster, commitment = DEFAULT_COMMITMENT): Connection {
  const key = cluster;
  let conn = connectionCache.get(key);
  if (!conn) {
    conn = new Connection(rpcUrlForCluster(cluster), { commitment });
    connectionCache.set(key, conn);
  }
  return conn;
}

export function invalidateConnectionCache(): void {
  connectionCache.clear();
}

export async function fetchSolBalance(
  cluster: SolanaCluster,
  pubkey: PublicKey,
): Promise<number> {
  const conn = getConnection(cluster);
  const lamports = await conn.getBalance(pubkey, DEFAULT_COMMITMENT);
  return lamports / LAMPORTS_PER_SOL;
}

export interface TokenBalanceRow {
  mint: string;
  amount: number;
  decimals: number;
}

/** Parsed SPL token balances for a wallet (mainnet or devnet RPC). */
export async function fetchTokenBalances(
  cluster: SolanaCluster,
  owner: PublicKey,
): Promise<TokenBalanceRow[]> {
  const conn = getConnection(cluster);
  const resp = await conn.getParsedTokenAccountsByOwner(owner, {
    programId: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
  });

  const rows: TokenBalanceRow[] = [];
  for (const { account } of resp.value) {
    const parsed = account.data.parsed;
    if (parsed?.type !== 'account') continue;
    const info = parsed.info;
    const tokenAmount = info?.tokenAmount;
    if (!tokenAmount || !info?.mint) continue;
    const amount = Number(tokenAmount.uiAmount ?? 0);
    if (amount <= 0) continue;
    rows.push({
      mint: info.mint as string,
      amount,
      decimals: tokenAmount.decimals as number,
    });
  }
  return rows.sort((a, b) => b.amount - a.amount);
}
