import { SCHEMA_VERSION, type SwapEvent, type WSBatchFrame, type WSMessage } from '@/lib/stream/types';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

export function makeSwap(overrides: Partial<SwapEvent> = {}): SwapEvent {
  return {
    v: SCHEMA_VERSION,
    signature: `sig_${Math.random().toString(16).slice(2)}`,
    dex: 'raydium',
    token_in: SOL,
    token_out: USDC,
    amount_in: '1000000000',
    amount_out: '145000000',
    wallet: 'wallet_test',
    slot: 280_000_000,
    timestamp_ms: Date.now(),
    ...overrides,
  };
}

export function makeBatch(
  messages: WSMessage[],
  seq = 1,
  ts_ms = Date.now(),
): WSBatchFrame {
  return {
    v: SCHEMA_VERSION,
    type: 'batch',
    seq,
    ts_ms,
    messages,
  };
}

export function batchJson(frame: WSBatchFrame): string {
  return JSON.stringify(frame);
}

export function swapMessage(swap: SwapEvent): WSMessage {
  return { type: 'swap', payload: swap };
}
