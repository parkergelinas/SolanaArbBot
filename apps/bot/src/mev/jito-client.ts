import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import { logger } from '../logger.js';

/** Verified Jito tip accounts — one is chosen randomly per bundle. */
const TIP_ACCOUNTS = [
  '96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5',
  'HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe',
  'Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY',
  'ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt13ib8T3s',
  'DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh',
  '3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT',
  'r21Gamwd4nh4LQMX28SQQKKDsX4UiiFJAgD5GkHmEXD',
  'ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt',
];

const BLOCK_ENGINE_URLS = [
  'https://mainnet.block-engine.jito.wtf',
  'https://amsterdam.mainnet.block-engine.jito.wtf',
  'https://frankfurt.mainnet.block-engine.jito.wtf',
  'https://ny.mainnet.block-engine.jito.wtf',
  'https://tokyo.mainnet.block-engine.jito.wtf',
];

export interface JitoBundleResult {
  bundleId: string;
  success: boolean;
  error?: string;
}

function pick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

export class JitoClient {
  private readonly blockEngineUrl: string;
  readonly defaultTipLamports: number;

  constructor(tipLamports = 10_000) {
    this.blockEngineUrl = pick(BLOCK_ENGINE_URLS);
    this.defaultTipLamports = tipLamports;
  }

  randomTipAccount(): PublicKey {
    return new PublicKey(pick(TIP_ACCOUNTS));
  }

  /**
   * Build a tip transfer instruction that must be appended to the swap transaction.
   * Using a single-transaction bundle is cheaper and simpler than a two-tx bundle.
   */
  buildTipInstruction(payer: PublicKey, lamports: number): TransactionInstruction {
    return SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: this.randomTipAccount(),
      lamports,
    });
  }

  async sendBundle(transactions: VersionedTransaction[]): Promise<JitoBundleResult> {
    const encoded = transactions.map((tx) =>
      Buffer.from(tx.serialize()).toString('base64'),
    );

    try {
      const resp = await fetch(`${this.blockEngineUrl}/api/v1/bundles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'sendBundle',
          params: [encoded],
        }),
        signal: AbortSignal.timeout(10_000),
      });

      if (!resp.ok) {
        const body = await resp.text().catch(() => resp.statusText);
        return { bundleId: '', success: false, error: `HTTP ${resp.status}: ${body.slice(0, 300)}` };
      }

      const json = (await resp.json()) as { result?: string; error?: { message: string } };
      if (json.error) {
        return { bundleId: '', success: false, error: json.error.message };
      }
      return { bundleId: json.result ?? '', success: true };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      logger.warn({ err: msg, url: this.blockEngineUrl }, 'jito sendBundle failed');
      return { bundleId: '', success: false, error: msg };
    }
  }

  async isAvailable(): Promise<boolean> {
    try {
      const resp = await fetch(`${this.blockEngineUrl}/api/v1/bundles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getTipAccounts', params: [] }),
        signal: AbortSignal.timeout(3_000),
      });
      return resp.ok;
    } catch {
      return false;
    }
  }
}

/**
 * Rebuild the swap VersionedTransaction with an extra Jito tip transfer instruction
 * and an updated compute-unit limit derived from simulation.
 *
 * Loads ALTs from chain, decompiles the Jupiter-built message, injects instructions,
 * recompiles to v0, and signs with the provided keypair.
 */
export async function buildJitoTransaction(
  connection: Connection,
  originalTx: VersionedTransaction,
  payer: Keypair,
  jito: JitoClient,
  tipLamports: number,
  computeUnits: number,
): Promise<VersionedTransaction> {
  const { addressTableLookups } = originalTx.message;

  const altAccounts = await Promise.all(
    addressTableLookups.map(async (lookup) => {
      const result = await connection.getAddressLookupTable(lookup.accountKey);
      if (!result.value) throw new Error(`ALT not found: ${lookup.accountKey.toBase58()}`);
      return result.value;
    }),
  );

  const decomp = TransactionMessage.decompile(originalTx.message, {
    addressLookupTableAccounts: altAccounts,
  });

  // Strip any existing ComputeBudget instructions so we can replace them.
  const filteredIxs = decomp.instructions.filter(
    (ix) => !ix.programId.equals(ComputeBudgetProgram.programId),
  );

  const instructions = [
    ComputeBudgetProgram.setComputeUnitLimit({ units: Math.ceil(computeUnits * 1.2) }),
    ...filteredIxs,
    jito.buildTipInstruction(payer.publicKey, tipLamports),
  ];

  const message = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: decomp.recentBlockhash,
    instructions,
  }).compileToV0Message(altAccounts);

  const tx = new VersionedTransaction(message);
  tx.sign([payer]);
  return tx;
}
