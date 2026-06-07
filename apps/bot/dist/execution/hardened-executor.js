/**
 * Production-grade swap executor:
 * - Versioned (v0) transactions with address lookup tables
 * - Dynamic compute-unit estimation via simulation
 * - Simulate-before-send; aborts on simulation failure
 * - Dead-man's switch: halts after 3 consecutive failures
 * - Jito bundle submission with SOL-transfer tip; falls back to standard RPC
 * - Every attempt persisted to SQLite with full metadata
 */
import { ComputeBudgetProgram, Connection, Keypair, TransactionMessage, VersionedTransaction, } from '@solana/web3.js';
import bs58 from 'bs58';
import { insertTrade } from '../db/sqlite.js';
import { logger } from '../logger.js';
import { JitoClient, buildJitoTransaction } from '../mev/jito-client.js';
import { DeadManSwitch } from './dead-man-switch.js';
const LAMPORTS_PER_SOL = 1_000_000_000;
// Buffer added on top of simulated CU consumption.
const CU_BUFFER_FACTOR = 1.2;
// Recent block window for prioritization fee sampling.
const PRIORITY_FEE_PERCENTILE = 0.75;
export class HardenedExecutor {
    env;
    client;
    journal;
    deadManSwitch;
    connection;
    jito;
    keypair;
    jitoEnabled;
    jitoTipLamports;
    minProfitLamports;
    jitoAvailable = true;
    constructor(env, client, journal, opts = {}) {
        this.env = env;
        this.client = client;
        this.journal = journal;
        this.connection = new Connection(env.rpcUrl, 'confirmed');
        this.deadManSwitch = new DeadManSwitch({ maxConsecutiveFailures: 3 });
        this.jitoEnabled = opts.jitoEnabled ?? (process.env.JITO_ENABLED === '1');
        this.jitoTipLamports = opts.jitoTipLamports
            ?? Number(process.env.JITO_TIP_LAMPORTS ?? '10000');
        this.minProfitLamports = opts.minProfitLamports
            ?? Number(process.env.MIN_PROFIT_LAMPORTS ?? '0');
        this.jito = new JitoClient(this.jitoTipLamports);
        this.keypair = loadKeypair();
    }
    async execute(decision, opts = {}) {
        if (this.deadManSwitch.isHalted()) {
            return { success: false, error: 'dead_man_switch_halted' };
        }
        if (this.env.paperMode) {
            return this.paperResult(decision);
        }
        if (!this.keypair) {
            return { success: false, error: 'wallet_keypair_not_configured' };
        }
        const maxRetries = opts.maxRetries ?? 3;
        const baseBackoff = opts.baseBackoffMs ?? 200;
        for (let attempt = 0; attempt < maxRetries; attempt++) {
            const result = await this.attemptOnce(decision, attempt);
            if (result.success) {
                this.deadManSwitch.recordSuccess();
                this.logToJournal(decision, result, attempt);
                this.persistToDb(decision, result);
                return result;
            }
            const isRetryable = isRetryableError(result.error ?? '');
            if (!isRetryable || attempt + 1 >= maxRetries) {
                this.deadManSwitch.recordFailure(result.error ?? 'unknown');
                this.logToJournal(decision, result, attempt);
                this.persistToDb(decision, result);
                return result;
            }
            await sleep(baseBackoff * 2 ** attempt);
        }
        const exhausted = { success: false, error: 'retries_exhausted' };
        this.deadManSwitch.recordFailure('retries_exhausted');
        this.persistToDb(decision, exhausted);
        return exhausted;
    }
    async attemptOnce(decision, attempt) {
        const wallet = this.keypair;
        try {
            // 1. Fresh quote
            const quote = await withTimeout(this.client.getQuote({
                inputMint: decision.inputMint,
                outputMint: decision.outputMint,
                amount: decision.amountInAtomic,
                slippageBps: this.env.slippageBps,
            }), 8_000, 'jupiter_quote_timeout');
            // 2. Estimate dynamic priority fee from recent blocks (returns micro-lamports per CU)
            const priorityFeeMicroLamports = await this.estimatePriorityFee();
            // 3. Guard: abort if estimated gas + tip would exceed expected profit.
            //    Convert the per-CU rate to an estimated total fee using a typical Jupiter
            //    swap CU budget (200,000 CU).  This is a conservative pre-simulation estimate.
            const TYPICAL_SWAP_CUS = 200_000;
            const estimatedPriorityFeeL = Math.ceil(priorityFeeMicroLamports * TYPICAL_SWAP_CUS / 1_000_000);
            const totalFeeLamports = estimatedPriorityFeeL + (this.jitoEnabled ? this.jitoTipLamports : 0);
            if (this.minProfitLamports > 0 && totalFeeLamports > this.minProfitLamports) {
                return {
                    success: false,
                    error: `fees_exceed_profit_floor: fees=${totalFeeLamports} min=${this.minProfitLamports}`,
                };
            }
            // 4. Build swap transaction from Jupiter.
            //    Pass micro-lamports as `prioritizationFeeLamports` — Jupiter v6 treats this
            //    field as the per-CU priority fee (despite the confusing "Lamports" suffix).
            const swap = await withTimeout(this.client.getSwapTransaction({
                quoteResponse: quote,
                userPublicKey: wallet.publicKey.toBase58(),
                wrapAndUnwrapSol: true,
                dynamicComputeUnitLimit: false, // we set CU limit ourselves after simulation
                prioritizationFeeLamports: priorityFeeMicroLamports,
            }), 8_000, 'jupiter_swap_timeout');
            // 5. Deserialize — Jupiter returns a v0 VersionedTransaction
            const tx = VersionedTransaction.deserialize(Buffer.from(swap.swapTransaction, 'base64'));
            // 6. Simulate to get consumed compute units and catch logical failures
            const sim = await this.connection.simulateTransaction(tx, { sigVerify: false });
            if (sim.value.err) {
                return {
                    success: false,
                    error: `simulation_failed:${JSON.stringify(sim.value.err)}`,
                };
            }
            const simulatedCUs = sim.value.unitsConsumed ?? 200_000;
            // 7. Rebuild transaction with correct CU limit (and Jito tip if enabled)
            let finalTx;
            let usedJito = false;
            if (this.jitoEnabled && this.jitoAvailable) {
                try {
                    finalTx = await buildJitoTransaction(this.connection, tx, wallet, this.jito, this.jitoTipLamports, simulatedCUs);
                    usedJito = true;
                }
                catch (err) {
                    logger.warn({ err }, 'jito tx build failed — falling back to standard RPC');
                    finalTx = await this.rebuildWithCuLimit(tx, wallet, simulatedCUs, priorityFeeMicroLamports);
                }
            }
            else {
                finalTx = await this.rebuildWithCuLimit(tx, wallet, simulatedCUs, priorityFeeMicroLamports);
            }
            // 8. Submit
            let signature;
            if (usedJito) {
                const bundleResult = await this.jito.sendBundle([finalTx]);
                if (!bundleResult.success) {
                    // Jito submission failed — flip availability flag, fall back to standard RPC
                    this.jitoAvailable = false;
                    setTimeout(() => { this.jitoAvailable = true; }, 30_000);
                    logger.warn({ error: bundleResult.error }, 'jito bundle rejected; falling back');
                    finalTx = await this.rebuildWithCuLimit(tx, wallet, simulatedCUs, priorityFeeMicroLamports);
                    usedJito = false;
                }
                else {
                    signature = bundleResult.bundleId; // bundle ID used as signature for logging
                    return {
                        success: true,
                        signature,
                        realizedProfitUsd: decision.netProfitUsd,
                        routeMetadata: {
                            attempt,
                            hops: quote.routePlan?.length ?? 0,
                            jito: true,
                            jitoTipLamports: this.jitoTipLamports,
                            computeUnits: simulatedCUs,
                            priorityFeeMicroLamports,
                        },
                    };
                }
            }
            // Standard RPC send
            signature = await this.connection.sendRawTransaction(finalTx.serialize(), {
                skipPreflight: true, // already simulated above
                maxRetries: 0,
            });
            // 9. Confirm (use lastValidBlockHeight for expiry detection)
            const confirmation = await this.connection.confirmTransaction({
                signature,
                blockhash: finalTx.message.recentBlockhash,
                lastValidBlockHeight: swap.lastValidBlockHeight ?? (await this.connection.getBlockHeight()) + 150,
            }, 'confirmed');
            if (confirmation.value.err) {
                return {
                    success: false,
                    error: `confirmation_error:${JSON.stringify(confirmation.value.err)}`,
                };
            }
            return {
                success: true,
                signature,
                realizedProfitUsd: decision.netProfitUsd,
                routeMetadata: {
                    attempt,
                    hops: quote.routePlan?.length ?? 0,
                    jito: false,
                    computeUnits: simulatedCUs,
                    priorityFeeMicroLamports,
                },
            };
        }
        catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            return { success: false, error: msg };
        }
    }
    /** Rebuild VersionedTransaction with correct CU limit and priority fee, sign it (no Jito tip). */
    async rebuildWithCuLimit(originalTx, payer, simulatedCUs, priorityFeeMicroLamports = 0) {
        const { addressTableLookups } = originalTx.message;
        const altAccounts = await Promise.all(addressTableLookups.map(async (lookup) => {
            const result = await this.connection.getAddressLookupTable(lookup.accountKey);
            if (!result.value)
                throw new Error(`ALT not found: ${lookup.accountKey.toBase58()}`);
            return result.value;
        }));
        const decomp = TransactionMessage.decompile(originalTx.message, {
            addressLookupTableAccounts: altAccounts,
        });
        const filteredIxs = decomp.instructions.filter((ix) => !ix.programId.equals(ComputeBudgetProgram.programId));
        const budgetIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: Math.ceil(simulatedCUs * CU_BUFFER_FACTOR),
            }),
        ];
        if (priorityFeeMicroLamports > 0) {
            budgetIxs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: priorityFeeMicroLamports }));
        }
        const message = new TransactionMessage({
            payerKey: payer.publicKey,
            recentBlockhash: decomp.recentBlockhash,
            instructions: [...budgetIxs, ...filteredIxs],
        }).compileToV0Message(altAccounts);
        const tx = new VersionedTransaction(message);
        tx.sign([payer]);
        return tx;
    }
    /** Returns p75 of recent slot prioritization fees in micro-lamports per compute unit. */
    async estimatePriorityFee() {
        try {
            const fees = await this.connection.getRecentPrioritizationFees();
            if (fees.length === 0)
                return 50_000;
            const sorted = fees.map((f) => f.prioritizationFee).sort((a, b) => a - b);
            const idx = Math.floor(sorted.length * PRIORITY_FEE_PERCENTILE);
            const microLamports = sorted[idx] ?? 50_000;
            return Math.max(5_000, Math.min(500_000, microLamports));
        }
        catch {
            return 50_000;
        }
    }
    paperResult(decision) {
        const r = {
            success: true,
            realizedProfitUsd: decision.netProfitUsd * 0.85,
            routeMetadata: { mode: 'paper' },
        };
        this.logToJournal(decision, r, 0);
        return r;
    }
    logToJournal(decision, result, attempt) {
        this.journal.append({
            type: result.success ? 'execution' : 'rejection',
            strategyId: decision.strategyId,
            pairLabel: decision.pairLabel,
            expectedProfitUsd: decision.netProfitUsd,
            realizedProfitUsd: result.realizedProfitUsd,
            rejectionReason: result.error,
            routeMetadata: { attempt, ...result.routeMetadata },
        });
    }
    persistToDb(decision, result) {
        const meta = result.routeMetadata ?? {};
        try {
            insertTrade({
                timestamp: Date.now(),
                strategy_id: decision.strategyId,
                pair_label: decision.pairLabel,
                input_mint: decision.inputMint,
                output_mint: decision.outputMint,
                amount_in_atomic: decision.amountInAtomic,
                expected_out_atomic: decision.expectedOutAtomic,
                actual_out_atomic: null,
                signature: result.signature ?? null,
                profit_usd: result.realizedProfitUsd ?? null,
                success: result.success ? 1 : 0,
                failure_reason: result.error ?? null,
                priority_fee_lamports: typeof meta.priorityFeeMicroLamports === 'number'
                    ? meta.priorityFeeMicroLamports : null,
                jito_tip_lamports: meta.jito ? this.jitoTipLamports : null,
                compute_units_used: typeof meta.computeUnits === 'number' ? meta.computeUnits : null,
                simulation_passed: result.success ? 1 : 0,
            });
        }
        catch (err) {
            logger.warn({ err }, 'failed to persist trade to SQLite');
        }
    }
}
// ─── helpers ──────────────────────────────────────────────────────────────────
function loadKeypair() {
    const raw = process.env.SOLANA_ARB_WALLET_KEY?.trim();
    if (!raw)
        return null;
    try {
        // Accept JSON array format: [12, 34, ...] (Solana CLI keypair file)
        if (raw.startsWith('[')) {
            const bytes = Uint8Array.from(JSON.parse(raw));
            return Keypair.fromSecretKey(bytes);
        }
        // Fall back to base58-encoded 64-byte key
        return Keypair.fromSecretKey(bs58.decode(raw));
    }
    catch (err) {
        logger.error({ err }, 'failed to load wallet keypair from SOLANA_ARB_WALLET_KEY');
        return null;
    }
}
function isRetryableError(msg) {
    return (msg.includes('blockhash') ||
        msg.includes('expired') ||
        msg.includes('timeout') ||
        msg.includes('429') ||
        msg.includes('503'));
}
function withTimeout(promise, ms, label) {
    return Promise.race([
        promise,
        new Promise((_, reject) => setTimeout(() => reject(new Error(label)), ms)),
    ]);
}
function sleep(ms) {
    return new Promise((r) => setTimeout(r, ms));
}
