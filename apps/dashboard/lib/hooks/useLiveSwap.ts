'use client';

/**
 * useLiveSwap — full on-chain swap state machine.
 *
 * State transitions:
 *   idle ──[fetchQuote]──► quoting ──► ready
 *                                        │
 *                          [execute] ────▼
 *                                     signing ──► sending ──► confirming ──► confirmed
 *                                        │            │              │
 *                                        └────────────┴──────────────▼
 *                                                                  error
 *
 * Usage:
 *   const swap = useLiveSwap();
 *   await swap.fetchQuote(inputMint, outputMint, amountAtomic);
 *   // when swap.status === 'ready', show confirm dialog
 *   await swap.execute();
 *   // when swap.status === 'confirmed', swap.signature is the tx sig
 */

import { useCallback, useReducer, useRef } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';

import {
  deserializeJupiterTx,
  getJupiterQuote,
  getJupiterSwapTx,
  type JupQuote,
} from '@/lib/solana/jupiter';
import { estimatePriorityFee } from '@/lib/solana/priorityFee';

// ── State ──────────────────────────────────────────────────────────────────────

export type SwapStatus =
  | 'idle'
  | 'quoting'
  | 'ready'       // quote fetched — waiting for user to press Confirm
  | 'signing'     // wallet extension popup open
  | 'sending'     // tx serialised and submitted to RPC
  | 'confirming'  // waiting for >= 'confirmed' commitment
  | 'confirmed'   // done ✓
  | 'error';

export interface LiveSwapState {
  status: SwapStatus;
  quote: JupQuote | null;
  signature: string | null;
  error: string | null;
  priorityFeeLamports: number;
}

// ── Reducer ────────────────────────────────────────────────────────────────────

type Action =
  | { type: 'QUOTING' }
  | { type: 'READY'; quote: JupQuote; priorityFeeLamports: number }
  | { type: 'SIGNING' }
  | { type: 'SENDING' }
  | { type: 'CONFIRMING' }
  | { type: 'CONFIRMED'; signature: string }
  | { type: 'ERROR'; error: string }
  | { type: 'RESET' };

const INITIAL: LiveSwapState = {
  status: 'idle',
  quote: null,
  signature: null,
  error: null,
  priorityFeeLamports: 50_000,
};

function reducer(state: LiveSwapState, action: Action): LiveSwapState {
  switch (action.type) {
    case 'QUOTING':
      return { ...INITIAL, status: 'quoting' };
    case 'READY':
      return { ...state, status: 'ready', quote: action.quote, priorityFeeLamports: action.priorityFeeLamports, error: null };
    case 'SIGNING':
      return { ...state, status: 'signing', error: null };
    case 'SENDING':
      return { ...state, status: 'sending' };
    case 'CONFIRMING':
      return { ...state, status: 'confirming' };
    case 'CONFIRMED':
      return { ...state, status: 'confirmed', signature: action.signature };
    case 'ERROR':
      return { ...state, status: 'error', error: action.error };
    case 'RESET':
      return { ...INITIAL };
    default:
      return state;
  }
}

// ── Hook ───────────────────────────────────────────────────────────────────────

export interface UseLiveSwapReturn extends LiveSwapState {
  /**
   * Step 1 — fetch a Jupiter quote + priority fee estimate in parallel.
   * After this completes with status='ready', call execute() to proceed.
   */
  fetchQuote: (
    inputMint: string,
    outputMint: string,
    amountAtomic: number,
    slippageBps?: number,
    swapMode?: 'ExactIn' | 'ExactOut',
  ) => Promise<void>;

  /**
   * Step 2 — get the swap transaction, sign with wallet, send and confirm.
   * Only valid to call when status === 'ready'.
   */
  execute: () => Promise<void>;

  /** Reset state machine back to idle (also cancels any pending quote fetch). */
  reset: () => void;
}

