import { NextResponse } from 'next/server';

// Auth disabled — always return authenticated with the owner wallet.
export async function GET() {
  return NextResponse.json({ walletAddress: '127fFEvPQ4FtaLQGmNC9MNQBPA6UJ9HftndJNxJNDVJE' });
}
