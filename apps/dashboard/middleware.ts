import { type NextRequest, NextResponse } from 'next/server';

// Auth disabled — all routes pass through without session check.
export async function middleware(_req: NextRequest) {
  return NextResponse.next();
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico|.*\\.(?:png|svg|ico|webp|jpg|jpeg)).*)'],
};
