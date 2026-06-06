import { type NextRequest, NextResponse } from 'next/server';

import { generateNonce } from '@/lib/auth';

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null);
  const walletAddress = body?.walletAddress;

  if (!walletAddress || typeof walletAddress !== 'string') {
    return NextResponse.json({ error: 'walletAddress required' }, { status: 400 });
  }

  const nonce = generateNonce(walletAddress);
  const message = `Sign in to Solana Arb\nWallet: ${walletAddress}\nNonce: ${nonce}`;
  return NextResponse.json({ nonce, message });
}
