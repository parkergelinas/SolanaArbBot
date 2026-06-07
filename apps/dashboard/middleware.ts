import { type NextRequest, NextResponse } from 'next/server';

import { verifyJwt } from '@/lib/auth';

/**
 * Auth endpoints that must be reachable before a session exists.
 * Everything else under /api/ requires a valid arb_session cookie.
 */
const PUBLIC_API_PREFIXES = [
  '/api/auth/challenge',
  '/api/auth/verify',
  '/api/auth/logout',
  '/api/auth/session',
];

export async function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;

  // Always allow static assets handled by the matcher exclusions.

  // Pass public auth endpoints through unauthenticated.
  if (PUBLIC_API_PREFIXES.some((p) => pathname.startsWith(p))) {
    return NextResponse.next();
  }

  // All /api/* proxy routes require a valid session.
  if (pathname.startsWith('/api/')) {
    const token = req.cookies.get('arb_session')?.value ?? '';
    const wallet = token ? await verifyJwt(token) : null;
    if (!wallet) {
      return NextResponse.json(
        { error: 'Unauthorized — sign in with your approved Solana wallet' },
        { status: 401 },
      );
    }
    return NextResponse.next();
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    '/((?!_next/static|_next/image|favicon.ico|.*\\.(?:png|svg|ico|webp|jpg|jpeg)).*)',
  ],
};
