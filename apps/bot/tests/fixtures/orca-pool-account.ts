/**
 * Synthetic Orca Whirlpool account data for testing.
 *
 * Encodes a SOL/USDC pool with sqrtPriceX64 that resolves to ~170 USDC per SOL.
 * Layout mirrors the real on-chain Whirlpool account (see whirlpool-monitor.ts).
 */

import { PublicKey } from '@solana/web3.js';

const SOL_MINT  = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

/** Build sqrtPriceX64 (u128 LE) for a given UI price (tokenB per tokenA). */
function encodeSqrtPriceX64(uiPriceB_perA: number, decimalsA: number, decimalsB: number): Buffer {
  // raw price = uiPrice / 10^(decimalsA - decimalsB)
  const rawPrice = uiPriceB_perA / Math.pow(10, decimalsA - decimalsB);
  const sqrtRaw = Math.sqrt(rawPrice);
  const Q64 = 2n ** 64n;
  const sqrtX64 = BigInt(Math.floor(sqrtRaw * Number(Q64)));

  const buf = Buffer.alloc(16);
  let v = sqrtX64;
  for (let i = 0; i < 16; i++) {
    buf[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return buf;
}

function pubkeyBytes(mint: string): Buffer {
  return Buffer.from(new PublicKey(mint).toBytes());
}

/**
 * 300-byte synthetic Whirlpool account buffer for SOL/USDC @ 170 USD.
 * Offsets used:
 *   65..81  sqrtPrice (u128 LE)
 *   101..133 tokenMintA (Pubkey)
 *   181..213 tokenMintB (Pubkey)
 */
export function buildOrcaPoolAccount(uiPriceSolInUsdc = 170): Buffer {
  const buf = Buffer.alloc(300, 0);

  // Discriminator (8 bytes) — arbitrary for tests
  Buffer.from([0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81]).copy(buf, 0);

  // sqrtPrice at offset 65 (16 bytes, u128 LE) — SOL=9 dec, USDC=6 dec
  const sqrtBuf = encodeSqrtPriceX64(uiPriceSolInUsdc, 9, 6);
  sqrtBuf.copy(buf, 65);

  // tokenMintA at offset 101 (SOL)
  pubkeyBytes(SOL_MINT).copy(buf, 101);

  // tokenMintB at offset 181 (USDC)
  pubkeyBytes(USDC_MINT).copy(buf, 181);

  return buf;
}

export { SOL_MINT, USDC_MINT };
