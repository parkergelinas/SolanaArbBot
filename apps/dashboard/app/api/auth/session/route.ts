import { type NextRequest, NextResponse } from 'next/server';

import { verifyJwt } from '@/lib/auth';

export async function GET(req: NextRequest) {
  const token = req.cookies.get('arb_session')?.value;
  if (!token) {
    return NextResponse.json({ error: 'Not authenticated' }, { status: 401 });
  }
  const walletAddress = await verifyJwt(token);
  if (!walletAddress) {
    return NextResponse.json({ error: 'Invalid or expired session' }, { status: 401 });
  }
  return NextResponse.json({ walletAddress });
}
