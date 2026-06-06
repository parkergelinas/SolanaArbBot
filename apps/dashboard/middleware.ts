import { type NextRequest, NextResponse } from 'next/server';

import { verifyJwt } from '@/lib/auth';

const PUBLIC_PREFIXES = ['/login', '/api/auth'];

export async function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;

  if (PUBLIC_PREFIXES.some((p) => pathname === p || pathname.startsWith(`${p}/`))) {
    return NextResponse.next();
  }

  const token = req.cookies.get('arb_session')?.value;
  if (!token) {
    return NextResponse.redirect(new URL('/login', req.url));
  }

  const wallet = await verifyJwt(token);
  if (!wallet) {
    const res = NextResponse.redirect(new URL('/login', req.url));
    res.cookies.delete('arb_session');
    return res;
  }

  return NextResponse.next();
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico|.*\\.(?:png|svg|ico|webp|jpg|jpeg)).*)'],
};
