import { Connection, Keypair, PublicKey, TransactionInstruction, VersionedTransaction } from '@solana/web3.js';
export interface JitoBundleResult {
    bundleId: string;
    success: boolean;
    error?: string;
}
export declare class JitoClient {
    private readonly blockEngineUrl;
    readonly defaultTipLamports: number;
    constructor(tipLamports?: number);
    randomTipAccount(): PublicKey;
    /**
     * Build a tip transfer instruction that must be appended to the swap transaction.
     * Using a single-transaction bundle is cheaper and simpler than a two-tx bundle.
     */
    buildTipInstruction(payer: PublicKey, lamports: number): TransactionInstruction;
    sendBundle(transactions: VersionedTransaction[]): Promise<JitoBundleResult>;
    isAvailable(): Promise<boolean>;
}
/**
 * Rebuild the swap VersionedTransaction with an extra Jito tip transfer instruction
 * and an updated compute-unit limit derived from simulation.
 *
 * Loads ALTs from chain, decompiles the Jupiter-built message, injects instructions,
 * recompiles to v0, and signs with the provided keypair.
 */
export declare function buildJitoTransaction(connection: Connection, originalTx: VersionedTransaction, payer: Keypair, jito: JitoClient, tipLamports: number, computeUnits: number): Promise<VersionedTransaction>;
