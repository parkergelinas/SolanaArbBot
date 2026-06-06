import { type NextRequest, NextResponse } from 'next/server';
import nacl from 'tweetnacl';
import bs58 from 'bs58';

import { consumeNonce, isApprovedWallet, signJwt } from '@/lib/auth';

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null);
  const { walletAddress, signature, nonce } = body ?? {};

  if (!walletAddress || !signature || !nonce) {
    return NextResponse.json({ error: 'Missing fields' }, { status: 400 });
  }

  if (!isApprovedWallet(walletAddress)) {
    return NextResponse.json({ error: 'Wallet not authorized' }, { status: 403 });
  }

  if (!consumeNonce(walletAddress, nonce)) {
    return NextResponse.json({ error: 'Invalid or expired nonce' }, { status: 401 });
  }

  const message = `Sign in to Solana Arb\nWallet: ${walletAddress}\nNonce: ${nonce}`;
  const messageBytes = new TextEncoder().encode(message);

  let signatureBytes: Uint8Array;
  let publicKeyBytes: Uint8Array;
  try {
    signatureBytes = bs58.decode(signature);
    publicKeyBytes = bs58.decode(walletAddress);
  } catch {
    return NextResponse.json({ error: 'Invalid encoding' }, { status: 400 });
  }

  const valid = nacl.sign.detached.verify(messageBytes, signatureBytes, publicKeyBytes);
  if (!valid) {
    return NextResponse.json({ error: 'Invalid signature' }, { status: 401 });
  }

  const token = await signJwt(walletAddress);
  const res = NextResponse.json({ ok: true, walletAddress });
  res.cookies.set('arb_session', token, {
    httpOnly: true,
    sameSite: 'lax',
    path: '/',
    maxAge: 8 * 60 * 60,
    secure: process.env.NODE_ENV === 'production',
  });
  return res;
}
