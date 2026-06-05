import { NextRequest } from 'next/server';

import { proxyToControlApi } from '@/lib/control-api-proxy';

export const dynamic = 'force-dynamic';

type RouteContext = { params: { path: string[] } };

async function handle(req: NextRequest, { params }: RouteContext) {
  return proxyToControlApi(req, params.path);
}

export const GET = handle;
export const POST = handle;
export const PATCH = handle;