export function useLiveSwap(): UseLiveSwapReturn {
  const { connection } = useConnection();
  const { publicKey, signTransaction, connected } = useWallet();
  const [state, dispatch] = useReducer(reducer, INITIAL);

  // Used to cancel a stale fetchQuote if the user changes their mind.
  const fetchAbort = useRef<AbortController | null>(null);

  // ── fetchQuote ─────────────────────────────────────────────────────────────

  const fetchQuote = useCallback(
    async (
      inputMint: string,
      outputMint: string,
      amountAtomic: number,
      slippageBps = 50,
      swapMode: 'ExactIn' | 'ExactOut' = 'ExactIn',
    ) => {
      if (!connected || !publicKey) {
        dispatch({ type: 'ERROR', error: 'Wallet not connected' });
        return;
      }

      // Cancel any in-flight quote fetch.
      fetchAbort.current?.abort();
      const ctrl = new AbortController();
      fetchAbort.current = ctrl;

      dispatch({ type: 'QUOTING' });

      try {
        const [quote, priorityFeeMicroLamports] = await Promise.all([
          getJupiterQuote({ inputMint, outputMint, amount: amountAtomic, slippageBps, swapMode }),
          estimatePriorityFee(connection),
        ]);

        // Ignore stale result if the user already cancelled.
        if (ctrl.signal.aborted) return;

        // Convert μL/CU rate to estimated total lamports using a typical Jupiter
        // swap CU budget (200,000 CU).  This is what Jupiter receives and what we
        // display in the confirm dialog.
        const priorityFeeLamports = Math.ceil(priorityFeeMicroLamports * 200_000 / 1_000_000);
        dispatch({ type: 'READY', quote, priorityFeeLamports });
      } catch (err) {
        if (ctrl.signal.aborted) return;
        dispatch({
          type: 'ERROR',
          error: err instanceof Error ? err.message : 'Quote failed — try again',
        });
      }
    },
    [connected, publicKey, connection],
  );

  // ── execute ────────────────────────────────────────────────────────────────

  const execute = useCallback(async () => {
    const { quote, priorityFeeLamports } = state;

    if (!quote) {
      dispatch({ type: 'ERROR', error: 'No quote — call fetchQuote first' });
      return;
    }
    if (!publicKey || !signTransaction) {
      dispatch({ type: 'ERROR', error: 'Wallet not connected or cannot sign' });
      return;
    }

    try {
      // 1. Fetch the swap transaction from Jupiter.
      dispatch({ type: 'SIGNING' });
      const swapResp = await getJupiterSwapTx(quote, publicKey.toBase58(), priorityFeeLamports);
      const tx = deserializeJupiterTx(swapResp.swapTransaction);

      // 2. Ask the wallet to sign (opens Phantom/Trust extension popup).
      const signed = await signTransaction(tx);

      // 3. Send to RPC.
      dispatch({ type: 'SENDING' });
      const rawTx = signed.serialize();
      const signature = await connection.sendRawTransaction(rawTx, {
        skipPreflight: false,
        maxRetries: 2,
        preflightCommitment: 'processed',
      });

      // 4. Confirm.
      dispatch({ type: 'CONFIRMING' });
      const blockhash = tx.message.recentBlockhash;
      const lastValidBlockHeight =
        swapResp.lastValidBlockHeight ?? (await connection.getBlockHeight()) + 150;

      const result = await connection.confirmTransaction(
        { signature, blockhash, lastValidBlockHeight },
        'confirmed',
      );

      if (result.value.err) {
        dispatch({
          type: 'ERROR',
          error: `Transaction failed on-chain: ${JSON.stringify(result.value.err)}`,
        });
        return;
      }

      dispatch({ type: 'CONFIRMED', signature });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      // User rejected the wallet prompt — return to ready state so they can try again.
      if (/reject|cancel|user denied/i.test(msg)) {
        dispatch({ type: 'READY', quote: state.quote!, priorityFeeLamports: state.priorityFeeLamports });
      } else {
        dispatch({ type: 'ERROR', error: msg });
      }
    }
  }, [state, publicKey, signTransaction, connection]);

  // ── reset ──────────────────────────────────────────────────────────────────

  const reset = useCallback(() => {
    fetchAbort.current?.abort();
    dispatch({ type: 'RESET' });
  }, []);

  return { ...state, fetchQuote, execute, reset };
}
